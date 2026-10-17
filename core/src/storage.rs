//! SQLite 存储层：todos/bubbles/whiteboard 三表增删改查（全参数化绑定，禁 SQL 拼接），
//! 含气泡去重裁决（add_bubble 重复文本返回 Duplicate 不入库，PL015.5）。
//! 单一事实源 = db（PL002 定案）：操作即落库，list 排序 = 未完成在前按 sort_order
//! 升序（拖拽序），id 兜底。
//! PL010 扩容：todos 三列迁移（created_at/done_at/note，幂等 ALTER）+ 时间源注入
//! （NowFn 默认系统时钟，测试注入固定值零真实等待）+ rename/set_note。

use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension};
use thiserror::Error;

use crate::bubble::BubbleItem;
use crate::todo::{NoteBlock, NoteBlockKind, TodoItem};

/// 气泡新增结果（PL015.5 去重裁决）：重复文本拒入库不是错误——手动捕获据此切换
/// "重复捕获，无效！"占字态，热键路径据此静默；Added 携带回填 id 的新条目
#[derive(Debug, Clone)]
pub enum BubbleAddOutcome {
    /// 新增成功（含回填 id 的条目）
    Added(BubbleItem),
    /// 同文本气泡已存在，未入库
    Duplicate,
}

impl BubbleAddOutcome {
    /// 测试辅助：取新增条目（Duplicate 时 panic——调用方应先分支；生产路径一律 match）
    #[cfg(test)]
    pub fn added_item(self) -> BubbleItem {
        match self {
            BubbleAddOutcome::Added(item) => item,
            BubbleAddOutcome::Duplicate => panic!("气泡新增意外重复"),
        }
    }

    /// 是否重复拒入库（测试辅助，FIX005.30 沿 added_item 先例收敛——生产路径一律 match）
    #[cfg(test)]
    pub fn is_duplicate(&self) -> bool {
        matches!(self, BubbleAddOutcome::Duplicate)
    }
}

/// 存储层错误：SQLite 透传 / IO（目录自建失败）/ 指定条目不存在 / 重排 id 集合不合法
#[derive(Debug, Error)]
pub enum StorageError {
    /// SQLite 操作失败（打开/建表/读写）
    #[error("SQLite 错误：{0}")]
    Sqlite(#[from] rusqlite::Error),
    /// 数据目录创建或文件 IO 失败
    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),
    /// 指定 id 的条目不存在（更新/删除零行，零静默；待办/气泡共用故文案不带表语境）
    #[error("条目不存在：{0}")]
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

/// FNV-1a 64 位内容哈希（hex 16 位）：图片去重判据（PL025，零新依赖手写）
fn fnv1a_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// 同 stamp 已存在的图片文件夹最大序号（PL027 参数化：`prefix` 由调用域给——
/// 气泡传 `"bubble"`、详情传 `"note"`；无则 0）
pub(crate) fn max_seq(images_dir: &Path, prefix: &str, stamp: &str) -> u32 {
    let prefix = format!("{prefix}_{stamp}_");
    let mut max_n = 0u32;
    if let Ok(entries) = std::fs::read_dir(images_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(rest) = name.strip_prefix(&prefix) {
                    if let Ok(n) = rest.parse::<u32>() {
                        max_n = max_n.max(n);
                    }
                }
            }
        }
    }
    max_n
}

/// 下一个图片文件夹名 `{prefix}_{stamp}_{n}`（PL027 参数化）：n = 同 stamp 最大序号 + 1
pub(crate) fn next_seq_folder(images_dir: &Path, prefix: &str, stamp: &str) -> String {
    format!(
        "{prefix}_{stamp}_{}",
        max_seq(images_dir, prefix, stamp) + 1
    )
}

/// 净化文件名（PL025 文件图内层名）：取末段 + 替换 Windows 非法字符；空则回退 `image`
fn sanitize_file_name(name: &str) -> String {
    let base = std::path::Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image");
    let cleaned: String = base
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches(['.', ' ']);
    if trimmed.is_empty() {
        "image".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 从 image_file 相对路径取所属文件夹（PL025 删气泡删文件夹）
fn folder_of(rel: &str) -> String {
    rel.rsplit_once('/')
        .map(|(dir, _)| dir.to_string())
        .unwrap_or_else(|| rel.to_string())
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

    /// 打开内存库（测试专用通道，禁触真实用户数据；系统时钟）。
    /// FIX010.10：#[cfg(test)] 门控（A004 收敛先例）——文档声明改编译期强制，
    /// release 二进制不再携带；集成测试 storage_probe.rs 直连 rusqlite 不受影响
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, StorageError> {
        Self::open_in_memory_with_now(Arc::new(system_now))
    }

    /// 打开内存库并注入时间源（龄期/done_at 测试用）；门控同上（FIX010.10）
    #[cfg(test)]
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
            );
            CREATE TABLE IF NOT EXISTS note_blocks (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                todo_id    INTEGER NOT NULL,
                sort_order INTEGER NOT NULL,
                kind       TEXT NOT NULL DEFAULT 'text',
                text       TEXT NOT NULL DEFAULT '',
                image_hash TEXT,
                image_file TEXT
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

    /// PL010 数据迁移：todos 补 created_at/done_at 两列（幂等——先探列再 ALTER；
    /// **PL027 起 note 列不再新建**：其语义已由 note_blocks 块模型取代，见
    /// `migrate_note_to_blocks` + `drop_note_column`）。存量回填语义：created_at/done_at
    /// = NULL（不模拟历史时间，龄期裁决对 NULL 恒无提醒）。新库建表后同样走本函数补齐
    /// （建表语句保持 V0.1.1 原样，让"旧 schema → 迁移"路径与新库路径汇合同一份代码）
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
        self.migrate_sort_order()?;
        self.migrate_bubble_kind()?;
        self.migrate_bubble_image_file()?;
        self.migrate_note_to_blocks()?;
        self.drop_note_column()?;
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

    /// PL024 数据迁移：bubbles 补 kind 列（幂等——先探列再 ALTER，沿 PL013 先例）。
    /// 存量回填 kind = 'text'（DEFAULT 承担）；**不再新建 image 列**（PL025 载荷落盘化，
    /// 老库的 image 列由 migrate_blobs_to_files + drop_image_column 处理）
    fn migrate_bubble_kind(&self) -> Result<(), StorageError> {
        let bubble_cols = self.column_set("bubbles")?;
        if !bubble_cols.contains("kind") {
            self.conn.execute(
                "ALTER TABLE bubbles ADD COLUMN kind TEXT NOT NULL DEFAULT 'text'",
                [],
            )?;
        }
        Ok(())
    }

    /// PL025 数据迁移：bubbles 补 image_file/image_hash/image_source 三列（幂等，沿 PL013
    /// 先例）。image_source 默认 'screenshot'（存量 PL024 图为图片粘贴语义）
    fn migrate_bubble_image_file(&self) -> Result<(), StorageError> {
        let bubble_cols = self.column_set("bubbles")?;
        if !bubble_cols.contains("image_file") {
            self.conn
                .execute("ALTER TABLE bubbles ADD COLUMN image_file TEXT", [])?;
        }
        if !bubble_cols.contains("image_hash") {
            self.conn
                .execute("ALTER TABLE bubbles ADD COLUMN image_hash TEXT", [])?;
        }
        if !bubble_cols.contains("image_source") {
            self.conn.execute(
                "ALTER TABLE bubbles ADD COLUMN image_source TEXT NOT NULL DEFAULT 'screenshot'",
                [],
            )?;
        }
        Ok(())
    }

    /// PL025 存量 BLOB → 文件迁移（幂等）：kind='image' 且 image 非空且 image_file 为空的行
    /// → 写盘 `bubble_{stamp}_{n}/snap_{stamp}_{n}.png` + 回填 image_file（**含子目录**）/
    /// image_hash + image_source='screenshot' + image=NULL。须在 drop_image_column 之前调用；
    /// 新库无 image 列则直接跳过。`stamp` 由调用方给（一次迁移共用，n 递增）
    pub fn migrate_blobs_to_files(
        &self,
        images_dir: &Path,
        stamp: &str,
    ) -> Result<(), StorageError> {
        if !self.column_set("bubbles")?.contains("image") {
            return Ok(());
        }
        let rows: Vec<(i64, Vec<u8>)> = self
            .conn
            .prepare(
                "SELECT id, image FROM bubbles
                 WHERE kind = 'image' AND image IS NOT NULL
                   AND (image_file IS NULL OR image_file = '')",
            )?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        let base = max_seq(images_dir, "bubble", stamp) + 1;
        for (offset, (id, blob)) in rows.into_iter().enumerate() {
            let seq = base + offset as u32;
            let folder = format!("bubble_{stamp}_{seq}");
            let inner = format!("snap_{stamp}_{seq}.png");
            let dir = images_dir.join(&folder);
            std::fs::create_dir_all(&dir)?;
            std::fs::write(dir.join(&inner), &blob)?;
            let rel = format!("{folder}/{inner}");
            self.conn.execute(
                "UPDATE bubbles SET image_file = ?1, image_hash = ?2,
                     image_source = 'screenshot', image = NULL WHERE id = ?3",
                rusqlite::params![rel, fnv1a_hex(&blob), id],
            )?;
        }
        Ok(())
    }

    /// PL025 退役 image 列（迁移后调用；列存在才删，幂等）
    pub fn drop_image_column(&self) -> Result<(), StorageError> {
        if self.column_set("bubbles")?.contains("image") {
            self.conn
                .execute("ALTER TABLE bubbles DROP COLUMN image", [])?;
        }
        Ok(())
    }

    /// PL027 数据迁移：`todos.note`（单列纯文本）→ `note_blocks` 文本块（**一段一行一块**）。
    /// 存量 note 按 `\n` 拆成多个文本块（`\r` 一并剥除），空 note 建一个空文本块
    /// （不变量：至少一个文本块）。幂等：该 todo 已有块则跳过；无 note 列（新库/已迁移）
    /// 直接跳过。须在 `drop_note_column` 之前调用
    pub fn migrate_note_to_blocks(&self) -> Result<(), StorageError> {
        if !self.column_set("todos")?.contains("note") {
            return Ok(());
        }
        let rows: Vec<(i64, String)> = self
            .conn
            .prepare("SELECT id, note FROM todos")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        for (todo_id, note) in rows {
            let existing: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM note_blocks WHERE todo_id = ?1",
                [todo_id],
                |row| row.get(0),
            )?;
            if existing > 0 {
                continue;
            }
            let lines: Vec<String> = note
                .split('\n')
                .map(|line| line.trim_end_matches('\r').to_string())
                .collect();
            // PATCH002 归一化：前导空行丢弃、连续空行合并（只留一个）；结果为空则补一个空块
            let mut order = 0i64;
            let mut prev_blank = true;
            for line in &lines {
                let blank = line.trim().is_empty();
                if blank && prev_blank {
                    continue;
                }
                self.conn.execute(
                    "INSERT INTO note_blocks(todo_id, sort_order, kind, text)
                     VALUES (?1, ?2, 'text', ?3)",
                    rusqlite::params![todo_id, order, line],
                )?;
                prev_blank = blank;
                order += 1;
            }
            if order == 0 {
                self.conn.execute(
                    "INSERT INTO note_blocks(todo_id, sort_order, kind, text)
                     VALUES (?1, 0, 'text', '')",
                    [todo_id],
                )?;
            }
        }
        Ok(())
    }

    /// PL027 退役 note 列（迁移后调用；列存在才删，幂等）
    pub fn drop_note_column(&self) -> Result<(), StorageError> {
        if self.column_set("todos")?.contains("note") {
            self.conn
                .execute("ALTER TABLE todos DROP COLUMN note", [])?;
        }
        Ok(())
    }

    /// 新增待办（文本须先经 todo::validate_text 业务校验，且调用方须传 **trim 后**
    /// 文本——trim 执行点在命令层，直调本方法绕过命令层即失守此契约，FIX009.7
    /// 对齐 add_bubble 同款声明），返回含回填 id 的条目；
    /// created_at = 时间源 now（新建行非 NULL）；sort_order = 未完成段最小值 − 1
    /// （排头插入——2026-09-30 用户定案：新增置顶，与气泡排头插入语义对齐；
    /// 限定 done = 0 段取 MIN，不侵完成序）
    pub fn add(&self, text: &str) -> Result<TodoItem, StorageError> {
        let now = (self.now)();
        self.conn.execute(
            "INSERT INTO todos(text, done, created_at, sort_order)
             VALUES (?1, 0, ?2, (
                 SELECT COALESCE(MIN(sort_order), 0) - 1 FROM todos WHERE done = 0
             ))",
            rusqlite::params![text, now],
        )?;
        Ok(TodoItem {
            id: self.conn.last_insert_rowid(),
            text: text.to_string(),
            done: false,
            created_at: Some(now),
            done_at: None,
            has_note: false,
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

    /// 改标题（文本须先经 validate_text，且调用方须传 **trim 后**文本——trim
    /// 执行点在命令层，FIX009.7 同款契约声明）；零行返回 NotFound
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

    /// 行映射（get/list 共用；NULL 时刻映射 Option::None；`has_note` 由 SELECT 里的
    /// EXISTS 子查询单点派生——PL027 改口径为「**任一块非空**」：存在图块 或 存在
    /// trim 非空文本块；toggle/rename 经 get(id) 自动覆盖）
    fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<TodoItem> {
        Ok(TodoItem {
            id: row.get(0)?,
            text: row.get(1)?,
            done: row.get::<_, i64>(2)? != 0,
            created_at: row.get(3)?,
            done_at: row.get(4)?,
            has_note: row.get::<_, i64>(5)? != 0,
        })
    }

    /// 条目查询列清单（get/list/list_done 三处共用；第 6 列 = has_note 的 EXISTS 子查询）。
    /// 仅内部字面量拼接（无外部注入面，沿 column_set 的 format! 先例）
    const TODO_COLUMNS: &str = "id, text, done, created_at, done_at,
         EXISTS(SELECT 1 FROM note_blocks nb WHERE nb.todo_id = todos.id
                AND (nb.kind = 'image' OR TRIM(nb.text) <> ''))";

    /// 读取单条；不存在返回 NotFound
    pub fn get(&self, id: i64) -> Result<TodoItem, StorageError> {
        self.conn
            .query_row(
                &format!("SELECT {} FROM todos WHERE id = ?1", Self::TODO_COLUMNS),
                rusqlite::params![id],
                Self::row_to_item,
            )
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound(id),
                other => StorageError::Sqlite(other),
            })
    }

    /// 删除条目；零行删除返回 NotFound。PL027：**连带删除该条目的详情块行**（其图片
    /// 文件由调用方经 `sweep_orphan_note_images` 释放——此时引用已归零）
    pub fn remove(&self, id: i64) -> Result<(), StorageError> {
        self.conn.execute(
            "DELETE FROM note_blocks WHERE todo_id = ?1",
            rusqlite::params![id],
        )?;
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
    /// 非 NULL 同值按 id 次级排序——SQLite ASC 中 NULL 恒排最前，迁移满射回填后无
    /// NULL，此处为防御，FIX011.3 措辞精确化）
    pub fn list(&self) -> Result<Vec<TodoItem>, StorageError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {} FROM todos
             ORDER BY done ASC, sort_order ASC, id ASC",
            Self::TODO_COLUMNS
        ))?;
        let rows = stmt.query_map([], Self::row_to_item)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 归档视图：已完成条目按 done_at 倒序（最新完成的在最上，V0.015 用户定案）；
    /// done_at 相同（同秒完成/迁移 NULL 行）按 id 倒序保持稳定序
    pub fn list_done(&self) -> Result<Vec<TodoItem>, StorageError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {} FROM todos
             WHERE done = 1
             ORDER BY done_at DESC, id DESC",
            Self::TODO_COLUMNS
        ))?;
        let rows = stmt.query_map([], Self::row_to_item)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 归档回收（PATCH001）：删除已完成且 done_at 早于 cutoff 的条目，返回删除行数。
    /// `done_at IS NOT NULL` 谓词 = 迁移遗留 NULL 行永不回收（PL010"不模拟历史时间"
    /// 先例）；未完成条目（done = 0）不在谓词内。cutoff 由调用方按业务常量
    /// todo::ARCHIVE_RETENTION_MS 计算——存储层不持保留期策略。
    pub fn purge_expired_done(&self, cutoff: i64) -> Result<usize, StorageError> {
        // PL027：连带删除待回收条目的详情块行（先删块再删条目——子查询须在条目还在时求值）；
        // 其图片文件由调用方经 sweep_orphan_note_images 释放（此时引用已归零）
        self.conn.execute(
            "DELETE FROM note_blocks WHERE todo_id IN (
                 SELECT id FROM todos WHERE done = 1 AND done_at IS NOT NULL AND done_at < ?1
             )",
            rusqlite::params![cutoff],
        )?;
        let changed = self.conn.execute(
            "DELETE FROM todos
             WHERE done = 1 AND done_at IS NOT NULL AND done_at < ?1",
            rusqlite::params![cutoff],
        )?;
        Ok(changed)
    }

    // —— PL027 详情块（note_blocks：一段一行一块 + 图块混排，域内引用计数）——

    /// 详情块行映射（读路径共用；kind 未知值宽容回落 Text，沿 row_to_bubble 先例）
    fn row_to_note_block(row: &rusqlite::Row<'_>) -> rusqlite::Result<NoteBlock> {
        Ok(NoteBlock {
            id: row.get(0)?,
            kind: match row.get::<_, String>(1)?.as_str() {
                "image" => NoteBlockKind::Image,
                _ => NoteBlockKind::Text,
            },
            text: row.get(2)?,
            image_file: row.get(3)?,
        })
    }

    /// 读该条目的详情块（sort_order 升序 = 展示序，id 兜底）
    pub fn note_blocks(&self, todo_id: i64) -> Result<Vec<NoteBlock>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, text, image_file FROM note_blocks
             WHERE todo_id = ?1 ORDER BY sort_order ASC, id ASC",
        )?;
        let rows = stmt.query_map([todo_id], Self::row_to_note_block)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 读单个块；不存在 NotFound（开原图取数源，PL027）
    pub fn get_note_block(&self, id: i64) -> Result<NoteBlock, StorageError> {
        self.conn
            .query_row(
                "SELECT id, kind, text, image_file FROM note_blocks WHERE id = ?1",
                [id],
                Self::row_to_note_block,
            )
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound(id),
                other => StorageError::Sqlite(other),
            })
    }

    /// 该条目全部图块的图片相对路径（释放/迁移用）
    fn note_image_paths(&self, todo_id: i64) -> Result<Vec<String>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT image_file FROM note_blocks WHERE todo_id = ?1 AND image_file IS NOT NULL",
        )?;
        let rows = stmt.query_map([todo_id], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 释放详情图引用（PL027 域内引用计数）：该文件**已无任何块引用**才递归删其
    /// `note_{…}` 文件夹；删除失败 best-effort 落日志不阻断（文件残留代价小于阻断用户操作）
    fn release_note_image(&self, images_dir: &Path, rel: &str) -> Result<(), StorageError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM note_blocks WHERE image_file = ?1",
            [rel],
            |row| row.get(0),
        )?;
        if count > 0 {
            return Ok(());
        }
        let dir = images_dir.join(folder_of(rel));
        if let Err(err) = std::fs::remove_dir_all(&dir) {
            eprintln!("详情图文件夹删除失败（best-effort 不阻断）：{err}");
        }
        Ok(())
    }

    /// 孤儿清扫式引用释放（PL027；供删条目 / 归档回收后调用）：删除 `images/` 下所有
    /// **已无块引用**的 `note_*` 文件夹（与精确释放等价，且顺带清理崩溃遗留孤儿文件）；
    /// **只扫 `note_` 前缀**（气泡 `bubble_*` 域一行不碰）。返回清理的文件夹数
    pub fn sweep_orphan_note_images(&self, images_dir: &Path) -> Result<usize, StorageError> {
        let entries = match std::fs::read_dir(images_dir) {
            Ok(e) => e,
            Err(err) => {
                eprintln!("详情图清扫跳过（目录不可读）：{err}");
                return Ok(0);
            }
        };
        let mut removed = 0usize;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if !name.starts_with("note_") || !entry.path().is_dir() {
                continue;
            }
            let referenced: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM note_blocks WHERE image_file LIKE ?1",
                [format!("{name}/%")],
                |row| row.get(0),
            )?;
            if referenced > 0 {
                continue;
            }
            if let Err(err) = std::fs::remove_dir_all(entry.path()) {
                eprintln!("详情图孤儿文件夹删除失败（best-effort 不阻断）：{err}");
                continue;
            }
            removed += 1;
        }
        Ok(removed)
    }

    /// 全量覆盖写块（PL027）：事务内**按 id 差分 upsert**（id > 0 = 更新、id = 0 = 新增），
    /// 并删除清单外的旧块（图块 id 恒 > 0——新图块只由粘贴命令产生）。形态不变量由调用方
    /// 经 `todo::validate_note_blocks` 把关。**被移除图块的文件引用一并精确释放**（块集
    /// 变更即引用集变更的固有语义）。返回落库后的新块列表（含回填 id）
    pub fn set_note_blocks(
        &self,
        images_dir: &Path,
        todo_id: i64,
        blocks: &[NoteBlock],
    ) -> Result<Vec<NoteBlock>, StorageError> {
        let old_paths = self.note_image_paths(todo_id)?;
        let keep_ids: Vec<i64> = blocks.iter().map(|b| b.id).filter(|id| *id > 0).collect();
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), StorageError> {
            let existing: Vec<i64> = self
                .conn
                .prepare("SELECT id FROM note_blocks WHERE todo_id = ?1")?
                .query_map([todo_id], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;
            for id in existing {
                if !keep_ids.contains(&id) {
                    self.conn
                        .execute("DELETE FROM note_blocks WHERE id = ?1", [id])?;
                }
            }
            for (i, block) in blocks.iter().enumerate() {
                match block.kind {
                    NoteBlockKind::Text => {
                        let text = block.text.as_str();
                        if block.id > 0 {
                            let changed = self.conn.execute(
                                "UPDATE note_blocks SET sort_order = ?2, kind = 'text', text = ?3,
                                     image_hash = NULL, image_file = NULL
                                 WHERE id = ?1 AND todo_id = ?4",
                                rusqlite::params![block.id, i as i64, text, todo_id],
                            )?;
                            if changed == 0 {
                                return Err(StorageError::NotFound(block.id));
                            }
                        } else {
                            self.conn.execute(
                                "INSERT INTO note_blocks(todo_id, sort_order, kind, text)
                                 VALUES (?1, ?2, 'text', ?3)",
                                rusqlite::params![todo_id, i as i64, text],
                            )?;
                        }
                    }
                    NoteBlockKind::Image => {
                        // 图块只可能来自粘贴命令（落盘 + hash 已就位）——此处只改序
                        let changed = self.conn.execute(
                            "UPDATE note_blocks SET sort_order = ?2, kind = 'image'
                             WHERE id = ?1 AND todo_id = ?3",
                            rusqlite::params![block.id, i as i64, todo_id],
                        )?;
                        if changed == 0 {
                            return Err(StorageError::NotFound(block.id));
                        }
                    }
                }
            }
            Ok(())
        })();
        match result {
            Ok(()) => self.conn.execute_batch("COMMIT")?,
            Err(err) => {
                if let Err(rollback) = self.conn.execute_batch("ROLLBACK") {
                    eprintln!("详情块写入事务回滚失败：{rollback}");
                }
                return Err(err);
            }
        }
        // 释放不再被引用的图文件（旧路径 − **落库后仍存的**路径；引用数归零才删）。
        // 用落库后查询而非入参推断：入参只带 {id, kind, text}，图块文件信息在库内
        let surviving = self.note_image_paths(todo_id)?;
        for rel in old_paths {
            if !surviving.contains(&rel) {
                self.release_note_image(images_dir, &rel)?;
            }
        }
        self.note_blocks(todo_id)
    }

    /// 粘贴插图（PL027）：图片字节 → **同 hash 复用同一文件**（域内引用计数，命中不写盘）
    /// 或新落盘 `note_{stamp}_{n}/…`（内层名：截图 `snap_{stamp}_{n}.png` / 文件图净化原名；
    /// 大图预览阈值同气泡）→ 事务内把 `block_id` 的文本按 `before` / `after` 切成两块 +
    /// 中间插图块（保持 sort_order 连续）→ 返回新块列表。调用方须先 `validate_image_bytes`
    /// 与 `validate_note_blocks`；`before`/`after` 由前端按光标/选区本地切好（免 UTF-16 换算）
    #[allow(clippy::too_many_arguments)] // 粘贴操作的完整输入集（切分点/字节/来源名/时间戳），
                                         // 打包成结构体只会把参数搬到别处、无实益
    pub fn add_note_image_block(
        &self,
        images_dir: &Path,
        todo_id: i64,
        block_id: i64,
        before: &str,
        after: &str,
        bytes: &[u8],
        orig_name: Option<&str>,
        stamp: &str,
    ) -> Result<Vec<NoteBlock>, StorageError> {
        let hash = fnv1a_hex(bytes);
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT image_file FROM note_blocks
                 WHERE image_hash = ?1 AND image_file IS NOT NULL LIMIT 1",
                [&hash],
                |row| row.get(0),
            )
            .optional()?;
        let rel = match existing {
            Some(rel) => rel,
            None => {
                let folder = next_seq_folder(images_dir, "note", stamp);
                let seq = folder.rsplit('_').next().unwrap_or("1");
                let inner = match orig_name {
                    Some(name) => sanitize_file_name(name),
                    None => format!("snap_{stamp}_{seq}.png"),
                };
                let dir = images_dir.join(&folder);
                std::fs::create_dir_all(&dir)?;
                std::fs::write(dir.join(&inner), bytes)?;
                if let Some(preview) = crate::clipboard_image::generate_preview(bytes) {
                    if let Err(err) = std::fs::write(dir.join("preview.png"), &preview) {
                        eprintln!("详情图预览写入失败（降级用原图）：{err}");
                    }
                }
                format!("{folder}/{inner}")
            }
        };
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), StorageError> {
            let order: i64 = self.conn.query_row(
                "SELECT sort_order FROM note_blocks WHERE id = ?1 AND todo_id = ?2",
                rusqlite::params![block_id, todo_id],
                |row| row.get(0),
            )?;
            let follows: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM note_blocks WHERE todo_id = ?1 AND sort_order > ?2",
                rusqlite::params![todo_id, order],
                |row| row.get(0),
            )?;
            // PATCH002 建块规则：上段为空 → 原块**改图块**（顶替，不留前导空块）；
            // 下段为空 → 不留尾块，**但图成为整条最后一块时补一个空文本块**（否则图后无法输入）
            let keep_before = !before.trim().is_empty();
            let keep_after = !after.trim().is_empty();
            let tail_blank = !keep_after && follows == 0;
            let shift = i64::from(keep_before) + i64::from(keep_after) + i64::from(tail_blank);
            // 该块之后的块后移（腾出新增块的位置）
            self.conn.execute(
                "UPDATE note_blocks SET sort_order = sort_order + ?3
                 WHERE todo_id = ?1 AND sort_order > ?2",
                rusqlite::params![todo_id, order, shift],
            )?;
            let mut next = order + 1;
            if keep_before {
                // 原块保留为「上段」文本块，图块插在其后
                self.conn.execute(
                    "UPDATE note_blocks SET text = ?2, kind = 'text',
                         image_hash = NULL, image_file = NULL
                     WHERE id = ?1",
                    rusqlite::params![block_id, before],
                )?;
                self.conn.execute(
                    "INSERT INTO note_blocks(todo_id, sort_order, kind, image_hash, image_file)
                     VALUES (?1, ?2, 'image', ?3, ?4)",
                    rusqlite::params![todo_id, next, hash, rel],
                )?;
                next += 1;
            } else {
                // 上段为空：原块直接变图块（顶替，不留前导空块）
                self.conn.execute(
                    "UPDATE note_blocks SET kind = 'image', text = '',
                         image_hash = ?2, image_file = ?3
                     WHERE id = ?1",
                    rusqlite::params![block_id, hash, rel],
                )?;
            }
            if keep_after {
                self.conn.execute(
                    "INSERT INTO note_blocks(todo_id, sort_order, kind, text)
                     VALUES (?1, ?2, 'text', ?3)",
                    rusqlite::params![todo_id, next, after],
                )?;
                next += 1;
            }
            if tail_blank {
                self.conn.execute(
                    "INSERT INTO note_blocks(todo_id, sort_order, kind, text)
                     VALUES (?1, ?2, 'text', '')",
                    rusqlite::params![todo_id, next],
                )?;
            }
            // 保底：结果无文本块 → 补一个空文本块（"至少一个文本块"不变量）
            let texts: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM note_blocks WHERE todo_id = ?1 AND kind = 'text'",
                [todo_id],
                |row| row.get(0),
            )?;
            if texts == 0 {
                let last: i64 = self.conn.query_row(
                    "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM note_blocks WHERE todo_id = ?1",
                    [todo_id],
                    |row| row.get(0),
                )?;
                self.conn.execute(
                    "INSERT INTO note_blocks(todo_id, sort_order, kind, text)
                     VALUES (?1, ?2, 'text', '')",
                    rusqlite::params![todo_id, last],
                )?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => self.conn.execute_batch("COMMIT")?,
            Err(err) => {
                if let Err(rollback) = self.conn.execute_batch("ROLLBACK") {
                    eprintln!("插图事务回滚失败：{rollback}");
                }
                return Err(err);
            }
        }
        self.note_blocks(todo_id)
    }

    /// 删除单个详情块（PL027 图块删除/合并残留）：连带释放该图块文件引用；零行 NotFound
    pub fn remove_note_block(&self, images_dir: &Path, id: i64) -> Result<(), StorageError> {
        let rel: Option<String> = self
            .conn
            .query_row(
                "SELECT image_file FROM note_blocks WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let changed = self
            .conn
            .execute("DELETE FROM note_blocks WHERE id = ?1", [id])?;
        if changed == 0 {
            return Err(StorageError::NotFound(id));
        }
        if let Some(rel) = rel {
            self.release_note_image(images_dir, &rel)?;
        }
        Ok(())
    }

    /// 详情块重排（图块拖拽落点提交）：id 集合须与该 todo 现存块完全一致（长度/幽灵/重复），
    /// 通过则按序 UPDATE sort_order（沿 reorder_todos 先例）
    pub fn reorder_note_blocks(&self, todo_id: i64, ids: &[i64]) -> Result<(), StorageError> {
        let mut existing: Vec<i64> = self
            .conn
            .prepare("SELECT id FROM note_blocks WHERE todo_id = ?1")?
            .query_map([todo_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        existing.sort_unstable();
        Self::validate_reorder_ids(ids, &existing, "详情块集")?;
        for (i, id) in ids.iter().enumerate() {
            self.conn.execute(
                "UPDATE note_blocks SET sort_order = ?2 WHERE id = ?1",
                rusqlite::params![id, i as i64],
            )?;
        }
        Ok(())
    }

    /// 气泡行映射（get/list 共用，FIX008.9 收敛双份闭包；镜像 todo row_to_item 形态）。
    /// kind 值域由自有写入收敛（'text'/'image' 字面量），未知值宽容回落 Text
    fn row_to_bubble(row: &rusqlite::Row<'_>) -> rusqlite::Result<BubbleItem> {
        Ok(BubbleItem {
            id: row.get(0)?,
            text: row.get(1)?,
            kind: match row.get::<_, String>(2)?.as_str() {
                "image" => crate::bubble::BubbleKind::Image,
                _ => crate::bubble::BubbleKind::Text,
            },
        })
    }

    /// 新增气泡（文本须先经 bubble::validate_bubble_text 校验，且调用方须传
    /// **trim 后**文本——trim 执行点在命令层，直调本方法绕过命令层即失守此契约，
    /// FIX008.9 显形；重复文本拒入库返回 Duplicate——PL015.5 去重裁决，手动捕获
    /// 与热键捕获两入口天然同规）；
    /// sort_order = 现存最小值 − 1（**排头插入**——design captureBubble unshift 语义：
    /// 新捕获的气泡永远在最上，升序输出即新在前；拖拽重排后取 MIN−1 依然排头）
    pub fn add_bubble(&self, text: &str) -> Result<BubbleAddOutcome, StorageError> {
        let dup: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM bubbles WHERE text = ?1",
            [text],
            |row| row.get(0),
        )?;
        if dup > 0 {
            return Ok(BubbleAddOutcome::Duplicate);
        }
        self.conn.execute(
            "INSERT INTO bubbles(text, sort_order)
             VALUES (?1, (SELECT COALESCE(MIN(sort_order), 0) - 1 FROM bubbles))",
            [text],
        )?;
        Ok(BubbleAddOutcome::Added(BubbleItem {
            id: self.conn.last_insert_rowid(),
            text: text.to_string(),
            kind: crate::bubble::BubbleKind::Text,
        }))
    }

    /// 新增图片气泡（PL025 落盘化）：算内容哈希（FNV-1a）→ 查重（同 hash 已在库返
    /// Duplicate）→ 事务内 INSERT 取 id → 写盘 `bubble_{stamp}_{n}/{内层名}` + 生成
    /// 大图预览 → UPDATE image_file（**相对路径含子目录**）。排头插入同 add_bubble。
    /// `orig_name` = 文件来源原名（None = 截图）；bytes 须先过
    /// clipboard_image::validate_image_bytes（格式嗅探执行点在命令层）
    pub fn add_image_bubble(
        &self,
        images_dir: &Path,
        text: &str,
        bytes: &[u8],
        orig_name: Option<&str>,
        stamp: &str,
    ) -> Result<BubbleAddOutcome, StorageError> {
        let hash = fnv1a_hex(bytes);
        let dup: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM bubbles WHERE kind = 'image' AND image_hash = ?1",
            [&hash],
            |row| row.get(0),
        )?;
        if dup > 0 {
            return Ok(BubbleAddOutcome::Duplicate);
        }
        let source = if orig_name.is_some() {
            "file"
        } else {
            "screenshot"
        };
        let folder = next_seq_folder(images_dir, "bubble", stamp);
        let seq = folder.rsplit('_').next().unwrap_or("1");
        let inner = match orig_name {
            Some(name) => sanitize_file_name(name),
            None => format!("snap_{stamp}_{seq}.png"),
        };
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<BubbleAddOutcome, StorageError> {
            self.conn.execute(
                "INSERT INTO bubbles(text, kind, image_hash, image_source, sort_order)
                 VALUES (?1, 'image', ?2, ?3,
                         (SELECT COALESCE(MIN(sort_order), 0) - 1 FROM bubbles))",
                rusqlite::params![text, hash, source],
            )?;
            let id = self.conn.last_insert_rowid();
            let dir = images_dir.join(&folder);
            std::fs::create_dir_all(&dir)?;
            std::fs::write(dir.join(&inner), bytes)?;
            // 大图预览（失败降级不阻断——详情退回原图）
            if let Some(preview) = crate::clipboard_image::generate_preview(bytes) {
                if let Err(err) = std::fs::write(dir.join("preview.png"), &preview) {
                    eprintln!("图片预览写入失败（降级用原图）：{err}");
                }
            }
            let rel = format!("{folder}/{inner}");
            self.conn.execute(
                "UPDATE bubbles SET image_file = ?1 WHERE id = ?2",
                rusqlite::params![rel, id],
            )?;
            Ok(BubbleAddOutcome::Added(BubbleItem {
                id,
                text: text.to_string(),
                kind: crate::bubble::BubbleKind::Image,
            }))
        })();
        match result {
            Ok(outcome) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(outcome)
            }
            Err(err) => {
                if let Err(rollback) = self.conn.execute_batch("ROLLBACK") {
                    eprintln!("图片入库事务回滚失败：{rollback}");
                }
                Err(err)
            }
        }
    }

    /// 读图片气泡的落盘相对文件名 + 来源（详情/复制回取数源，PL025）：不存在 NotFound
    /// 严格报错；文本气泡或无文件名返回 None（正常态非错误——调用面只对 kind=Image 调用）
    pub fn get_bubble_image_file(&self, id: i64) -> Result<Option<(String, String)>, StorageError> {
        self.get_bubble(id)?;
        let (file, source): (Option<String>, String) = self.conn.query_row(
            "SELECT image_file, image_source FROM bubbles WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(file.map(|f| (f, source)))
    }

    /// 气泡列表：按 sort_order 升序（拖拽序；新捕获排头插入 = 展示序与实验场
    /// unshift 语义一致，历史 id 倒序语义由迁移回填保持）
    pub fn list_bubbles(&self) -> Result<Vec<BubbleItem>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, text, kind FROM bubbles ORDER BY sort_order ASC, id ASC")?;
        let rows = stmt.query_map([], Self::row_to_bubble)?;
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
        Self::validate_reorder_ids(ids, &active, "未完成集")?;
        self.reorder_in_transaction(ids, "todos")
    }

    /// 气泡重排（拖拽落点提交）：ids = 全量气泡的目标顺序。校验与事务语义同 reorder_todos
    pub fn reorder_bubbles(&self, ids: &[i64]) -> Result<(), StorageError> {
        let mut current: Vec<i64> = self
            .conn
            .prepare("SELECT id FROM bubbles")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        current.sort_unstable();
        Self::validate_reorder_ids(ids, &current, "气泡集")?;
        self.reorder_in_transaction(ids, "bubbles")
    }

    /// 重排 id 集三重校验（FIX005.27 收敛）：去重后与现存集长度一致、集合相等——
    /// 任一不符返回 ReorderMismatch（防丢行/幽灵行/重复 id，拒绝执行）
    fn validate_reorder_ids(
        ids: &[i64],
        active: &[i64],
        active_label: &str,
    ) -> Result<(), StorageError> {
        let mut given: Vec<i64> = ids.to_vec();
        given.sort_unstable();
        given.dedup();
        if given.len() != ids.len() || given != active {
            return Err(StorageError::ReorderMismatch(format!(
                "ids 长度 {}（去重 {}）与{} {} 不一致",
                ids.len(),
                given.len(),
                active_label,
                active.len()
            )));
        }
        Ok(())
    }

    /// 重排事务骨架（FIX005.27 收敛，两 reorder 共用）：表名按字面量拼定值子句
    ///（非用户输入，无注入面），逐条 UPDATE 后提交；失败回滚落日志并上抛
    fn reorder_in_transaction(&self, ids: &[i64], table: &str) -> Result<(), StorageError> {
        self.conn.execute_batch("BEGIN")?;
        let sql = format!("UPDATE {table} SET sort_order = ?1 WHERE id = ?2");
        let result = (|| -> Result<(), StorageError> {
            for (order, id) in ids.iter().enumerate() {
                self.conn
                    .execute(&sql, rusqlite::params![order as i64, id])?;
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
                    eprintln!("重排事务回滚失败：{rollback}");
                }
                Err(err)
            }
        }
    }

    /// 读取单条气泡（复制回剪贴板的取数源）；不存在返回 NotFound
    pub fn get_bubble(&self, id: i64) -> Result<BubbleItem, StorageError> {
        self.conn
            .query_row(
                "SELECT id, text, kind FROM bubbles WHERE id = ?1",
                rusqlite::params![id],
                Self::row_to_bubble,
            )
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound(id),
                other => StorageError::Sqlite(other),
            })
    }

    /// 删除单条气泡；返回其**所属文件夹**相对路径（供调用方递归删，best-effort）；零行返回 NotFound
    pub fn remove_bubble(&self, id: i64) -> Result<Option<String>, StorageError> {
        let rel: Option<Option<String>> = self
            .conn
            .query_row(
                "SELECT image_file FROM bubbles WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(rel) = rel else {
            return Err(StorageError::NotFound(id));
        };
        self.conn
            .execute("DELETE FROM bubbles WHERE id = ?1", rusqlite::params![id])?;
        Ok(rel.map(|r| folder_of(&r)))
    }

    /// 一键清空气泡：返回（清除条数, 所属**文件夹**相对路径列表——供调用方递归删 best-effort）
    pub fn clear_bubbles(&self) -> Result<(usize, Vec<String>), StorageError> {
        let rels: Vec<String> = self
            .conn
            .prepare("SELECT image_file FROM bubbles WHERE image_file IS NOT NULL")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let removed = self.conn.execute("DELETE FROM bubbles", [])?;
        let folders = rels.iter().map(|r| folder_of(r)).collect();
        Ok((removed, folders))
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
    // FIX011.5：测试域时钟原子量统一 mod 级导入（取代各测试 fn 内散装 use）
    use std::sync::atomic::{AtomicI64, Ordering};

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
        // 未完成段先行；完成段按勾选前 sort_order 快照升序——置顶插入后加入序
        // 倒挂（后加者更小），c 先于 a（2026-09-30 排序语义变更连带）
        assert_eq!(ids, vec![b.id, c.id, a.id]);
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
        let a = st.add_bubble("片段一").expect("写入必须成功").added_item();
        let b = st.add_bubble("片段二").expect("写入必须成功").added_item();
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
        st.add_bubble("一").expect("写入必须成功").added_item();
        st.add_bubble("二").expect("写入必须成功").added_item();
        let (removed, _rels) = st.clear_bubbles().expect("清空必须成功");
        assert_eq!(removed, 2);
        assert_eq!(st.list_bubbles().expect("读取必须成功").len(), 0);
    }

    #[test]
    fn bubble_add_duplicate_returns_duplicate_outcome() {
        // PL015.5 去重裁决：同文本拒入库（手动捕获与热键捕获两入口同规）
        let st = storage();
        let first = st.add_bubble("重复片段").expect("写入必须成功");
        assert!(!first.is_duplicate());
        let second = st.add_bubble("重复片段").expect("查询必须成功");
        assert!(second.is_duplicate(), "重复文本必须返回 Duplicate");
        assert_eq!(
            st.list_bubbles().expect("读取必须成功").len(),
            1,
            "重复不入库"
        );
    }

    #[test]
    fn bubble_add_distinct_text_still_added() {
        // 去重只拦同文本：不同文本正常新增
        let st = storage();
        st.add_bubble("片段一").expect("写入必须成功").added_item();
        st.add_bubble("片段二").expect("写入必须成功").added_item();
        assert_eq!(st.list_bubbles().expect("读取必须成功").len(), 2);
    }

    #[test]
    fn bubble_add_duplicate_allows_readd_after_removal() {
        // 删除后同文本可再入（去重按现存集合裁决，非历史黑名单）
        let st = storage();
        let item = st
            .add_bubble("临时片段")
            .expect("写入必须成功")
            .added_item();
        st.remove_bubble(item.id).expect("删除必须成功");
        let re = st.add_bubble("临时片段").expect("写入必须成功");
        assert!(!re.is_duplicate(), "删除后可再添加同文本");
    }

    // ===== PL024.1 图片气泡存储（TDD 红灯） =====

    /// 图片内容夹具（storage 层不校验格式——任意字节即可；格式校验在命令层）
    fn png_bytes() -> Vec<u8> {
        vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3]
    }

    /// 临时图片目录（测试用，落系统临时目录，禁触真实 data/images）
    fn temp_images_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "capsule-todo-images-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("临时图片目录必须可建");
        dir
    }

    #[test]
    fn migration_backfills_bubble_kind_text() {
        // legacy bubbles（PL013 fixture，无 kind/image 列）→ 迁移后存量行 kind = Text
        let st = legacy_storage_with_bubbles();
        st.init().expect("迁移必须成功");
        let items = st.list_bubbles().expect("读取必须成功");
        assert!(!items.is_empty());
        for it in &items {
            assert_eq!(
                it.kind,
                crate::bubble::BubbleKind::Text,
                "存量行 = 文本气泡"
            );
        }
    }

    #[test]
    fn add_image_bubble_dedupes_by_content() {
        // 图片按内容去重（PL025）：同字节第二次 Duplicate；不同字节入库；子文件夹 + 回填
        let st = storage();
        let dir = temp_images_dir();
        let png = png_bytes();
        let a = st
            .add_image_bubble(&dir, "🖼️ 截图 261009143205", &png, None, "261009143205")
            .expect("首次必须成功")
            .added_item();
        let dup = st
            .add_image_bubble(&dir, "🖼️ 截图 261009143206", &png, None, "261009143206")
            .expect("去重判定必须成功");
        assert!(dup.is_duplicate(), "同字节应判重复");
        let mut other = png.clone();
        other.push(0);
        let b = st
            .add_image_bubble(
                &dir,
                "🖼️ 图片 示例.png",
                &other,
                Some("示例.png"),
                "261009143207",
            )
            .expect("不同字节必须成功")
            .added_item();
        let items = st.list_bubbles().expect("读取必须成功");
        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .all(|it| it.kind == crate::bubble::BubbleKind::Image));
        // 截图：文件夹 bubble_{stamp}_1 + 内层 snap_{stamp}_1.png
        let (rel_a, src_a) = st
            .get_bubble_image_file(a.id)
            .expect("读取必须成功")
            .expect("应有文件");
        assert_eq!(rel_a, "bubble_261009143205_1/snap_261009143205_1.png");
        assert_eq!(src_a, "screenshot");
        assert_eq!(std::fs::read(dir.join(&rel_a)).expect("回读必须成功"), png);
        // 文件图：内层原名
        let (rel_b, src_b) = st
            .get_bubble_image_file(b.id)
            .expect("读取必须成功")
            .expect("应有文件");
        assert_eq!(rel_b, "bubble_261009143207_1/示例.png");
        assert_eq!(src_b, "file");
        assert_eq!(
            std::fs::read(dir.join(&rel_b)).expect("回读必须成功"),
            other
        );
    }

    #[test]
    fn migrate_blobs_to_files_and_drop_column_idempotent() {
        // 老库（含 image 列）存量 BLOB → 写盘 + 回填 + 清空；二次跑零变化；drop 幂等
        let st = storage();
        st.conn
            .execute("ALTER TABLE bubbles ADD COLUMN image BLOB", [])
            .expect("模拟老库加列必须成功");
        st.conn
            .execute(
                "INSERT INTO bubbles(text, kind, image) VALUES ('🖼 截图 1', 'image', ?1)",
                rusqlite::params![png_bytes()],
            )
            .expect("插存量 blob 必须成功");
        let dir = temp_images_dir();
        st.migrate_blobs_to_files(&dir, "261010120000")
            .expect("迁移必须成功");
        let (file, blob): (Option<String>, Option<Vec<u8>>) = st
            .conn
            .query_row(
                "SELECT image_file, image FROM bubbles WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("读回必须成功");
        let rel = file.expect("image_file 应回填");
        assert_eq!(rel, "bubble_261010120000_1/snap_261010120000_1.png");
        assert!(blob.is_none(), "BLOB 应清空");
        assert!(dir.join(&rel).is_file(), "文件应落盘");
        st.migrate_blobs_to_files(&dir, "261010120000")
            .expect("二次迁移幂等");
        st.drop_image_column().expect("drop 必须成功");
        assert!(!st
            .column_set("bubbles")
            .expect("探列必须成功")
            .contains("image"));
        st.drop_image_column().expect("drop 幂等");
    }

    #[test]
    fn get_bubble_image_file_text_none_and_missing_errors() {
        // 文本气泡 = None（非错误）；不存在 = NotFound 严格报错
        let st = storage();
        let item = st.add_bubble("文本").expect("写入必须成功").added_item();
        assert_eq!(
            st.get_bubble_image_file(item.id).expect("读取必须成功"),
            None
        );
        assert!(matches!(
            st.get_bubble_image_file(99),
            Err(StorageError::NotFound(99))
        ));
    }

    #[test]
    fn add_bubble_text_path_stays_text_kind() {
        // 文本路径回归锁定：kind = Text（无 image_file 由 get_bubble_image_file None 证）
        let st = storage();
        let item = st.add_bubble("文本").expect("写入必须成功").added_item();
        assert_eq!(item.kind, crate::bubble::BubbleKind::Text);
        assert_eq!(
            st.get_bubble_image_file(item.id).expect("读取必须成功"),
            None
        );
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
        // 迁移后新字段可读写：存量行 created_at/done_at = NULL（不模拟时间）；
        // PL027：无 note 列的旧库无笔记可迁（note 列不再新建），块集为空
        let items = st.list().expect("读取必须成功");
        assert_eq!(items.len(), 2);
        for it in &items {
            assert_eq!(it.created_at, None, "存量行 created_at 必须为 NULL");
            assert_eq!(it.done_at, None, "存量行 done_at 必须为 NULL");
            assert!(!it.has_note, "无块 → has_note 不亮");
            assert!(st.note_blocks(it.id).expect("读块必须成功").is_empty());
        }
    }

    /// 造 PL010 期库（todos 含 note 列、无 note_blocks）——测 PL027 笔记→块迁移
    fn pl010_storage_with_notes() -> Storage {
        let conn = Connection::open_in_memory().expect("内存库必须可开");
        conn.execute_batch(
            "CREATE TABLE todos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER,
                done_at INTEGER,
                note TEXT NOT NULL DEFAULT ''
            );
            INSERT INTO todos(text, done, created_at, note) VALUES
                ('单行', 0, 1, '只有一行'),
                ('多行', 0, 2, '第一行
第二行'),
                ('空笔记', 0, 3, ''),
                ('空行归一', 0, 4, '

正文');",
        )
        .expect("造 PL010 期库必须成功");
        Storage {
            conn,
            now: Arc::new(system_now),
        }
    }

    #[test]
    fn migration_splits_note_into_blocks_and_drops_column() {
        // PL027：存量 note（单行/多行/空）→ 文本块（一段一行一块）；迁移后 note 列退役
        let st = pl010_storage_with_notes();
        st.init().expect("迁移必须成功");
        let items = st.list().expect("读取必须成功");
        let id_of = |text: &str| -> i64 {
            items
                .iter()
                .find(|i| i.text == text)
                .map(|i| i.id)
                .expect("存量行必须在")
        };
        let single = st.note_blocks(id_of("单行")).expect("读块必须成功");
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].text, "只有一行");
        let multi = st.note_blocks(id_of("多行")).expect("读块必须成功");
        assert_eq!(multi.len(), 2, "多行按换行拆成两块");
        assert_eq!(multi[0].text, "第一行");
        assert_eq!(multi[1].text, "第二行");
        let empty = st.note_blocks(id_of("空笔记")).expect("读块必须成功");
        assert_eq!(
            empty.len(),
            1,
            "空 note → 一个空文本块（不变量：至少一个文本块）"
        );
        assert_eq!(empty[0].text, "");
        assert!(st.get(id_of("单行")).expect("读必须成功").has_note);
        assert!(!st.get(id_of("空笔记")).expect("读必须成功").has_note);
        // 幂等：二次 init 不重复建块
        st.init().expect("二次迁移必须幂等");
        assert_eq!(
            st.note_blocks(id_of("多行")).expect("读块必须成功").len(),
            2
        );
        // note 列已退役（再读该列必然报错）
        assert!(st
            .conn
            .query_row("SELECT note FROM todos LIMIT 1", [], |row| row
                .get::<_, String>(0))
            .is_err());
    }

    #[test]
    fn migration_is_idempotent() {
        let st = legacy_storage();
        st.init().expect("首次迁移必须成功");
        st.init().expect("二次迁移必须幂等（列已存在不重复 ALTER）");
        assert_eq!(st.list().expect("读取必须成功").len(), 2);
    }

    #[test]
    fn new_row_gets_created_at_and_no_blocks() {
        // 注入固定时间源：add 落 created_at = now（PL027：新行无块、has_note 不亮）
        let fixed = 1_700_000_000_000i64;
        let st = Storage::open_in_memory_with_now(Arc::new(move || fixed)).expect("库必须可开");
        let item = st.add("新条目").expect("写入必须成功");
        assert_eq!(item.created_at, Some(fixed));
        assert_eq!(item.done_at, None);
        assert!(!item.has_note);
        assert!(st.note_blocks(item.id).expect("读块必须成功").is_empty());
        let read = st.get(item.id).expect("读取必须成功");
        assert_eq!(read.created_at, Some(fixed));
    }

    #[test]
    fn toggle_sets_done_at_forward_and_nulls_backward() {
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
        let clock = Arc::new(AtomicI64::new(1_700_000_000_000));
        let clock2 = Arc::clone(&clock);
        let st = Storage::open_in_memory_with_now(Arc::new(move || clock2.load(Ordering::SeqCst)))
            .expect("库必须可开");
        (st, clock)
    }

    #[test]
    fn list_done_orders_by_done_at_desc() {
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

    // ===== PATCH001 归档 7 天自动回收（TDD） =====

    #[test]
    fn purge_deletes_only_expired_done_items() {
        let (st, clock) = stepped_storage();
        let t0 = clock.load(Ordering::SeqCst);
        let old = st.add("老条目").expect("写入必须成功");
        let fresh = st.add("新条目").expect("写入必须成功");
        // old 勾选于 T0；fresh 勾选于 T0 + 3 天
        st.toggle(old.id).expect("勾选必须成功");
        clock.store(t0 + 3 * 24 * 3600 * 1000, Ordering::SeqCst);
        st.toggle(fresh.id).expect("勾选必须成功");
        // 推进到恰好 T0 + 7 天：cutoff = T0，old 的 done_at == cutoff 不回收（< 才删）
        clock.store(t0 + crate::todo::ARCHIVE_RETENTION_MS, Ordering::SeqCst);
        let cutoff = clock.load(Ordering::SeqCst) - crate::todo::ARCHIVE_RETENTION_MS;
        assert_eq!(
            st.purge_expired_done(cutoff).expect("回收必须成功"),
            0,
            "恰好满 7 天不回收"
        );
        // 再 +1ms：old 超期 1ms 回收；fresh（done_at = T0+3d）差近 3 天保留
        clock.store(clock.load(Ordering::SeqCst) + 1, Ordering::SeqCst);
        let cutoff = clock.load(Ordering::SeqCst) - crate::todo::ARCHIVE_RETENTION_MS;
        assert_eq!(
            st.purge_expired_done(cutoff).expect("回收必须成功"),
            1,
            "超期 1ms 回收 1 条"
        );
        let rest = st.list_done().expect("读取必须成功");
        assert_eq!(rest.len(), 1, "未满期条目保留在归档");
        assert_eq!(rest[0].id, fresh.id);
        assert!(
            !st.list()
                .expect("读取必须成功")
                .iter()
                .any(|it| it.id == old.id),
            "回收条目从库中消失"
        );
    }

    #[test]
    fn purge_skips_null_done_at_and_unchecked() {
        let (st, clock) = stepped_storage();
        let a = st.add("迁移遗留").expect("写入必须成功");
        let b = st.add("未完成").expect("写入必须成功");
        // 直改 SQL 模拟迁移遗留形态：done = 1 且 done_at = NULL（PL010"不模拟历史
        // 时间"回填语义）——正常路径 toggle 勾选必带 done_at，此形态只能来自存量
        st.conn
            .execute(
                "UPDATE todos SET done = 1, done_at = NULL WHERE id = ?1",
                rusqlite::params![a.id],
            )
            .expect("模拟存量必须成功");
        // 推进远超 7 天：NULL 行与未完成条目均不在回收谓词内
        clock.store(
            clock.load(Ordering::SeqCst) + 100 * 24 * 3600 * 1000,
            Ordering::SeqCst,
        );
        let cutoff = clock.load(Ordering::SeqCst) - crate::todo::ARCHIVE_RETENTION_MS;
        assert_eq!(
            st.purge_expired_done(cutoff).expect("回收必须成功"),
            0,
            "NULL done_at 与未完成条目均不回收"
        );
        assert_eq!(
            st.list_done().expect("读取必须成功").len(),
            1,
            "迁移遗留归档行保留"
        );
        assert!(
            st.list()
                .expect("读取必须成功")
                .iter()
                .any(|it| it.id == b.id),
            "未完成条目不受回收影响"
        );
    }

    #[test]
    fn recheck_restarts_retention_clock() {
        let (st, clock) = stepped_storage();
        let t0 = clock.load(Ordering::SeqCst);
        let item = st.add("任务").expect("写入必须成功");
        st.toggle(item.id).expect("勾选必须成功"); // done_at = T0
                                                   // 6 天后退回：done_at 清 NULL，计时清零
        clock.store(t0 + 6 * 24 * 3600 * 1000, Ordering::SeqCst);
        st.toggle(item.id).expect("退回必须成功");
        // 推进到 T0 + 7 天 + 500ms：若按首次勾选计时早已超期，重计时则未满
        clock.store(
            t0 + crate::todo::ARCHIVE_RETENTION_MS + 500,
            Ordering::SeqCst,
        );
        let cutoff = clock.load(Ordering::SeqCst) - crate::todo::ARCHIVE_RETENTION_MS;
        assert_eq!(
            st.purge_expired_done(cutoff).expect("回收必须成功"),
            0,
            "退回后计时清零不回收"
        );
        // 重勾于此刻 → 新 done_at，要到 T0 + 14 天 + 500ms 才到期
        st.toggle(item.id).expect("重勾必须成功");
        assert_eq!(
            st.purge_expired_done(cutoff).expect("回收必须成功"),
            0,
            "重勾按新 done_at 计时"
        );
        clock.store(
            t0 + 2 * crate::todo::ARCHIVE_RETENTION_MS + 600,
            Ordering::SeqCst,
        );
        let cutoff = clock.load(Ordering::SeqCst) - crate::todo::ARCHIVE_RETENTION_MS;
        assert_eq!(
            st.purge_expired_done(cutoff).expect("回收必须成功"),
            1,
            "新计时满 7 天后回收"
        );
    }

    #[test]
    fn rename_updates_text_only() {
        let st = storage();
        let item = st.add("旧名").expect("写入必须成功");
        let renamed = st.rename(item.id, "新名").expect("改名必须成功");
        assert_eq!(renamed.text, "新名");
        assert!(!renamed.has_note, "改名不触碰详情");
        assert!(matches!(
            st.rename(99, "x"),
            Err(StorageError::NotFound(99))
        ));
    }

    #[test]
    fn has_note_reflects_blocks() {
        // PL027 口径：**任一块非空**——文本块 trim 非空 或 存在图块（纯图条目也亮）
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        assert!(!st.get(item.id).expect("读取必须成功").has_note);
        let saved = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "有内容")])
            .expect("写块必须成功");
        assert!(
            st.get(item.id).expect("读取必须成功").has_note,
            "非空文本块 → 亮"
        );
        st.set_note_blocks(&dir, item.id, &[nb_text(saved[0].id, "   ")])
            .expect("写纯空白必须成功");
        assert!(
            !st.get(item.id).expect("读取必须成功").has_note,
            "纯空白文本块 → 不亮"
        );
        st.set_note_blocks(&dir, item.id, &[nb_text(saved[0].id, "")])
            .expect("清空必须成功");
        assert!(!st.get(item.id).expect("读取必须成功").has_note);
        // 纯图条目也亮（用户定案）
        st.add_note_image_block(
            &dir,
            item.id,
            saved[0].id,
            "",
            "",
            &png_bytes(),
            None,
            "261018000000",
        )
        .expect("插图必须成功");
        assert!(
            st.get(item.id).expect("读取必须成功").has_note,
            "纯图条目 → 亮"
        );
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
        let a = st.add_bubble("泡一").expect("写入必须成功").added_item();
        let b = st.add_bubble("泡二").expect("写入必须成功").added_item();
        let c = st.add_bubble("泡三").expect("写入必须成功").added_item();
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
        let a = st.add_bubble("泡一").expect("写入必须成功").added_item();
        let _b = st.add_bubble("泡二").expect("写入必须成功").added_item();
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
    fn new_rows_get_topmost_sort_order() {
        let st = storage();
        let a = st.add("甲").expect("写入必须成功");
        let b = st.add("乙").expect("写入必须成功");
        st.reorder_todos(&[b.id, a.id]).expect("重排必须成功");
        let c = st.add("丙").expect("写入必须成功");
        // 新行 sort_order 排头：置顶插入不打乱已有排序（list 首位，2026-09-30 用户定案）
        let ids: Vec<i64> = st
            .list()
            .expect("读取必须成功")
            .into_iter()
            .map(|it| it.id)
            .collect();
        assert_eq!(ids, vec![c.id, b.id, a.id], "新行落排头");
    }

    // ===== PL027 详情块（note_blocks：一段一行一块 + 图块混排 + 域内引用计数）TDD =====

    /// 造块辅助（id = 0 = 新块）
    fn nb_text(id: i64, text: &str) -> NoteBlock {
        NoteBlock {
            id,
            kind: NoteBlockKind::Text,
            text: text.to_string(),
            image_file: None,
        }
    }

    #[test]
    fn note_blocks_table_ready_and_empty_by_default() {
        let st = storage();
        let item = st.add("任务").expect("写入必须成功");
        assert!(st.note_blocks(item.id).expect("读块必须成功").is_empty());
        assert!(!item.has_note, "新条目无详情内容");
    }

    #[test]
    fn set_note_blocks_upserts_and_keeps_ids() {
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let saved = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "第一行"), nb_text(0, "第二行")])
            .expect("写块必须成功");
        assert_eq!(saved.len(), 2);
        assert!(saved[0].id > 0, "回填 id");
        // 差分 upsert：改首块 + 追加一块 → 既有块 id 稳定（前端不重挂失焦点）
        let first_id = saved[0].id;
        let updated = st
            .set_note_blocks(
                &dir,
                item.id,
                &[
                    nb_text(first_id, "改过的"),
                    nb_text(saved[1].id, "第二行"),
                    nb_text(0, "第三行"),
                ],
            )
            .expect("再写必须成功");
        assert_eq!(updated.len(), 3);
        assert_eq!(updated[0].id, first_id, "既有块 id 稳定");
        assert_eq!(updated[0].text, "改过的");
        assert_eq!(updated[2].text, "第三行");
    }

    #[test]
    fn paste_image_reuses_same_file_for_same_bytes() {
        // 引用计数核心断言：同一张图两次插入 = 一份文件、两个块引用（不存两份）
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "文字")])
            .expect("写块必须成功");
        let after = st
            .add_note_image_block(
                &dir,
                item.id,
                base[0].id,
                "文",
                "字",
                &png_bytes(),
                None,
                "261018000000",
            )
            .expect("插图必须成功");
        assert_eq!(after.len(), 3, "块变 3 个");
        assert_eq!(after[1].kind, NoteBlockKind::Image);
        let rel = after[1].image_file.clone().expect("图块必有文件");
        // 第二次插入同字节（插到另一块）
        let again = st
            .add_note_image_block(
                &dir,
                item.id,
                after[0].id,
                "",
                "",
                &png_bytes(),
                None,
                "261018000001",
            )
            .expect("再插必须成功");
        let images: Vec<&NoteBlock> = again
            .iter()
            .filter(|b| b.kind == NoteBlockKind::Image)
            .collect();
        assert_eq!(images.len(), 2, "两个图块");
        assert_eq!(images[0].image_file.as_deref(), Some(rel.as_str()));
        assert_eq!(
            images[1].image_file.as_deref(),
            Some(rel.as_str()),
            "同内容复用同一文件（不存两份）"
        );
        let folders = std::fs::read_dir(&dir)
            .expect("目录可读")
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("note_"))
            .count();
        assert_eq!(folders, 1, "盘上只落一份文件");
    }

    #[test]
    fn paste_image_splits_block_into_three() {
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "前半后半")])
            .expect("写块必须成功");
        let after = st
            .add_note_image_block(
                &dir,
                item.id,
                base[0].id,
                "前半",
                "后半",
                &png_bytes(),
                None,
                "261018000000",
            )
            .expect("插图必须成功");
        let texts: Vec<&str> = after.iter().map(|b| b.text.as_str()).collect();
        assert_eq!(
            texts,
            vec!["前半", "", "后半"],
            "光标处切分 before/图/after"
        );
    }

    #[test]
    fn note_image_released_only_at_zero_reference() {
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "文字")])
            .expect("写块必须成功");
        let first = st
            .add_note_image_block(
                &dir,
                item.id,
                base[0].id,
                "文",
                "字",
                &png_bytes(),
                None,
                "261018000000",
            )
            .expect("插图必须成功");
        let first_img_id = first[1].id;
        let rel = first[1].image_file.clone().expect("有文件");
        let folder = dir.join(rel.rsplit_once('/').expect("含子目录").0);
        assert!(folder.is_dir(), "落盘文件夹在");
        // 第二个引用（同文件）
        let two = st
            .add_note_image_block(
                &dir,
                item.id,
                first[2].id,
                "",
                "",
                &png_bytes(),
                None,
                "261018000000",
            )
            .expect("再插必须成功");
        assert_eq!(
            two.iter()
                .filter(|b| b.kind == NoteBlockKind::Image)
                .count(),
            2
        );
        // 删一个图块：文件仍在（还有引用）
        st.remove_note_block(&dir, first_img_id)
            .expect("删块必须成功");
        assert!(folder.is_dir(), "仍有引用 → 文件保留");
        // 删最后一个：文件夹消失
        let last = st
            .note_blocks(item.id)
            .expect("读块必须成功")
            .into_iter()
            .find(|b| b.kind == NoteBlockKind::Image)
            .expect("还剩一个图块");
        st.remove_note_block(&dir, last.id)
            .expect("删最后一块必须成功");
        assert!(!folder.is_dir(), "引用归零 → 文件夹删除");
    }

    #[test]
    fn remove_todo_sweeps_orphan_note_images() {
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "文字")])
            .expect("写块必须成功");
        let with_img = st
            .add_note_image_block(
                &dir,
                item.id,
                base[0].id,
                "",
                "",
                &png_bytes(),
                None,
                "261018000000",
            )
            .expect("插图必须成功");
        let rel = with_img
            .iter()
            .find(|b| b.kind == NoteBlockKind::Image)
            .expect("应有图块")
            .image_file
            .clone()
            .expect("有文件");
        let folder = dir.join(rel.rsplit_once('/').expect("含子目录").0);
        st.remove(item.id).expect("删条目必须成功");
        assert_eq!(st.sweep_orphan_note_images(&dir).expect("清扫必须成功"), 1);
        assert!(!folder.is_dir(), "删条目后图片文件夹消失");
    }

    #[test]
    fn purge_expired_sweeps_note_images() {
        let (st, clock) = stepped_storage();
        let dir = temp_images_dir();
        let t0 = clock.load(Ordering::SeqCst);
        let item = st.add("老条目").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "文字")])
            .expect("写块必须成功");
        let with_img = st
            .add_note_image_block(
                &dir,
                item.id,
                base[0].id,
                "",
                "",
                &png_bytes(),
                None,
                "261018000000",
            )
            .expect("插图必须成功");
        let rel = with_img
            .iter()
            .find(|b| b.kind == NoteBlockKind::Image)
            .expect("应有图块")
            .image_file
            .clone()
            .expect("有文件");
        let folder = dir.join(rel.rsplit_once('/').expect("含子目录").0);
        st.toggle(item.id).expect("勾选必须成功");
        clock.store(t0 + crate::todo::ARCHIVE_RETENTION_MS + 1, Ordering::SeqCst);
        let cutoff = clock.load(Ordering::SeqCst) - crate::todo::ARCHIVE_RETENTION_MS;
        assert_eq!(st.purge_expired_done(cutoff).expect("回收必须成功"), 1);
        assert_eq!(st.sweep_orphan_note_images(&dir).expect("清扫必须成功"), 1);
        assert!(!folder.is_dir(), "归档回收后图片文件夹消失");
    }

    #[test]
    fn reorder_note_blocks_roundtrip_and_mismatch() {
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let saved = st
            .set_note_blocks(
                &dir,
                item.id,
                &[nb_text(0, "甲"), nb_text(0, "乙"), nb_text(0, "丙")],
            )
            .expect("写块必须成功");
        let ids: Vec<i64> = saved.iter().map(|b| b.id).collect();
        st.reorder_note_blocks(item.id, &[ids[2], ids[0], ids[1]])
            .expect("重排必须成功");
        let texts: Vec<String> = st
            .note_blocks(item.id)
            .expect("读块必须成功")
            .into_iter()
            .map(|b| b.text)
            .collect();
        assert_eq!(texts, vec!["丙", "甲", "乙"]);
        assert!(
            st.reorder_note_blocks(item.id, &[ids[0]]).is_err(),
            "部分集拒绝"
        );
        assert!(
            st.reorder_note_blocks(item.id, &[ids[0], ids[1], 999])
                .is_err(),
            "幽灵 id 拒绝"
        );
    }

    #[test]
    fn note_preview_generated_for_large_image() {
        // 预览阈值与气泡一致（宽 > 2000px 或 > 2MB）：2100×10 PNG → 生成 preview.png
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "文字")])
            .expect("写块必须成功");
        let big = {
            let img = image::RgbaImage::new(2100, 10);
            let mut out = Vec::new();
            image::DynamicImage::ImageRgba8(img)
                .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
                .expect("夹具编码必须成功");
            out
        };
        let after = st
            .add_note_image_block(
                &dir,
                item.id,
                base[0].id,
                "",
                "",
                &big,
                None,
                "261018000000",
            )
            .expect("插图必须成功");
        let rel = after
            .iter()
            .find(|b| b.kind == NoteBlockKind::Image)
            .expect("应有图块")
            .image_file
            .clone()
            .expect("有文件");
        let folder = dir.join(rel.rsplit_once('/').expect("含子目录").0);
        assert!(folder.join("preview.png").is_file(), "大图生成预览");
    }

    #[test]
    fn sweep_ignores_bubble_folders() {
        // 域隔离红线：清扫只认 note_ 前缀，气泡 bubble_* 文件夹一行不碰
        let st = storage();
        let dir = temp_images_dir();
        let bubble_dir = dir.join("bubble_261018000000_1");
        std::fs::create_dir_all(&bubble_dir).expect("造气泡夹必须成功");
        assert_eq!(st.sweep_orphan_note_images(&dir).expect("清扫必须成功"), 0);
        assert!(bubble_dir.is_dir(), "气泡文件夹不受详情清扫影响");
    }

    // ===== PATCH002 贴图建块规则与迁移归一化（TDD） =====

    #[test]
    fn paste_into_blank_block_replaces_it_and_tails_when_last() {
        // 空块贴图 → 图顶替；图成最后一块 → 末尾补一个空文本块
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "")])
            .expect("写块必须成功");
        let after = st
            .add_note_image_block(
                &dir,
                item.id,
                base[0].id,
                "",
                "",
                &png_bytes(),
                None,
                "261018000000",
            )
            .expect("插图必须成功");
        assert_eq!(after.len(), 2, "空块被图顶替 + 末尾补一个空块");
        assert_eq!(after[0].kind, NoteBlockKind::Image);
        assert_eq!(after[1].kind, NoteBlockKind::Text);
        assert_eq!(after[1].text, "");
    }

    #[test]
    fn paste_at_first_char_leaves_no_leading_block() {
        // 现象②：首字前贴图 → [图][原文]，不留前导空块
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "原文")])
            .expect("写块必须成功");
        let after = st
            .add_note_image_block(
                &dir,
                item.id,
                base[0].id,
                "",
                "原文",
                &png_bytes(),
                None,
                "261018000000",
            )
            .expect("插图必须成功");
        assert_eq!(after.len(), 2, "不留前导空块");
        assert_eq!(after[0].kind, NoteBlockKind::Image);
        assert_eq!(after[1].kind, NoteBlockKind::Text);
        assert_eq!(after[1].text, "原文");
    }

    #[test]
    fn paste_at_end_of_middle_block_leaves_no_trailing_block() {
        // 现象③：上块末字贴图、下面还有块 → [上文][图][下文]，不留尾随空块
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "上文"), nb_text(0, "下文")])
            .expect("写块必须成功");
        let after = st
            .add_note_image_block(
                &dir,
                item.id,
                base[0].id,
                "上文",
                "",
                &png_bytes(),
                None,
                "261018000000",
            )
            .expect("插图必须成功");
        let kinds: Vec<NoteBlockKind> = after.iter().map(|b| b.kind).collect();
        assert_eq!(
            kinds,
            vec![
                NoteBlockKind::Text,
                NoteBlockKind::Image,
                NoteBlockKind::Text
            ],
            "不留尾随空块"
        );
        assert_eq!(after[0].text, "上文");
        assert_eq!(after[2].text, "下文");
    }

    #[test]
    fn paste_at_very_end_tails_blank_block() {
        // 图成整条最后一块 → 末尾补一个空文本块（否则图后无法输入）
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "原文")])
            .expect("写块必须成功");
        let after = st
            .add_note_image_block(
                &dir,
                item.id,
                base[0].id,
                "原文",
                "",
                &png_bytes(),
                None,
                "261018000000",
            )
            .expect("插图必须成功");
        assert_eq!(after.len(), 3, "[原文][图][空块]");
        assert_eq!(after[1].kind, NoteBlockKind::Image);
        assert_eq!(after[2].kind, NoteBlockKind::Text);
        assert_eq!(after[2].text, "");
    }

    #[test]
    fn migration_normalizes_blank_lines() {
        // 迁移归一化：存量「前导空行 + 连续空行」→ 只留正文块
        let st = pl010_storage_with_notes();
        st.init().expect("迁移必须成功");
        let items = st.list().expect("读取必须成功");
        let id = items
            .iter()
            .find(|i| i.text == "空行归一")
            .map(|i| i.id)
            .expect("存量行必须在");
        let blocks = st.note_blocks(id).expect("读块必须成功");
        assert_eq!(blocks.len(), 1, "前导/连续空行被归一化");
        assert_eq!(blocks[0].text, "正文");
    }
}
