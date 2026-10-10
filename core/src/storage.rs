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
use crate::todo::TodoItem;

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

/// 同 stamp 已存在的气泡图片文件夹最大序号（PL025；无则 0）
fn max_bubble_seq(images_dir: &Path, stamp: &str) -> u32 {
    let prefix = format!("bubble_{stamp}_");
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

/// 下一个气泡图片文件夹名 `bubble_{stamp}_{n}`（PL025）：n = 同 stamp 最大序号 + 1
fn next_bubble_folder(images_dir: &Path, stamp: &str) -> String {
    format!("bubble_{stamp}_{}", max_bubble_seq(images_dir, stamp) + 1)
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
        self.migrate_bubble_kind()?;
        self.migrate_bubble_image_file()?;
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
        let base = max_bubble_seq(images_dir, stamp) + 1;
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

    /// 新增待办（文本须先经 todo::validate_text 业务校验，且调用方须传 **trim 后**
    /// 文本——trim 执行点在命令层，直调本方法绕过命令层即失守此契约，FIX009.7
    /// 对齐 add_bubble 同款声明），返回含回填 id 的条目；
    /// created_at = 时间源 now（新建行非 NULL）；sort_order = 未完成段最小值 − 1
    /// （排头插入——2026-09-30 用户定案：新增置顶，与气泡排头插入语义对齐；
    /// 限定 done = 0 段取 MIN，不侵完成序）
    pub fn add(&self, text: &str) -> Result<TodoItem, StorageError> {
        let now = (self.now)();
        self.conn.execute(
            "INSERT INTO todos(text, done, created_at, note, sort_order)
             VALUES (?1, 0, ?2, '', (
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
            note: String::new(),
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

    /// 行映射（get/list 共用；NULL 时刻映射 Option::None；has_note 由此单点派生，
    /// toggle/rename 经 get(id) 自动覆盖——PL023）
    fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<TodoItem> {
        let note: String = row.get(5)?;
        Ok(TodoItem {
            id: row.get(0)?,
            text: row.get(1)?,
            done: row.get::<_, i64>(2)? != 0,
            created_at: row.get(3)?,
            done_at: row.get(4)?,
            has_note: !note.trim().is_empty(),
            note,
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
    /// 非 NULL 同值按 id 次级排序——SQLite ASC 中 NULL 恒排最前，迁移满射回填后无
    /// NULL，此处为防御，FIX011.3 措辞精确化）
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

    /// 归档回收（PATCH001）：删除已完成且 done_at 早于 cutoff 的条目，返回删除行数。
    /// `done_at IS NOT NULL` 谓词 = 迁移遗留 NULL 行永不回收（PL010"不模拟历史时间"
    /// 先例）；未完成条目（done = 0）不在谓词内。cutoff 由调用方按业务常量
    /// todo::ARCHIVE_RETENTION_MS 计算——存储层不持保留期策略。
    pub fn purge_expired_done(&self, cutoff: i64) -> Result<usize, StorageError> {
        let changed = self.conn.execute(
            "DELETE FROM todos
             WHERE done = 1 AND done_at IS NOT NULL AND done_at < ?1",
            rusqlite::params![cutoff],
        )?;
        Ok(changed)
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
        let folder = next_bubble_folder(images_dir, stamp);
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

    #[test]
    fn has_note_derives_from_note_trim() {
        // PL023.1："有笔记"裁决字段（trim 判空单源）——非空白亮、纯空白与空串不亮
        //（validate_note 允许空白入库，红点语义只认 trim 后非空，PL023 定案）
        let st = storage();
        let item = st.add("任务").expect("写入必须成功");
        assert!(!st.get(item.id).expect("读取必须成功").has_note);
        st.set_note(item.id, "有内容").expect("写笔记必须成功");
        assert!(st.get(item.id).expect("读取必须成功").has_note);
        st.set_note(item.id, "   ").expect("写纯空白必须成功");
        assert!(!st.get(item.id).expect("读取必须成功").has_note);
        st.set_note(item.id, "").expect("清空笔记必须成功");
        assert!(!st.get(item.id).expect("读取必须成功").has_note);
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
}
