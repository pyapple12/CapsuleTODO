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

/// 存储层错误：SQLite 透传 / IO（目录自建失败）/ 指定条目不存在
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

    /// 建表（幂等：CREATE TABLE IF NOT EXISTS）+ 迁移（PL010：todos 三列幂等补齐；
    /// bubbles 同理暂不扩——排序按 id 倒序即创建序，PL013 再加 sort_order）
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

    /// PL010 数据迁移：todos 补 created_at/done_at/note 三列（幂等——先探列再 ALTER）。
    /// 存量回填语义：created_at/done_at = NULL（不模拟历史时间，龄期裁决对 NULL 恒无提醒）；
    /// note = ''（自由文本缺省）。新库建表后同样走本函数补齐（建表语句保持 V0.1.1 原样，
    /// 让"旧 schema → 迁移"路径与新库路径汇合同一份代码）
    fn migrate(&self) -> Result<(), StorageError> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(todos)")?;
        let existing: std::collections::HashSet<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .collect();
        drop(stmt);
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
        Ok(())
    }

    /// 新增待办（文本须先经 todo::validate_text 业务校验），返回含回填 id 的条目；
    /// created_at = 时间源 now（新建行非 NULL）
    pub fn add(&self, text: &str) -> Result<TodoItem, StorageError> {
        let now = (self.now)();
        self.conn.execute(
            "INSERT INTO todos(text, done, created_at, note) VALUES (?1, 0, ?2, '')",
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

    /// 清单排序视图：未完成在前按 id 升序、已完成在后按 id 升序
    pub fn list(&self) -> Result<Vec<TodoItem>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, done, created_at, done_at, note FROM todos ORDER BY done ASC, id ASC",
        )?;
        let rows = stmt.query_map([], Self::row_to_item)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 新增气泡（文本须先经 bubble::validate_bubble_text 校验），返回含回填 id 的条目
    pub fn add_bubble(&self, text: &str) -> Result<BubbleItem, StorageError> {
        self.conn
            .execute("INSERT INTO bubbles(text) VALUES (?1)", [text])?;
        Ok(BubbleItem {
            id: self.conn.last_insert_rowid(),
            text: text.to_string(),
        })
    }

    /// 气泡列表：新在前（id 倒序）
    pub fn list_bubbles(&self) -> Result<Vec<BubbleItem>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, text FROM bubbles ORDER BY id DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok(BubbleItem {
                id: row.get(0)?,
                text: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
    fn bubble_add_list_roundtrip_desc() {
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
        // 新在前（id 倒序）
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
}
