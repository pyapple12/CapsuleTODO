//! SQLite 清单存储：todos 表增删勾查（全参数化绑定，禁 SQL 拼接）。
//! 单一事实源 = db（PL002 定案）：操作即落库，list 排序 = 未完成在前按 id 升序。
//! PL010 扩容：todos 三列迁移（created_at/done_at/note，幂等 ALTER）+ 时间源注入
//! （NowFn 默认系统时钟，测试注入固定值零真实等待）+ rename/set_note。

use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension};
use thiserror::Error;

use crate::bubble::BubbleItem;
use crate::todo::TodoItem;

/// 存储层错误：SQLite 透传 / IO（目录自建失败）/ 指定条目不存在 / 重排 id 集合不合法
#[derive(Debug, Error)]
pub enum StorageError {
    /// SQLite 操作失败（打开/建表/读写）
    #[error("SQLite 错误：{0}")]
    Sqlite(#[from] rusqlite::Error),
    /// 数据目录创建或文件 IO 失败
    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),
    /// 指定 id 的条目不存在（更新/删除零行，零静默）
    #[error("待办条目不存在：{0}")]
    NotFound(i64),
    /// 重排 id 集合与现存集合不一致（长度不符 / 幽灵 id / 重复 id——防丢行，拒绝执行）
    #[error("重排 id 集合不合法：{0}")]
    ReorderMismatch(String),
}

/// 时间源（注入可测：默认系统时钟 epoch 毫秒；测试注入固定值）
pub type NowFn = Arc<dyn Fn() -> i64 + Send + Sync>;

/// 系统时钟默认实现
pub fn system_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// SQLite 清单存储（rusqlite Connection 非 Sync，跨线程共享须经 Mutex）
pub struct Storage {
    conn: Connection,
    now: NowFn,
}

impl Storage {
    /// 当前时刻（epoch 毫秒；时间源注入出口——命令层龄期裁决共用同一时钟）
    pub fn now_ms(&self) -> i64 {
        (self.now)()
    }

    /// 打开（或创建）库文件：父目录缺失自建，并确保表结构就位（系统时钟）
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        Self::open_with_now(path, Arc::new(system_now))
    }

    /// 打开并注入时间源（测试通道；生产走 open）
    pub fn open_with_now(path: &Path, now: NowFn) -> Result<Self, StorageError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        let storage = Self { conn, now };
        storage.init()?;
        Ok(storage)
    }

    /// 打开内存库（测试专用通道，禁触真实用户数据；系统时钟）
    pub fn open_in_memory() -> Result<Self, StorageError> {
        Self::open_in_memory_with_now(Arc::new(system_now))
    }

    /// 打开内存库并注入时间源（龄期/done_at 测试用）
    pub fn open_in_memory_with_now(now: NowFn) -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let storage = Self { conn, now };
        storage.init()?;
        Ok(storage)
    }

    /// 建表（幂等：CREATE TABLE IF NOT EXISTS）+ 迁移（PL010/PL013：todos 五列、
    /// bubbles 一列幂等补齐，见 migrate/migrate_sort_order）
    pub fn init(&self) -> Result<(), StorageError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS todos (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS bubbles (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS whiteboard (
                id      INTEGER PRIMARY KEY CHECK (id = 1),
                content TEXT NOT NULL DEFAULT ''
            );",
        )?;
        self.migrate()?;
        Ok(())
    }

    /// 探测表的列名集合（PRAGMA table_info 收敛：migrate/migrate_sort_order 三处
    /// 共用的幂等前置判定；table 仅内部字面量传入，无外部注入面）
    fn column_set(&self, table: &str) -> Result<std::collections::HashSet<String>, StorageError> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let cols: std::collections::HashSet<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .collect();
        Ok(cols)
    }

    /// PL010 数据迁移：todos 补 created_at/done_at/note 三列（幂等——先探列再 ALTER）。
    /// 存量回填语义：created_at/done_at = NULL（不模拟历史时间，龄期裁决对 NULL 恒无提醒）；
    /// note = ''（自由文本缺省）。新库建表后同样走本函数补齐（建表语句保持 V0.1.1 原样，
    /// 让"旧 schema → 迁移"路径与新库路径汇合同一份代码）
    fn migrate(&self) -> Result<(), StorageError> {
        let existing = self.column_set("todos")?;
        if !existing.contains("created_at") {
            self.conn
                .execute("ALTER TABLE todos ADD COLUMN created_at INTEGER", [])?;
        }
        if !existing.contains("done_at") {
            self.conn
                .execute("ALTER TABLE todos ADD COLUMN done_at INTEGER", [])?;
        }
        if !existing.contains("note") {
            self.conn.execute(
                "ALTER TABLE todos ADD COLUMN note TEXT NOT NULL DEFAULT ''",
                [],
            )?;
        }
        self.migrate_sort_order()?;
        Ok(())
    }

    /// PL013 数据迁移：todos/bubbles 各补 sort_order INTEGER 列（幂等），存量回填 =
    /// 现序号（todos 按 done 段内 id 升序 0..n；气泡按 id 倒序 0..n——升序输出即新
    /// 在前，保历史 unshift 展示序。读出重写幂等，列已在位时直接跳过不重算，避免
    /// 覆盖用户拖拽结果）
    fn migrate_sort_order(&self) -> Result<(), StorageError> {
        let todo_cols = self.column_set("todos")?;
        let bubble_cols = self.column_set("bubbles")?;
        let todo_missing = !todo_cols.contains("sort_order");
        let bubble_missing = !bubble_cols.contains("sort_order");
        if todo_missing {
            self.conn
                .execute("ALTER TABLE todos ADD COLUMN sort_order INTEGER", [])?;
        }
        if bubble_missing {
            self.conn
                .execute("ALTER TABLE bubbles ADD COLUMN sort_order INTEGER", [])?;
        }
        // 存量回填仅在补列的当次执行（幂等：列已在位 = 已有用户排序，禁覆盖）
        if todo_missing {
            self.conn.execute_batch(
                "UPDATE todos SET sort_order = (
                     SELECT COUNT(*) FROM todos t2
                     WHERE t2.done = todos.done AND t2.id < todos.id
                 );",
            )?;
        }
        if bubble_missing {
            self.conn.execute_batch(
                "UPDATE bubbles SET sort_order = (
                     SELECT COUNT(*) FROM bubbles b2 WHERE b2.id > bubbles.id
                 );",
            )?;
        }
        Ok(())
    }

    /// 新增待办（文本须先经 todo::validate_text 业务校验），返回含回填 id 的条目；
    /// created_at = 时间源 now（新建行非 NULL）；sort_order = 现存最大值 + 1（排尾追加）
    pub fn add(&self, text: &str) -> Result<TodoItem, StorageError> {
        let now = (self.now)();
        self.conn.execute(
            "INSERT INTO todos(text, done, created_at, note, sort_order)
             VALUES (?1, 0, ?2, '', (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM todos))",
            rusqlite::params![text, now],
        )?;
        Ok(TodoItem {
            id: self.conn.last_insert_rowid(),
            text: text.to_string(),
            done: false,
            created_at: Some(now),
            done_at: None,
            note: String::new(),
        })
    }

    /// 翻转完成态并返回更新后的条目；零行返回 NotFound（零静默）。
    /// done_at 随勾选语义落库：置 done = 时间源 now，退回 = NULL
    pub fn toggle(&self, id: i64) -> Result<TodoItem, StorageError> {
        let now = (self.now)();
        let changed = self.conn.execute(
            "UPDATE todos SET
                done = NOT done,
                done_at = CASE WHEN NOT done THEN ?2 ELSE NULL END
             WHERE id = ?1",
            rusqlite::params![id, now],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound(id));
        }
        self.get(id)
    }

    /// 改标题（文本须先经 validate_text）；零行返回 NotFound
    pub fn rename(&self, id: i64, text: &str) -> Result<TodoItem, StorageError> {
        let changed = self.conn.execute(
            "UPDATE todos SET text = ?2 WHERE id = ?1",
            rusqlite::params![id, text],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound(id));
        }
        self.get(id)
    }

    /// 写笔记（全文覆盖）；零行返回 NotFound
    pub fn set_note(&self, id: i64, note: &str) -> Result<(), StorageError> {
        let changed = self.conn.execute(
            "UPDATE todos SET note = ?2 WHERE id = ?1",
            rusqlite::params![id, note],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound(id));
        }
        Ok(())
    }

    /// 行映射（get/list 共用；NULL 时刻映射 Option::None）
    fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<TodoItem> {
        Ok(TodoItem {
            id: row.get(0)?,
            text: row.get(1)?,
            done: row.get::<_, i64>(2)? != 0,
            created_at: row.get(3)?,
            done_at: row.get(4)?,
            note: row.get(5)?,
        })
    }

    /// 读取单条；不存在返回 NotFound
    pub fn get(&self, id: i64) -> Result<TodoItem, StorageError> {
        self.conn
            .query_row(
                "SELECT id, text, done, created_at, done_at, note FROM todos WHERE id = ?1",
                rusqlite::params![id],
                Self::row_to_item,
            )
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound(id),
                other => StorageError::Sqlite(other),
            })
    }

    /// 删除条目；零行删除返回 NotFound
    pub fn remove(&self, id: i64) -> Result<(), StorageError> {
        let changed = self
            .conn
            .execute("DELETE FROM todos WHERE id = ?1", rusqlite::params![id])?;
        if changed == 0 {
            return Err(StorageError::NotFound(id));
        }
        Ok(())
    }

    /// 清单排序视图：未完成在前按 sort_order 升序（拖拽序）、已完成在后按 sort_order
    /// 升序（done 段 sort_order 沿勾选前快照，归档序另由 list_done 的 done_at 裁决；
    /// 次级键 id 兜底 NULL/同值——迁移回填已满射，此处为防御）
    pub fn list(&self) -> Result<Vec<TodoItem>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, done, created_at, done_at, note FROM todos
             ORDER BY done ASC, sort_order ASC, id ASC",
        )?;
        let rows = stmt.query_map([], Self::row_to_item)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 归档视图：已完成条目按 done_at 倒序（最新完成的在最上，V0.015 用户定案）；
    /// done_at 相同（同秒完成/迁移 NULL 行）按 id 倒序保持稳定序
    pub fn list_done(&self) -> Result<Vec<TodoItem>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, done, created_at, done_at, note FROM todos
             WHERE done = 1
             ORDER BY done_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], Self::row_to_item)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 新增气泡（文本须先经 bubble::validate_bubble_text 校验），返回含回填 id 的条目；
    /// sort_order = 现存最小值 − 1（**排头插入**——design captureBubble unshift 语义：
    /// 新捕获的气泡永远在最上，升序输出即新在前；拖拽重排后取 MIN−1 依然排头）
    pub fn add_bubble(&self, text: &str) -> Result<BubbleItem, StorageError> {
        self.conn.execute(
            "INSERT INTO bubbles(text, sort_order)
             VALUES (?1, (SELECT COALESCE(MIN(sort_order), 0) - 1 FROM bubbles))",
            [text],
        )?;
        Ok(BubbleItem {
            id: self.conn.last_insert_rowid(),
            text: text.to_string(),
        })
    }

    /// 气泡列表：按 sort_order 升序（拖拽序；新捕获排头插入 = 展示序与实验场
    /// unshift 语义一致，历史 id 倒序语义由迁移回填保持）
    pub fn list_bubbles(&self) -> Result<Vec<BubbleItem>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, text FROM bubbles ORDER BY sort_order ASC, id ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(BubbleItem {
                id: row.get(0)?,
                text: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 清单重排（拖拽落点提交）：ids = 未完成条目的目标顺序（全量集）。
    /// 事务内三重校验——集合与现存未完成集长度一致、无幽灵 id、无重复 id——
    /// 任一不符整体回滚拒绝执行（防丢行/幽灵行）；校验通过逐条 UPDATE sort_order
    pub fn reorder_todos(&self, ids: &[i64]) -> Result<(), StorageError> {
        let mut active: Vec<i64> = self
            .conn
            .prepare("SELECT id FROM todos WHERE done = 0")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        active.sort_unstable();
        let mut given: Vec<i64> = ids.to_vec();
        given.sort_unstable();
        given.dedup();
        if given.len() != ids.len() || given != active {
            return Err(StorageError::ReorderMismatch(format!(
                "ids 长度 {}（去重 {}）与未完成集 {} 不一致",
                ids.len(),
                given.len(),
                active.len()
            )));
        }
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), StorageError> {
            for (order, id) in ids.iter().enumerate() {
                self.conn.execute(
                    "UPDATE todos SET sort_order = ?1 WHERE id = ?2",
                    rusqlite::params![order as i64, id],
                )?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(())
            }
            Err(err) => {
                if let Err(rollback) = self.conn.execute_batch("ROLLBACK") {
                    eprintln!("清单重排事务回滚失败：{rollback}");
                }
                Err(err)
            }
        }
    }

    /// 气泡重排（拖拽落点提交）：ids = 全量气泡的目标顺序。校验与事务语义同 reorder_todos
    pub fn reorder_bubbles(&self, ids: &[i64]) -> Result<(), StorageError> {
        let mut current: Vec<i64> = self
            .conn
            .prepare("SELECT id FROM bubbles")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        current.sort_unstable();
        let mut given: Vec<i64> = ids.to_vec();
        given.sort_unstable();
        given.dedup();
        if given.len() != ids.len() || given != current {
            return Err(StorageError::ReorderMismatch(format!(
                "ids 长度 {}（去重 {}）与气泡集 {} 不一致",
                ids.len(),
                given.len(),
                current.len()
            )));
        }
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), StorageError> {
            for (order, id) in ids.iter().enumerate() {
                self.conn.execute(
                    "UPDATE bubbles SET sort_order = ?1 WHERE id = ?2",
                    rusqlite::params![order as i64, id],
                )?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(())
            }
            Err(err) => {
                if let Err(rollback) = self.conn.execute_batch("ROLLBACK") {
                    eprintln!("气泡重排事务回滚失败：{rollback}");
                }
                Err(err)
            }
        }
    }

    /// 读取单条气泡（复制回剪贴板的取数源）；不存在返回 NotFound
    pub fn get_bubble(&self, id: i64) -> Result<BubbleItem, StorageError> {
        self.conn
            .query_row(
                "SELECT id, text FROM bubbles WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok(BubbleItem {
                        id: row.get(0)?,
                        text: row.get(1)?,
                    })
                },
            )
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound(id),
                other => StorageError::Sqlite(other),
            })
    }

    /// 删除单条气泡；零行删除返回 NotFound
    pub fn remove_bubble(&self, id: i64) -> Result<(), StorageError> {
        let changed = self
            .conn
            .execute("DELETE FROM bubbles WHERE id = ?1", rusqlite::params![id])?;
        if changed == 0 {
            return Err(StorageError::NotFound(id));
        }
        Ok(())
    }

    /// 一键清空气泡，返回清除条数
    pub fn clear_bubbles(&self) -> Result<usize, StorageError> {
        let removed = self.conn.execute("DELETE FROM bubbles", [])?;
        Ok(removed)
    }

    /// 白板内容（单行表；无行返回空串——首启正常态，非错误）
    pub fn load_whiteboard(&self) -> Result<String, StorageError> {
        let content: Option<String> = self
            .conn
            .query_row("SELECT content FROM whiteboard WHERE id = 1", [], |row| {
                row.get(0)
            })
            .optional()?;
        Ok(content.unwrap_or_default())
    }

    /// 保存白板内容（单行表 UPSERT，幂等覆盖）
    pub fn save_whiteboard(&self, content: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO whiteboard(id, content) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET content = excluded.content",
            [content],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage() -> Storage {
        Storage::open_in_memory().expect("内存库必须可开")
    }

    #[test]
    fn init_is_idempotent() {
        let st = storage();
        st.init().expect("重复建表必须幂等");
    }

    #[test]
    fn add_and_list_roundtrip() {
        let st = storage();
        let a = st.add("任务一").expect("写入必须成功");
        let b = st.add("任务二").expect("写入必须成功");
        assert_eq!(a.id, 1);
        assert_eq!(b.id, 2);
        assert_eq!(a.text, "任务一");
        assert!(!a.done);
        assert_eq!(st.list().expect("读取必须成功").len(), 2);
    }

    #[test]
    fn toggle_roundtrip_and_get() {
        let st = storage();
        let item = st.add("任务一").expect("写入必须成功");
        let flipped = st.toggle(item.id).expect("更新必须成功");
        assert!(flipped.done);
        assert!(st.get(item.id).expect("读取必须成功").done);
        let back = st.toggle(item.id).expect("更新必须成功");
        assert!(!back.done);
    }

    #[test]
    fn remove_deletes_row() {
        let st = storage();
        let item = st.add("任务一").expect("写入必须成功");
        st.remove(item.id).expect("删除必须成功");
        assert!(st.list().expect("读取必须成功").is_empty());
    }

    #[test]
    fn missing_id_operations_are_errors() {
        let st = storage();
        assert!(matches!(st.toggle(9), Err(StorageError::NotFound(9))));
        assert!(matches!(st.get(9), Err(StorageError::NotFound(9))));
        assert!(matches!(st.remove(9), Err(StorageError::NotFound(9))));
    }

    #[test]
    fn list_orders_undone_first_by_id() {
        let st = storage();
        let a = st.add("任务一").expect("写入必须成功");
        let b = st.add("任务二").expect("写入必须成功");
        let c = st.add("任务三").expect("写入必须成功");
        st.toggle(a.id).expect("更新必须成功");
        st.toggle(c.id).expect("更新必须成功");
        let ids: Vec<i64> = st
            .list()
            .expect("读取必须成功")
            .into_iter()
            .map(|it| it.id)
            .collect();
        assert_eq!(ids, vec![b.id, a.id, c.id]);
    }

    #[test]
    fn open_creates_parent_dir_and_file() {
        let root = std::env::temp_dir().join(format!("capsule-todo-test-{}", std::process::id()));
        let db = root.join("data").join("todo.db");
        {
            let st = Storage::open(&db).expect("打开必须成功（父目录自建）");
            st.add("落盘").expect("写入必须成功");
            assert!(db.exists());
        }
        // 先 drop 关闭连接再清理（Windows 下持有句柄时目录删不掉）
        std::fs::remove_dir_all(&root).expect("清理必须成功");
    }

    #[test]
    fn bubble_add_list_roundtrip() {
        let st = storage();
        let a = st.add_bubble("片段一").expect("写入必须成功");
        let b = st.add_bubble("片段二").expect("写入必须成功");
        assert_eq!(a.id, 1);
        assert_eq!(b.id, 2);
        let ids: Vec<i64> = st
            .list_bubbles()
            .expect("读取必须成功")
            .into_iter()
            .map(|it| it.id)
            .collect();
        // list_bubbles = sort_order 升序（拖拽序）：新捕获排头插入（design unshift
        // 语义）→ 后加的在前
        assert_eq!(ids, vec![b.id, a.id]);
    }

    #[test]
    fn bubble_remove_missing_is_error() {
        let st = storage();
        assert!(matches!(
            st.remove_bubble(9),
            Err(StorageError::NotFound(9))
        ));
        assert!(matches!(st.get_bubble(9), Err(StorageError::NotFound(9))));
    }

    #[test]
    fn bubble_clear_returns_count_and_empties() {
        let st = storage();
        st.add_bubble("一").expect("写入必须成功");
        st.add_bubble("二").expect("写入必须成功");
        let removed = st.clear_bubbles().expect("清空必须成功");
        assert_eq!(removed, 2);
        assert_eq!(st.list_bubbles().expect("读取必须成功").len(), 0);
    }

    #[test]
    fn whiteboard_default_empty_and_roundtrip() {
        let st = storage();
        // 首启无行 = 空串（正常态，非错误）
        assert_eq!(st.load_whiteboard().expect("读取必须成功"), "");
        st.save_whiteboard("草稿一").expect("保存必须成功");
        assert_eq!(st.load_whiteboard().expect("读取必须成功"), "草稿一");
    }

    #[test]
    fn whiteboard_upsert_idempotent() {
        let st = storage();
        st.save_whiteboard("一").expect("保存必须成功");
        st.save_whiteboard("二").expect("保存必须成功");
        st.save_whiteboard("三").expect("保存必须成功");
        // 单行表 UPSERT：多次保存只覆盖一行，不累积
        assert_eq!(st.load_whiteboard().expect("读取必须成功"), "三");
    }

    // ===== PL010.1 迁移与时间源（TDD） =====

    /// 造旧 schema 库（V0.1.1 形态：todos 只有 id/text/done），插入存量行
    fn legacy_storage() -> Storage {
        let conn = Connection::open_in_memory().expect("内存库必须可开");
        conn.execute_batch(
            "CREATE TABLE todos (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO todos(text, done) VALUES ('旧条目一', 0);
            INSERT INTO todos(text, done) VALUES ('旧条目二', 1);",
        )
        .expect("旧 schema 必须可建");
        Storage {
            conn,
            now: Arc::new(system_now),
        }
    }

    #[test]
    fn migration_adds_columns_and_backfills() {
        let st = legacy_storage();
        st.init().expect("迁移必须成功");
        // 迁移后新字段可读写：存量行 created_at/done_at = NULL（不模拟时间）、note = ''
        let items = st.list().expect("读取必须成功");
        assert_eq!(items.len(), 2);
        for it in &items {
            assert_eq!(it.created_at, None, "存量行 created_at 必须为 NULL");
            assert_eq!(it.done_at, None, "存量行 done_at 必须为 NULL");
            assert_eq!(it.note, "", "存量行 note 必须回填空串");
        }
    }

    #[test]
    fn migration_is_idempotent() {
        let st = legacy_storage();
        st.init().expect("首次迁移必须成功");
        st.init().expect("二次迁移必须幂等（列已存在不重复 ALTER）");
        assert_eq!(st.list().expect("读取必须成功").len(), 2);
    }

    #[test]
    fn new_row_gets_created_at_and_empty_note() {
        // 注入固定时间源：add 落 created_at = now
        let fixed = 1_700_000_000_000i64;
        let st = Storage::open_in_memory_with_now(Arc::new(move || fixed)).expect("库必须可开");
        let item = st.add("新条目").expect("写入必须成功");
        assert_eq!(item.created_at, Some(fixed));
        assert_eq!(item.done_at, None);
        assert_eq!(item.note, "");
        let read = st.get(item.id).expect("读取必须成功");
        assert_eq!(read.created_at, Some(fixed));
    }

    #[test]
    fn toggle_sets_done_at_forward_and_nulls_backward() {
        use std::sync::atomic::{AtomicI64, Ordering};
        let step = Arc::new(AtomicI64::new(1_700_000_000_000));
        let step2 = Arc::clone(&step);
        let st = Storage::open_in_memory_with_now(Arc::new(move || step2.load(Ordering::SeqCst)))
            .expect("库必须可开");
        let item = st.add("任务").expect("写入必须成功");
        step.store(1_700_000_100_000, Ordering::SeqCst);
        let done = st.toggle(item.id).expect("勾选必须成功");
        assert_eq!(
            done.done_at,
            Some(1_700_000_100_000),
            "勾选置 done_at = now"
        );
        step.store(1_700_000_200_000, Ordering::SeqCst);
        let back = st.toggle(item.id).expect("退回必须成功");
        assert_eq!(back.done_at, None, "退回清 done_at");
    }

    // ===== PL011.1 归档视图（TDD） =====

    /// 注入可步进时钟的存储（归档排序断言：逐条勾选落不同 done_at）
    fn stepped_storage() -> (Storage, Arc<std::sync::atomic::AtomicI64>) {
        use std::sync::atomic::{AtomicI64, Ordering};
        let clock = Arc::new(AtomicI64::new(1_700_000_000_000));
        let clock2 = Arc::clone(&clock);
        let st = Storage::open_in_memory_with_now(Arc::new(move || clock2.load(Ordering::SeqCst)))
            .expect("库必须可开");
        (st, clock)
    }

    #[test]
    fn list_done_orders_by_done_at_desc() {
        use std::sync::atomic::Ordering;
        let (st, clock) = stepped_storage();
        // 依次勾选 a、b、c（done_at 递增）→ 归档视图应 c、b、a（最新完成在最上）
        let a = st.add("甲").expect("写入必须成功");
        let b = st.add("乙").expect("写入必须成功");
        let c = st.add("丙").expect("写入必须成功");
        for (id, ms) in [(a.id, 100i64), (b.id, 200), (c.id, 300)] {
            clock.store(1_700_000_000_000 + ms, Ordering::SeqCst);
            st.toggle(id).expect("勾选必须成功");
        }
        let ids: Vec<i64> = st
            .list_done()
            .expect("读取必须成功")
            .into_iter()
            .map(|it| it.id)
            .collect();
        assert_eq!(ids, vec![c.id, b.id, a.id], "done_at 倒序 = 最新完成在最上");
    }

    #[test]
    fn list_done_same_done_at_ties_break_by_id_desc() {
        let (st, clock) = stepped_storage();
        // 三条同一时刻勾选（done_at 相同）→ 按 id 倒序稳定
        let a = st.add("甲").expect("写入必须成功");
        let b = st.add("乙").expect("写入必须成功");
        let c = st.add("丙").expect("写入必须成功");
        st.toggle(a.id).expect("勾选必须成功");
        st.toggle(b.id).expect("勾选必须成功");
        st.toggle(c.id).expect("勾选必须成功");
        let _ = clock;
        let ids: Vec<i64> = st
            .list_done()
            .expect("读取必须成功")
            .into_iter()
            .map(|it| it.id)
            .collect();
        assert_eq!(ids, vec![c.id, b.id, a.id], "同 done_at 按 id 倒序");
    }

    #[test]
    fn list_done_excludes_undone_and_empty_ok() {
        let (st, _clock) = stepped_storage();
        // 空归档 = 空 vec（正常态）；未完成条目不进归档视图
        assert!(st.list_done().expect("读取必须成功").is_empty());
        let a = st.add("未完成").expect("写入必须成功");
        let _ = a;
        assert!(st.list_done().expect("读取必须成功").is_empty());
    }

    #[test]
    fn rename_updates_text_only() {
        let st = storage();
        let item = st.add("旧名").expect("写入必须成功");
        let renamed = st.rename(item.id, "新名").expect("改名必须成功");
        assert_eq!(renamed.text, "新名");
        assert_eq!(renamed.note, "", "改名不触碰笔记");
        assert!(matches!(
            st.rename(99, "x"),
            Err(StorageError::NotFound(99))
        ));
    }

    #[test]
    fn set_note_roundtrip() {
        let st = storage();
        let item = st.add("任务").expect("写入必须成功");
        st.set_note(item.id, "第一版笔记\n第二行")
            .expect("写笔记必须成功");
        let read = st.get(item.id).expect("读取必须成功");
        assert_eq!(read.note, "第一版笔记\n第二行");
        st.set_note(item.id, "").expect("清空笔记必须成功");
        assert_eq!(st.get(item.id).expect("读取必须成功").note, "");
        assert!(matches!(
            st.set_note(99, "x"),
            Err(StorageError::NotFound(99))
        ));
    }

    // ===== PL013.1 排序持久化（TDD） =====

    /// 造旧 schema 库（V0.1.1 原始形态：todos/bubbles 均无 sort_order），插入存量行
    fn legacy_storage_with_bubbles() -> Storage {
        let conn = Connection::open_in_memory().expect("内存库必须可开");
        conn.execute_batch(
            "CREATE TABLE todos (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE bubbles (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL
            );
            INSERT INTO todos(text, done) VALUES ('旧一', 0);
            INSERT INTO todos(text, done) VALUES ('旧二', 0);
            INSERT INTO todos(text, done) VALUES ('旧三', 0);
            INSERT INTO bubbles(text) VALUES ('旧泡一');
            INSERT INTO bubbles(text) VALUES ('旧泡二');",
        )
        .expect("旧 schema 必须可建");
        Storage {
            conn,
            now: Arc::new(system_now),
        }
    }

    #[test]
    fn migration_backfills_sort_order() {
        let st = legacy_storage_with_bubbles();
        st.init().expect("迁移必须成功");
        // 存量回填 = 现序号：未完成按 id 升序得 0/1/2（list 视图语义）
        let ids: Vec<i64> = st
            .list()
            .expect("读取必须成功")
            .into_iter()
            .map(|it| it.id)
            .collect();
        assert_eq!(ids.len(), 3, "三条存量未完成行都在");
        // 迁移后 list 顺序稳定（回填后 sort_order 与原 id 序一致）
        let first = st.list().expect("读取必须成功");
        assert!(
            first.windows(2).all(|w| w[0].id < w[1].id),
            "回填序 = 原 id 序"
        );
    }

    #[test]
    fn reorder_todos_persists_new_order() {
        let st = storage();
        let a = st.add("甲").expect("写入必须成功");
        let b = st.add("乙").expect("写入必须成功");
        let c = st.add("丙").expect("写入必须成功");
        // 拖拽语义：把 a 挪到 c 后面 → b、c、a
        st.reorder_todos(&[b.id, c.id, a.id]).expect("重排必须成功");
        let ids: Vec<i64> = st
            .list()
            .expect("读取必须成功")
            .into_iter()
            .map(|it| it.id)
            .collect();
        assert_eq!(ids, vec![b.id, c.id, a.id], "list 按新 sort_order 输出");
    }

    #[test]
    fn reorder_todos_ignores_done_rows_in_list() {
        let st = storage();
        let a = st.add("甲").expect("写入必须成功");
        let b = st.add("乙").expect("写入必须成功");
        let c = st.add("丙").expect("写入必须成功");
        st.toggle(c.id).expect("勾选必须成功");
        // 活动项重排不受 done 项影响：sort_order 落库，list 未完成段按新序
        st.reorder_todos(&[b.id, a.id]).expect("重排必须成功");
        let ids: Vec<i64> = st
            .list()
            .expect("读取必须成功")
            .into_iter()
            .map(|it| it.id)
            .collect();
        assert_eq!(ids, vec![b.id, a.id, c.id], "done 项沿原位（id 序兜底）");
    }

    #[test]
    fn reorder_todos_partial_id_set_is_error() {
        let st = storage();
        let a = st.add("甲").expect("写入必须成功");
        let _b = st.add("乙").expect("写入必须成功");
        // 长度不符（2 个活动项只给 1 个 id）：拒绝执行防丢行
        assert!(matches!(
            st.reorder_todos(&[a.id]),
            Err(StorageError::ReorderMismatch(_))
        ));
    }

    #[test]
    fn reorder_todos_ghost_id_is_error() {
        let st = storage();
        let a = st.add("甲").expect("写入必须成功");
        let b = st.add("乙").expect("写入必须成功");
        // 幽灵 id：集合内含不存在的 id（长度恰好凑对也拒）
        assert!(matches!(
            st.reorder_todos(&[a.id, b.id, 99]),
            Err(StorageError::ReorderMismatch(_))
        ));
    }

    #[test]
    fn reorder_todos_duplicate_id_is_error() {
        let st = storage();
        let a = st.add("甲").expect("写入必须成功");
        let b = st.add("乙").expect("写入必须成功");
        // 重复 id：集合非法（长度对但差集虽空——去重后长度不符同拒）
        assert!(matches!(
            st.reorder_todos(&[a.id, a.id, b.id]),
            Err(StorageError::ReorderMismatch(_))
        ));
    }

    #[test]
    fn reorder_todos_keeps_done_rows_sort_order() {
        let st = storage();
        let a = st.add("甲").expect("写入必须成功");
        let b = st.add("乙").expect("写入必须成功");
        st.toggle(b.id).expect("勾选必须成功");
        // done 行的 sort_order 不被 reorder 触碰：归档序仍由 done_at 裁决
        st.reorder_todos(&[a.id]).expect("单元素重排必须成功");
        let done = st.list_done().expect("读取必须成功");
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].id, b.id);
    }

    #[test]
    fn reorder_bubbles_persists_new_order() {
        let st = storage();
        let a = st.add_bubble("泡一").expect("写入必须成功");
        let b = st.add_bubble("泡二").expect("写入必须成功");
        let c = st.add_bubble("泡三").expect("写入必须成功");
        // 气泡全量集重排：list_bubbles 输出 = 传入序（前端展示序即存储序）
        st.reorder_bubbles(&[c.id, a.id, b.id])
            .expect("重排必须成功");
        let ids: Vec<i64> = st
            .list_bubbles()
            .expect("读取必须成功")
            .into_iter()
            .map(|it| it.id)
            .collect();
        assert_eq!(ids, vec![c.id, a.id, b.id], "气泡按新 sort_order 输出");
    }

    #[test]
    fn reorder_bubbles_partial_or_ghost_is_error() {
        let st = storage();
        let a = st.add_bubble("泡一").expect("写入必须成功");
        let _b = st.add_bubble("泡二").expect("写入必须成功");
        assert!(matches!(
            st.reorder_bubbles(&[a.id]),
            Err(StorageError::ReorderMismatch(_))
        ));
        assert!(matches!(
            st.reorder_bubbles(&[a.id, 99]),
            Err(StorageError::ReorderMismatch(_))
        ));
    }

    #[test]
    fn new_rows_get_increasing_sort_order() {
        let st = storage();
        let a = st.add("甲").expect("写入必须成功");
        let b = st.add("乙").expect("写入必须成功");
        st.reorder_todos(&[b.id, a.id]).expect("重排必须成功");
        let c = st.add("丙").expect("写入必须成功");
        // 新行 sort_order 排尾：追加不插队（list 末位）
        let ids: Vec<i64> = st
            .list()
            .expect("读取必须成功")
            .into_iter()
            .map(|it| it.id)
            .collect();
        assert_eq!(ids, vec![b.id, a.id, c.id], "新行落排尾");
    }
}
