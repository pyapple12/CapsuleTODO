//! SQLite 存储层：**四表** `todos` / `bubbles` / `whiteboard` / `note_blocks`
//! 增删改查（全参数化绑定，禁 SQL 拼接），含气泡去重裁决（add_bubble 重复文本返回
//! Duplicate 不入库，PL015.5）。
//! 单一事实源 = db（PL002 定案）：操作即落库，list 排序 = 未完成在前按 sort_order
//! 升序（拖拽序），id 兜底。
//! 迁移链：todos 补齐 created_at/done_at/sort_order（`todos.note` 已由
//! `note_blocks` 块模型取代并退役）；bubbles 补齐 sort_order/kind/image_file/
//! image_hash/image_source；`note_blocks` 补齐 source_url/deleted_at 与查询索引。
//! 时间源注入（NowFn 默认系统时钟，测试注入固定值零真实等待）。

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension};
use thiserror::Error;

use crate::bubble::BubbleItem;
use crate::todo::{NoteBlock, NoteBlockKind, TodoItem};

/// 远端图下载占位文案（PATCH002.76）：与前端 `DetailOverlay.vue` 的
/// `PENDING_IMAGE_TEXT` 同源——**改一处必须同步**。占位块是文本块，原 URL 存
/// `note_blocks.source_url`；成功后原位升格图块，失败/启动清扫时换成 URL 文本。
pub const PENDING_NOTE_IMAGE_TEXT: &str = "🖼 下载中…";

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
    /// 图片相对路径非法（空串 / 目录穿越 / 绝对路径——禁止据此删除文件）
    #[error("图片相对路径非法：{0}")]
    InvalidImagePath(String),
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

/// FIX014.16：原图内层名与固定预览名 `preview.png` 冲突时改用 `original.png`。
/// 预览文件仍固定写 `preview.png`，`image_file` 始终指向真正的原图。
fn image_inner_name(orig_name: Option<&str>, stamp: &str, seq: &str) -> String {
    let name = match orig_name {
        Some(name) => sanitize_file_name(name),
        None => format!("snap_{stamp}_{seq}.png"),
    };
    if name.eq_ignore_ascii_case("preview.png") {
        "original.png".to_string()
    } else {
        name
    }
}

/// FIX014.15：图片落盘失败补偿——只删本次新建的文件夹，best-effort 不覆盖主错误。
fn cleanup_image_folder(dir: &Path) {
    if !dir.exists() {
        return;
    }
    if let Err(err) = std::fs::remove_dir_all(dir) {
        crate::diag::warn_diag(&format!("图片失败补偿删除目录失败：{err}"));
    }
}

/// FIX014.17：文件夹名只允许单个正常路径段（禁止空串、`.`、`..`、分隔符、盘符）。
pub(crate) fn is_safe_image_folder(name: &str, prefix: &str) -> bool {
    !name.is_empty()
        && name.starts_with(prefix)
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains(':')
}

/// FIX014.17：详情 `image_file` 必须是安全的 `note_* / 文件名` 两段路径。
fn safe_note_folder(rel: &str) -> Option<&str> {
    let (folder, file) = rel.split_once('/')?;
    if file.is_empty()
        || file == "."
        || file == ".."
        || file.contains('/')
        || file.contains('\\')
        || file.contains(':')
    {
        return None;
    }
    is_safe_image_folder(folder, "note_").then_some(folder)
}

/// 多段粘贴的一段（PATCH002.25）：文本段或图段（图段自带字节与来源名，落盘由存储层负责）
#[derive(Debug)]
pub enum NotePastePart {
    /// 文本段（**不得含换行**——块不变量；调用方按段落切好）
    Text(String),
    /// 图段（字节 + 可选原名；去重 / 命名 / 预览沿单图插入同款）
    Image {
        /// 图片字节（调用方须先 `validate_image_bytes`）
        bytes: Vec<u8>,
        /// 来源文件名（净化后作内层名；None = 截图式 `snap_…`）
        orig_name: Option<String>,
    },
    /// 远端图占位段（PATCH002.76）：先落 `PENDING_NOTE_IMAGE_TEXT` 文本并记
    /// `source_url`；后台成功原位升格图块，失败/启动清扫换成 URL 文本。
    /// **末块/至少一个文本块不变量按图段对待**（升级后不能再占文本块名额）。
    PendingImage {
        /// 规范化后的 http(s) URL（`image_fetch::normalize_url` 产物）
        url: String,
    },
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
        // PATCH002.66：写库快速失败（对齐 CapsulePulse PATCH005.14）——默认 5s 忙等
        // 会把一次点按拖成数秒；CT 现为单连接 + Mutex + 单实例，正常路径 app 是唯一写者，
        // 归零不误伤；外部第二写者/锁冲突立即 `SQLITE_BUSY`，UI 立即出错误行。
        conn.busy_timeout(std::time::Duration::from_millis(0))?;
        let storage = Self { conn, now };
        storage.init()?;
        // PATCH002.76：上次进程退出时仍在“下载中…”的占位块 → 启动时降级为 URL
        // （不做续传；失败态与占位同源，用户可见且可编辑）
        match storage.degrade_pending_note_images() {
            Ok(0) => {}
            Ok(count) => crate::diag::warn_diag(&format!(
                "启动清理未完成图片下载占位：{count} 块已降级为 URL"
            )),
            Err(err) => crate::diag::warn_diag(&format!("启动清理图片占位失败：{err}")),
        }
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

    /// 建表（幂等：CREATE TABLE IF NOT EXISTS）+ 迁移：
    /// - `todos`：补齐 created_at/done_at/sort_order（note 列已退役）；
    /// - `bubbles`：补齐 sort_order/kind/image_file/image_hash/image_source；
    /// - `note_blocks`：补齐 source_url/deleted_at 与 todo/order、image_file 索引。
    ///
    /// 新库与旧库都汇合到同一套 `migrate*` 函数。
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
                image_file TEXT,
                source_url TEXT,
                deleted_at INTEGER
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
        self.migrate_note_source_url()?;
        self.migrate_note_soft_delete()?;
        self.migrate_note_indexes()?;
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
    /// 直接跳过。须在 `drop_note_column` 之前调用。
    /// **FIX014.4**：整段迁移包事务——中途失败全部回滚，不留下部分迁移状态。
    pub fn migrate_note_to_blocks(&self) -> Result<(), StorageError> {
        if !self.column_set("todos")?.contains("note") {
            return Ok(());
        }
        let rows: Vec<(i64, String)> = self
            .conn
            .prepare("SELECT id, note FROM todos")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), StorageError> {
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
        })();
        match result {
            Ok(()) => {
                if let Err(err) = self.conn.execute_batch("COMMIT") {
                    let _ = self.conn.execute_batch("ROLLBACK");
                    return Err(err.into());
                }
                Ok(())
            }
            Err(err) => {
                if let Err(rollback) = self.conn.execute_batch("ROLLBACK") {
                    crate::diag::warn_diag(&format!("详情笔记迁移回滚失败：{rollback}"));
                }
                Err(err)
            }
        }
    }

    /// PATCH002.76：`note_blocks` 补 `source_url` 列（幂等）。远端图占位/成功后存原
    /// URL；启动清扫与写序守卫都读它。新库建表已带，旧库在此补列。
    fn migrate_note_source_url(&self) -> Result<(), StorageError> {
        if !self.column_set("note_blocks")?.contains("source_url") {
            self.conn
                .execute("ALTER TABLE note_blocks ADD COLUMN source_url TEXT", [])?;
        }
        Ok(())
    }

    /// FIX014.2：`note_blocks` 补 `deleted_at` 列（幂等）——图块删除在关板前只软删除，
    /// 保留行与图片文件供 Ctrl+Z；关板 finalize / 启动时再硬删（purge_deleted_note_blocks）。
    fn migrate_note_soft_delete(&self) -> Result<(), StorageError> {
        if !self.column_set("note_blocks")?.contains("deleted_at") {
            self.conn
                .execute("ALTER TABLE note_blocks ADD COLUMN deleted_at INTEGER", [])?;
        }
        Ok(())
    }

    /// FIX014.13：`note_blocks` 查询索引（幂等）——列表按 todo/order、删图与清扫按
    /// image_file。小数据量无感，块/图片增长后避免反复全表扫描。
    fn migrate_note_indexes(&self) -> Result<(), StorageError> {
        self.conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_note_blocks_todo_order
                 ON note_blocks(todo_id, sort_order);
             CREATE INDEX IF NOT EXISTS idx_note_blocks_image_file
                 ON note_blocks(image_file);",
        )?;
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
                AND nb.deleted_at IS NULL
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
        // FIX014.11：块删除与条目删除必须同事务——第二步失败时不得留下“块没了、条目还在”。
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), StorageError> {
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
        })();
        match result {
            Ok(()) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(())
            }
            Err(err) => {
                if let Err(rollback) = self.conn.execute_batch("ROLLBACK") {
                    crate::diag::warn_diag(&format!("删除条目事务回滚失败：{rollback}"));
                }
                Err(err)
            }
        }
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
        // 其图片文件由调用方经 sweep_orphan_note_images 释放（此时引用已归零）。
        // FIX014.11：两次 DELETE 必须同事务，失败不得留下“块删了、条目还在”的半态。
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<usize, StorageError> {
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
        })();
        match result {
            Ok(changed) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(changed)
            }
            Err(err) => {
                if let Err(rollback) = self.conn.execute_batch("ROLLBACK") {
                    crate::diag::warn_diag(&format!("归档回收事务回滚失败：{rollback}"));
                }
                Err(err)
            }
        }
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

    /// 读该条目的详情块（sort_order 升序 = 展示序，id 兜底）；软删除行不返回
    pub fn note_blocks(&self, todo_id: i64) -> Result<Vec<NoteBlock>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, text, image_file FROM note_blocks
             WHERE todo_id = ?1 AND deleted_at IS NULL
             ORDER BY sort_order ASC, id ASC",
        )?;
        let rows = stmt.query_map([todo_id], Self::row_to_note_block)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 读单个块；不存在/已软删除返回 NotFound（开原图取数源，PL027）
    pub fn get_note_block(&self, id: i64) -> Result<NoteBlock, StorageError> {
        self.conn
            .query_row(
                "SELECT id, kind, text, image_file FROM note_blocks
                 WHERE id = ?1 AND deleted_at IS NULL",
                [id],
                Self::row_to_note_block,
            )
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound(id),
                other => StorageError::Sqlite(other),
            })
    }

    /// 该条目全部图块的图片相对路径（释放/迁移用）。**包含软删除行**：关板前
    /// 被删图块仍占引用，防 sweep 提前删文件；真正的释放由 purge_deleted_note_blocks 收口。
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
        // FIX014.17：DB 路径异常时绝不 join/remove——空串会指向 images 根，
        // `../` 会跳出 images 目录。校验失败保留文件、向上报可读错误。
        let Some(folder) = safe_note_folder(rel) else {
            return Err(StorageError::InvalidImagePath(rel.to_string()));
        };
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM note_blocks WHERE image_file = ?1",
            [rel],
            |row| row.get(0),
        )?;
        if count > 0 {
            return Ok(());
        }
        let dir = images_dir.join(folder);
        if let Err(err) = std::fs::remove_dir_all(&dir) {
            crate::diag::warn_diag(&format!(
                "详情图文件夹删除失败（best-effort 不阻断）：{err}"
            ));
        }
        Ok(())
    }

    /// FIX014.2：关板/启动收口——硬删软删除的图块行并释放其图片。
    /// `todo_id = None` 用于启动清扫历史残留；删除前收集路径，删除后按**剩余 active
    /// 行引用数**释放文件（同图多块复用不会被误删）。行删除失败回滚，文件释放 best-effort。
    pub fn purge_deleted_note_blocks(
        &self,
        images_dir: &Path,
        todo_id: Option<i64>,
    ) -> Result<usize, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT image_file FROM note_blocks
             WHERE deleted_at IS NOT NULL
               AND (?1 IS NULL OR todo_id = ?1)
               AND image_file IS NOT NULL",
        )?;
        let paths: Vec<String> = stmt
            .query_map([todo_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        self.conn.execute_batch("BEGIN")?;
        let result = self.conn.execute(
            "DELETE FROM note_blocks
             WHERE deleted_at IS NOT NULL AND (?1 IS NULL OR todo_id = ?1)",
            [todo_id],
        );
        let deleted = match result {
            Ok(count) => {
                self.conn.execute_batch("COMMIT")?;
                count
            }
            Err(err) => {
                if let Err(rollback) = self.conn.execute_batch("ROLLBACK") {
                    crate::diag::warn_diag(&format!("软删除块清理回滚失败：{rollback}"));
                }
                return Err(StorageError::Sqlite(err));
            }
        };
        for rel in paths {
            if let Err(err) = self.release_note_image(images_dir, &rel) {
                crate::diag::warn_diag(&format!("软删除图片释放失败（best-effort）：{err}"));
            }
        }
        Ok(deleted)
    }

    /// 孤儿清扫式引用释放（PL027；供删条目 / 归档回收后调用）：删除 `images/` 下所有
    /// **已无块引用**的 `note_*` 文件夹（与精确释放等价，且顺带清理崩溃遗留孤儿文件）；
    /// **只扫 `note_` 前缀**（气泡 `bubble_*` 域一行不碰）。返回清理的文件夹数
    pub fn sweep_orphan_note_images(&self, images_dir: &Path) -> Result<usize, StorageError> {
        let entries = match std::fs::read_dir(images_dir) {
            Ok(e) => e,
            Err(err) => {
                crate::diag::warn_diag(&format!("详情图清扫跳过（目录不可读）：{err}"));
                return Ok(0);
            }
        };
        // FIX014.13：一次 DISTINCT 查询构造已引用文件夹集合，替代每个文件夹一次 COUNT/LIKE。
        let referenced: HashSet<String> = self
            .conn
            .prepare("SELECT DISTINCT image_file FROM note_blocks WHERE image_file IS NOT NULL")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|rel| folder_of(&rel))
            .collect();
        let mut removed = 0usize;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if !name.starts_with("note_") || !entry.path().is_dir() {
                continue;
            }
            if referenced.contains(name) {
                continue;
            }
            if let Err(err) = std::fs::remove_dir_all(entry.path()) {
                crate::diag::warn_diag(&format!(
                    "详情图孤儿文件夹删除失败（best-effort 不阻断）：{err}"
                ));
                continue;
            }
            removed += 1;
        }
        Ok(removed)
    }

    /// 目标块必须存在且属于该条目（PATCH002.46）：块 id 有三条路径会指向"库里不存在的
    /// 块"（空笔记的幻影块 id=0 / 跨条目陈旧 id / 前端未回填的新块），此前它们都在
    /// **落盘之后**才失败 → 留下孤儿夹（PATCH002.41）。校验前移到任何文件写入之前
    fn require_note_block(&self, todo_id: i64, block_id: i64) -> Result<(), StorageError> {
        let exists: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM note_blocks
                 WHERE id = ?1 AND todo_id = ?2 AND deleted_at IS NULL",
                rusqlite::params![block_id, todo_id],
                |row| row.get(0),
            )
            .optional()?;
        exists.ok_or(StorageError::NotFound(block_id))?;
        Ok(())
    }

    /// 落盘后失败 → 补偿清扫（PATCH002.46）：把刚落下的图收回去，避免"粘贴失败却留下
    /// 孤儿夹"（清扫只碰 `note_*` 且只删零引用夹；best-effort 不阻断错误向上传播）
    fn sweep_after_failed_write(&self, images_dir: &Path) {
        if let Err(err) = self.sweep_orphan_note_images(images_dir) {
            crate::diag::warn_diag(&format!(
                "粘贴失败后详情图清扫失败（best-effort 不阻断）：{err}"
            ));
        }
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
        // FIX014.12：正 id 重复会让同一行被多次更新，最终块数与内容可能静默丢失；
        // id=0 是新块占位，允许重复，不参与去重。
        let mut seen_ids = HashSet::new();
        for block in blocks {
            if block.id > 0 && !seen_ids.insert(block.id) {
                return Err(StorageError::ReorderMismatch(format!(
                    "详情块 id 重复：{}",
                    block.id
                )));
            }
        }
        let old_paths = self.note_image_paths(todo_id)?;
        let keep_ids: Vec<i64> = blocks.iter().map(|b| b.id).filter(|id| *id > 0).collect();
        let deleted_at = (self.now)();
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), StorageError> {
            // FIX014.12：迟到保存可能携带已删除条目 id；note_blocks 无外键，必须在此
            // 显式拒绝，否则会写入永远不可见的孤儿块。
            let todo_exists: Option<i64> = self
                .conn
                .query_row("SELECT id FROM todos WHERE id = ?1", [todo_id], |row| {
                    row.get(0)
                })
                .optional()?;
            if todo_exists.is_none() {
                return Err(StorageError::NotFound(todo_id));
            }
            let existing: Vec<(i64, String)> = self
                .conn
                .prepare("SELECT id, kind FROM note_blocks WHERE todo_id = ?1")?
                .query_map([todo_id], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            for (id, kind) in existing {
                if keep_ids.contains(&id) {
                    continue;
                }
                if kind == "image" {
                    // FIX014.2：图块删除先软删除，文件/行保留到关板 finalize；Ctrl+Z 可复活
                    self.conn.execute(
                        "UPDATE note_blocks SET deleted_at = COALESCE(deleted_at, ?2)
                         WHERE id = ?1",
                        rusqlite::params![id, deleted_at],
                    )?;
                } else {
                    self.conn
                        .execute("DELETE FROM note_blocks WHERE id = ?1", [id])?;
                }
            }
            for (i, block) in blocks.iter().enumerate() {
                match block.kind {
                    NoteBlockKind::Text => {
                        let text = block.text.as_str();
                        if block.id > 0 {
                            // PATCH002.76 写序守卫：后台图已升级 / 失败降级后，切源或收场的
                            // 迟到保存可能仍带“下载中…”占位或原 URL —— 识别为旧态，只改序，
                            // 不把图块/降级文本写回旧占位。用户真正编辑（文本既非占位也非
                            // source_url）仍正常降级，不挡撤销。
                            let current: Option<(String, String, Option<String>)> = self
                                .conn
                                .query_row(
                                    "SELECT kind, text, source_url FROM note_blocks
                                     WHERE id = ?1 AND todo_id = ?2",
                                    rusqlite::params![block.id, todo_id],
                                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                                )
                                .optional()?;
                            if let Some((kind, cur_text, Some(source))) = current {
                                let stale = match kind.as_str() {
                                    "image" => {
                                        text == PENDING_NOTE_IMAGE_TEXT || text == source.as_str()
                                    }
                                    "text" if cur_text == PENDING_NOTE_IMAGE_TEXT => {
                                        text == PENDING_NOTE_IMAGE_TEXT
                                    }
                                    "text" if cur_text == source.as_str() => {
                                        text == PENDING_NOTE_IMAGE_TEXT || text == source.as_str()
                                    }
                                    _ => false,
                                };
                                if stale {
                                    self.conn.execute(
                                        "UPDATE note_blocks SET sort_order = ?2
                                         WHERE id = ?1 AND todo_id = ?3",
                                        rusqlite::params![block.id, i as i64, todo_id],
                                    )?;
                                    continue;
                                }
                            }
                            let changed = self.conn.execute(
                                "UPDATE note_blocks SET sort_order = ?2, kind = 'text', text = ?3,
                                     image_hash = NULL, image_file = NULL, source_url = NULL,
                                     deleted_at = NULL
                                 WHERE id = ?1 AND todo_id = ?4",
                                rusqlite::params![block.id, i as i64, text, todo_id],
                            )?;
                            if changed == 0 {
                                // PATCH002.72 C7：**旧行可能已被删除** —— "粘贴替换选区"落库时会把
                                // 被快照引用的行删掉，随后 Ctrl+Z 要把它们写回 ⇒ 原 `NotFound`
                                // 会让**整次保存**失败（实证：`pre-save` 有、`save` 无、重开是粘贴版 ✗）
                                // ⇒ 改为**按新块插入**（内容保真；id 换新，无外部引用）
                                self.conn.execute(
                                    "INSERT INTO note_blocks(todo_id, sort_order, kind, text)
                                     VALUES (?1, ?2, 'text', ?3)",
                                    rusqlite::params![todo_id, i as i64, text],
                                )?;
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
                            "UPDATE note_blocks SET sort_order = ?2, kind = 'image',
                                 deleted_at = NULL
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
                    crate::diag::warn_diag(&format!("详情块写入事务回滚失败：{rollback}"));
                }
                return Err(err);
            }
        }
        // PATCH002.56 口径改定（2026-10-19）：storage **不自行补块** —— 末块不变量由**前端在保存点**
        // 唯一负责（两边都管会导致"补 → 下次保存被差分删"的来回抖动）
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
        // ⓪ 目标块必须先存在（PATCH002.46，同 `insert_note_parts`）——校验先于任何落盘
        self.require_note_block(todo_id, block_id)?;
        // ① 落盘或复用（与多段粘贴共用同一份 `write_note_image`：同 hash 复用同一文件、
        // 命中不写盘；此处原内联实现逐字等价，抽出即免两份维护）
        let (hash, rel) = match self.write_note_image(images_dir, bytes, orig_name, stamp) {
            Ok(pair) => pair,
            Err(err) => {
                self.sweep_after_failed_write(images_dir); // 写盘半途失败：补偿清扫
                return Err(err);
            }
        };
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), StorageError> {
            let order: Option<i64> = self
                .conn
                .query_row(
                    "SELECT sort_order FROM note_blocks
                     WHERE id = ?1 AND todo_id = ?2 AND deleted_at IS NULL",
                    rusqlite::params![block_id, todo_id],
                    |row| row.get(0),
                )
                .optional()?;
            let order = order.ok_or(StorageError::NotFound(block_id))?;
            let follows: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM note_blocks
                 WHERE todo_id = ?1 AND sort_order > ?2 AND deleted_at IS NULL",
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
                 WHERE todo_id = ?1 AND sort_order > ?2 AND deleted_at IS NULL",
                rusqlite::params![todo_id, order, shift],
            )?;
            let mut next = order + 1;
            if keep_before {
                // 原块保留为「上段」文本块，图块插在其后
                self.conn.execute(
                    "UPDATE note_blocks SET text = ?2, kind = 'text',
                         image_hash = NULL, image_file = NULL, source_url = NULL,
                         deleted_at = NULL
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
                         image_hash = ?2, image_file = ?3, source_url = NULL,
                         deleted_at = NULL
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
                "SELECT COUNT(*) FROM note_blocks
                 WHERE todo_id = ?1 AND kind = 'text' AND deleted_at IS NULL",
                [todo_id],
                |row| row.get(0),
            )?;
            if texts == 0 {
                let last: i64 = self.conn.query_row(
                    "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM note_blocks
                     WHERE todo_id = ?1 AND deleted_at IS NULL",
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
                    crate::diag::warn_diag(&format!("插图事务回滚失败：{rollback}"));
                }
                self.sweep_after_failed_write(images_dir); // PATCH002.46 补偿：刚落下的图不留孤儿
                return Err(err);
            }
        }
        self.note_blocks(todo_id)
    }

    /// 落盘一张详情图（去重 + 命名 + 预览），返回 `(hash, rel)`；**不含块表操作**——
    /// 单图插入与多段粘贴共用（PATCH002.25 抽取，语义与原先内联实现逐字一致）
    fn write_note_image(
        &self,
        images_dir: &Path,
        bytes: &[u8],
        orig_name: Option<&str>,
        stamp: &str,
    ) -> Result<(String, String), StorageError> {
        let preview = crate::clipboard_image::generate_preview(bytes);
        self.write_note_image_prepared(images_dir, bytes, orig_name, stamp, preview.as_deref())
    }

    /// 落盘一张详情图（PATCH002.76 变体）：预览由调用方**在 storage 锁外**预生成，
    /// 锁内只做去重查询 + 写原图/预览，避免后台大图解码堵住 300ms 自动保存。
    fn write_note_image_prepared(
        &self,
        images_dir: &Path,
        bytes: &[u8],
        orig_name: Option<&str>,
        stamp: &str,
        preview: Option<&[u8]>,
    ) -> Result<(String, String), StorageError> {
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
        if let Some(rel) = existing {
            return Ok((hash, rel));
        }
        let folder = next_seq_folder(images_dir, "note", stamp);
        let seq = folder.rsplit('_').next().unwrap_or("1");
        let inner = image_inner_name(orig_name, stamp, seq);
        let dir = images_dir.join(&folder);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(&inner), bytes)?;
        if let Some(preview) = preview {
            if let Err(err) = std::fs::write(dir.join("preview.png"), preview) {
                crate::diag::warn_diag(&format!("详情图预览写入失败（降级用原图）：{err}"));
            }
        }
        Ok((hash, format!("{folder}/{inner}")))
    }

    /// 多段粘贴（PATCH002.25；PATCH002.76 扩展远端占位）：把 `block_id` 的文本按
    /// `before` / `after` 切开，中间按序插入「文本段 / 图段 / 远端图占位段」（保持
    /// sort_order 连续）→ 返回（新块列表, **与 parts 同序的块 id**）。
    /// 建块规则沿单图插入同款：上段为空 → 原块**顶替**为序列首段；下段为空 → 不留尾块，
    /// 但序列末段是图（含远端占位）且其后无块时补一个空文本块；保底 = 结果无文本块则补一个。
    /// **远端占位按“升级后”口径算文本块**：`texts - pending_count <= 0` 时补空文本块，
    /// 否则远程图全部成功后整条会变成零文本块。调用方须先校验各图字节与最终块集；
    /// `parts` 文本段不得含换行
    #[allow(clippy::too_many_arguments)] // 与 add_note_image_block 同款：粘贴输入集完整传递
    pub fn insert_note_parts(
        &self,
        images_dir: &Path,
        todo_id: i64,
        block_id: i64,
        before: &str,
        after: &str,
        parts: &[NotePastePart],
        stamp: &str,
    ) -> Result<(Vec<NoteBlock>, Vec<i64>), StorageError> {
        // ⓪ 目标块必须先存在（PATCH002.46）：幻影块 id=0 / 跨条目陈旧 id 此前都在**落盘
        // 之后**才失败 → 留下孤儿夹（PATCH002.41）；校验前移到任何文件写入之前
        self.require_note_block(todo_id, block_id)?;
        // ① 先落盘全部图段（事务外：文件 IO 不该拖住事务；同 hash 去重保证幂等）。
        // FIX014.14：本批第一张图尚未入 DB，后续同图查库查不到——必须用批内
        // hash→rel 表复用，否则同一段 HTML 里的重复图片会落两份文件。
        let mut images: Vec<Option<(String, String)>> = Vec::with_capacity(parts.len());
        let mut batch_images: HashMap<String, (String, String)> = HashMap::new();
        let written = (|| -> Result<(), StorageError> {
            for part in parts {
                match part {
                    NotePastePart::Image { bytes, orig_name } => {
                        let hash = fnv1a_hex(bytes);
                        let pair = match batch_images.get(&hash) {
                            Some(pair) => pair.clone(),
                            None => {
                                let pair = self.write_note_image(
                                    images_dir,
                                    bytes,
                                    orig_name.as_deref(),
                                    stamp,
                                )?;
                                batch_images.insert(hash, pair.clone());
                                pair
                            }
                        };
                        images.push(Some(pair));
                    }
                    NotePastePart::Text(_) | NotePastePart::PendingImage { .. } => {
                        images.push(None)
                    }
                }
            }
            Ok(())
        })();
        if let Err(err) = written {
            self.sweep_after_failed_write(images_dir); // 写盘半途失败：补偿清扫（不留半份）
            return Err(err);
        }
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<Vec<i64>, StorageError> {
            let order: Option<i64> = self
                .conn
                .query_row(
                    "SELECT sort_order FROM note_blocks
                     WHERE id = ?1 AND todo_id = ?2 AND deleted_at IS NULL",
                    rusqlite::params![block_id, todo_id],
                    |row| row.get(0),
                )
                .optional()?;
            let order = order.ok_or(StorageError::NotFound(block_id))?;
            let follows: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM note_blocks
                 WHERE todo_id = ?1 AND sort_order > ?2 AND deleted_at IS NULL",
                rusqlite::params![todo_id, order],
                |row| row.get(0),
            )?;
            let keep_before = !before.trim().is_empty();
            let keep_after = !after.trim().is_empty();
            // 远端占位在“升级后”会变图块 → 末块/文本块兜底按图段口径算
            let last_is_image = matches!(
                parts.last(),
                Some(NotePastePart::Image { .. } | NotePastePart::PendingImage { .. })
            );
            let pending_count = parts
                .iter()
                .filter(|part| matches!(part, NotePastePart::PendingImage { .. }))
                .count() as i64;
            let tail_blank = !keep_after && follows == 0 && last_is_image;
            let shift = i64::from(keep_before)
                + i64::from(keep_after)
                + i64::from(tail_blank)
                + parts.len() as i64;
            self.conn.execute(
                "UPDATE note_blocks SET sort_order = sort_order + ?3
                 WHERE todo_id = ?1 AND sort_order > ?2 AND deleted_at IS NULL",
                rusqlite::params![todo_id, order, shift],
            )?;
            let mut next = order + 1;
            // PATCH002.76：与 parts 同序的块 id——后台任务按 id 精准定位占位块，
            // 不靠 URL 文本反查（重复 URL 会歧义）
            let mut part_ids: Vec<i64> = Vec::with_capacity(parts.len());
            if keep_before {
                self.conn.execute(
                    "UPDATE note_blocks SET text = ?2, kind = 'text',
                         image_hash = NULL, image_file = NULL, source_url = NULL,
                         deleted_at = NULL
                     WHERE id = ?1",
                    rusqlite::params![block_id, before],
                )?;
            }
            for (i, part) in parts.iter().enumerate() {
                // 上段为空时原块顶替为序列首段（不留前导空块）——文本 / 图 / 远端占位三态同款
                let reuse_original = !keep_before && i == 0;
                match part {
                    NotePastePart::Text(text) => {
                        if reuse_original {
                            self.conn.execute(
                                "UPDATE note_blocks SET kind = 'text', text = ?2,
                                     image_hash = NULL, image_file = NULL, source_url = NULL
                                 WHERE id = ?1",
                                rusqlite::params![block_id, text],
                            )?;
                            part_ids.push(block_id);
                            continue;
                        }
                        self.conn.execute(
                            "INSERT INTO note_blocks(todo_id, sort_order, kind, text)
                             VALUES (?1, ?2, 'text', ?3)",
                            rusqlite::params![todo_id, next, text],
                        )?;
                        part_ids.push(self.conn.last_insert_rowid());
                        next += 1;
                    }
                    NotePastePart::Image { .. } => {
                        let (hash, rel) = images[i].clone().unwrap_or_default();
                        if reuse_original {
                            self.conn.execute(
                                "UPDATE note_blocks SET kind = 'image', text = '',
                                     image_hash = ?2, image_file = ?3, source_url = NULL,
                                     deleted_at = NULL
                                 WHERE id = ?1",
                                rusqlite::params![block_id, hash, rel],
                            )?;
                            part_ids.push(block_id);
                            continue;
                        }
                        self.conn.execute(
                            "INSERT INTO note_blocks(todo_id, sort_order, kind, image_hash, image_file)
                             VALUES (?1, ?2, 'image', ?3, ?4)",
                            rusqlite::params![todo_id, next, hash, rel],
                        )?;
                        part_ids.push(self.conn.last_insert_rowid());
                        next += 1;
                    }
                    NotePastePart::PendingImage { url } => {
                        if reuse_original {
                            self.conn.execute(
                                "UPDATE note_blocks SET kind = 'text', text = ?2,
                                     image_hash = NULL, image_file = NULL, source_url = ?3
                                 WHERE id = ?1",
                                rusqlite::params![block_id, PENDING_NOTE_IMAGE_TEXT, url],
                            )?;
                            part_ids.push(block_id);
                            continue;
                        }
                        self.conn.execute(
                            "INSERT INTO note_blocks(todo_id, sort_order, kind, text, source_url)
                             VALUES (?1, ?2, 'text', ?3, ?4)",
                            rusqlite::params![todo_id, next, PENDING_NOTE_IMAGE_TEXT, url],
                        )?;
                        part_ids.push(self.conn.last_insert_rowid());
                        next += 1;
                    }
                }
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
            let texts: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM note_blocks
                 WHERE todo_id = ?1 AND kind = 'text' AND deleted_at IS NULL",
                [todo_id],
                |row| row.get(0),
            )?;
            // PATCH002.76：远端占位升级后会从文本块中消失 → 按“升级后”口径兜底，
            // 否则整条只有远程图时后台成功后会变成零文本块，下一次保存被校验拒绝
            if texts - pending_count <= 0 {
                let last: i64 = self.conn.query_row(
                    "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM note_blocks
                     WHERE todo_id = ?1 AND deleted_at IS NULL",
                    [todo_id],
                    |row| row.get(0),
                )?;
                self.conn.execute(
                    "INSERT INTO note_blocks(todo_id, sort_order, kind, text)
                     VALUES (?1, ?2, 'text', '')",
                    rusqlite::params![todo_id, last],
                )?;
            }
            Ok(part_ids)
        })();
        let part_ids = match result {
            Ok(part_ids) => {
                self.conn.execute_batch("COMMIT")?;
                part_ids
            }
            Err(err) => {
                if let Err(rollback) = self.conn.execute_batch("ROLLBACK") {
                    crate::diag::warn_diag(&format!("多段粘贴事务回滚失败：{rollback}"));
                }
                self.sweep_after_failed_write(images_dir); // PATCH002.46 补偿：刚落下的图不留孤儿
                return Err(err);
            }
        };
        Ok((self.note_blocks(todo_id)?, part_ids))
    }

    /// 后台下载成功（PATCH002.76）：占位文本块 → 图块。条件更新（文本仍为占位、
    /// source_url 仍匹配）防用户已编辑/删除；失败返回 None，文件由补偿清扫回收。
    #[allow(clippy::too_many_arguments)]
    pub fn complete_pending_note_image(
        &self,
        images_dir: &Path,
        todo_id: i64,
        block_id: i64,
        expected_url: &str,
        bytes: &[u8],
        orig_name: Option<&str>,
        preview: Option<&[u8]>,
        stamp: &str,
    ) -> Result<Option<NoteBlock>, StorageError> {
        let current: Option<(String, Option<String>)> = self
            .conn
            .query_row(
                "SELECT text, source_url FROM note_blocks
                 WHERE id = ?1 AND todo_id = ?2 AND kind = 'text' AND deleted_at IS NULL",
                rusqlite::params![block_id, todo_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((text, source)) = current else {
            return Ok(None);
        };
        if text != PENDING_NOTE_IMAGE_TEXT || source.as_deref() != Some(expected_url) {
            return Ok(None);
        }
        let (hash, rel) =
            match self.write_note_image_prepared(images_dir, bytes, orig_name, stamp, preview) {
                Ok(pair) => pair,
                Err(err) => {
                    self.sweep_after_failed_write(images_dir);
                    return Err(err);
                }
            };
        let changed = match self.conn.execute(
            "UPDATE note_blocks SET kind = 'image', text = '',
                 image_hash = ?3, image_file = ?4, source_url = ?5, deleted_at = NULL
             WHERE id = ?1 AND todo_id = ?2 AND kind = 'text' AND text = ?6
               AND source_url = ?5 AND deleted_at IS NULL",
            rusqlite::params![
                block_id,
                todo_id,
                hash,
                rel,
                expected_url,
                PENDING_NOTE_IMAGE_TEXT
            ],
        ) {
            Ok(changed) => changed,
            Err(err) => {
                self.sweep_after_failed_write(images_dir);
                return Err(StorageError::Sqlite(err));
            }
        };
        if changed == 0 {
            self.sweep_after_failed_write(images_dir);
            return Ok(None);
        }
        Ok(self
            .note_blocks(todo_id)?
            .into_iter()
            .find(|block| block.id == block_id))
    }

    /// 后台下载失败（PATCH002.76）：占位文本块 → URL 文本；保留 `source_url` 供
    /// `set_note_blocks` 识别迟到的“下载中…”保存。返回是否命中（用户已编辑/删除 = false）。
    pub fn fail_pending_note_image(
        &self,
        todo_id: i64,
        block_id: i64,
        url: &str,
    ) -> Result<bool, StorageError> {
        let changed = self.conn.execute(
            "UPDATE note_blocks SET text = ?3, source_url = ?3
             WHERE id = ?1 AND todo_id = ?2 AND kind = 'text' AND text = ?4
               AND source_url = ?3 AND deleted_at IS NULL",
            rusqlite::params![block_id, todo_id, url, PENDING_NOTE_IMAGE_TEXT],
        )?;
        Ok(changed > 0)
    }

    /// 启动清扫（PATCH002.76）：上次进程退出时仍在“下载中…”的占位 → URL 文本；
    /// 不做续传（用户定案）。返回清理行数。
    fn degrade_pending_note_images(&self) -> Result<usize, StorageError> {
        let changed = self.conn.execute(
            "UPDATE note_blocks SET text = source_url, source_url = NULL
             WHERE kind = 'text' AND text = ?1 AND source_url IS NOT NULL
               AND deleted_at IS NULL",
            [PENDING_NOTE_IMAGE_TEXT],
        )?;
        Ok(changed)
    }

    /// 详情块重排（图块拖拽落点提交）：id 集合须与该 todo 现存块完全一致（长度/幽灵/重复），
    /// 通过则按序 UPDATE sort_order（沿 reorder_todos 先例）。
    /// **不自行补块**（PATCH002.56 口径改定 2026-10-19）：末块不变量由**前端在保存点**唯一负责
    pub fn reorder_note_blocks(&self, todo_id: i64, ids: &[i64]) -> Result<(), StorageError> {
        // FIX014.11：集合校验与全部 UPDATE 同事务——中途失败不得留下部分新序。
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), StorageError> {
            let mut existing: Vec<i64> = self
                .conn
                .prepare("SELECT id FROM note_blocks WHERE todo_id = ?1 AND deleted_at IS NULL")?
                .query_map([todo_id], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;
            existing.sort_unstable();
            Self::validate_reorder_ids(ids, &existing, "详情块集")?;
            for (i, id) in ids.iter().enumerate() {
                self.conn.execute(
                    "UPDATE note_blocks SET sort_order = ?2
                     WHERE id = ?1 AND deleted_at IS NULL",
                    rusqlite::params![id, i as i64],
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
                    crate::diag::warn_diag(&format!("详情块重排事务回滚失败：{rollback}"));
                }
                Err(err)
            }
        }
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
        let inner = image_inner_name(orig_name, stamp, seq);
        let dir = images_dir.join(&folder);
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<BubbleAddOutcome, StorageError> {
            self.conn.execute(
                "INSERT INTO bubbles(text, kind, image_hash, image_source, sort_order)
                 VALUES (?1, 'image', ?2, ?3,
                         (SELECT COALESCE(MIN(sort_order), 0) - 1 FROM bubbles))",
                rusqlite::params![text, hash, source],
            )?;
            let id = self.conn.last_insert_rowid();
            std::fs::create_dir_all(&dir)?;
            std::fs::write(dir.join(&inner), bytes)?;
            // 大图预览（失败降级不阻断——详情退回原图）
            if let Some(preview) = crate::clipboard_image::generate_preview(bytes) {
                if let Err(err) = std::fs::write(dir.join("preview.png"), &preview) {
                    crate::diag::warn_diag(&format!("图片预览写入失败（降级用原图）：{err}"));
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
            Ok(outcome) => match self.conn.execute_batch("COMMIT") {
                Ok(()) => Ok(outcome),
                Err(err) => {
                    let _ = self.conn.execute_batch("ROLLBACK");
                    Self::cleanup_failed_image_folder(&dir);
                    Err(err.into())
                }
            },
            Err(err) => {
                if let Err(rollback) = self.conn.execute_batch("ROLLBACK") {
                    crate::diag::warn_diag(&format!("图片入库事务回滚失败：{rollback}"));
                }
                Self::cleanup_failed_image_folder(&dir);
                Err(err)
            }
        }
    }

    /// FIX014.15：失败补偿包一层，保留“只删本次文件夹”的语义并统一日志出口。
    fn cleanup_failed_image_folder(dir: &Path) {
        cleanup_image_folder(dir);
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
                    crate::diag::warn_diag(&format!("重排事务回滚失败：{rollback}"));
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
    fn note_blocks_indexes_are_created_idempotently() {
        // FIX014.13：迁移补两条索引；重复 init 不报错。
        let st = storage();
        st.init().expect("重复建表必须幂等");
        let indexes: Vec<String> = st
            .conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'note_blocks'",
            )
            .expect("查询索引必须成功")
            .query_map([], |row| row.get(0))
            .expect("读取索引必须成功")
            .collect::<Result<Vec<_>, _>>()
            .expect("收集索引必须成功");
        assert!(indexes
            .iter()
            .any(|name| name == "idx_note_blocks_todo_order"));
        assert!(indexes
            .iter()
            .any(|name| name == "idx_note_blocks_image_file"));
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
    fn remove_rolls_back_blocks_when_todo_delete_fails() {
        // FIX014.11：第二条 DELETE 失败不得留下“块没了、条目还在”。
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        st.set_note_blocks(&dir, item.id, &[nb_text(0, "笔记")])
            .expect("写块必须成功");
        st.conn
            .execute_batch(
                "CREATE TRIGGER fail_todo_delete
                 BEFORE DELETE ON todos
                 WHEN OLD.id = 1
                 BEGIN
                     SELECT RAISE(ABORT, 'injected delete failure');
                 END;",
            )
            .expect("造触发器必须成功");
        assert!(st.remove(item.id).is_err(), "注入失败必须返回错误");
        assert_eq!(
            st.note_blocks(item.id).expect("读块必须成功").len(),
            1,
            "第一条 DELETE 必须回滚"
        );
        assert!(st.get(item.id).is_ok(), "条目必须保留");
    }

    #[test]
    fn purge_expired_rolls_back_blocks_when_todo_delete_fails() {
        // FIX014.11：归档回收的两次 DELETE 也必须整体原子。
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("归档任务").expect("写入必须成功");
        st.set_note_blocks(&dir, item.id, &[nb_text(0, "笔记")])
            .expect("写块必须成功");
        st.toggle(item.id).expect("归档必须成功");
        st.conn
            .execute_batch(
                "CREATE TRIGGER fail_expired_delete
                 BEFORE DELETE ON todos
                 WHEN OLD.id = 1
                 BEGIN
                     SELECT RAISE(ABORT, 'injected purge failure');
                 END;",
            )
            .expect("造触发器必须成功");
        assert!(
            st.purge_expired_done(i64::MAX).is_err(),
            "注入失败必须返回错误"
        );
        assert_eq!(
            st.note_blocks(item.id).expect("读块必须成功").len(),
            1,
            "块删除必须回滚"
        );
        assert!(st.get(item.id).is_ok(), "归档条目必须保留");
    }

    #[test]
    fn reorder_note_blocks_rolls_back_partial_updates() {
        // FIX014.11：中途 UPDATE 失败不得留下部分新序。
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let saved = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "甲"), nb_text(0, "乙")])
            .expect("写块必须成功");
        let first_id = saved[0].id;
        st.conn
            .execute_batch(&format!(
                "CREATE TRIGGER fail_sort_update
                 BEFORE UPDATE OF sort_order ON note_blocks
                 WHEN OLD.id = {first_id}
                 BEGIN
                     SELECT RAISE(ABORT, 'injected reorder failure');
                 END;"
            ))
            .expect("造触发器必须成功");
        assert!(
            st.reorder_note_blocks(item.id, &[saved[1].id, first_id])
                .is_err(),
            "注入失败必须返回错误"
        );
        let texts: Vec<String> = st
            .note_blocks(item.id)
            .expect("读块必须成功")
            .into_iter()
            .map(|b| b.text)
            .collect();
        assert_eq!(texts, vec!["甲", "乙"], "排序必须整体回滚");
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
    fn image_bubble_failure_cleans_created_folder() {
        // FIX014.15：UPDATE image_file 失败时 DB 回滚，刚创建的 bubble_* 夹也要补偿删除。
        let st = storage();
        let dir = temp_images_dir();
        st.conn
            .execute_batch(
                "CREATE TRIGGER fail_bubble_image_update
                 BEFORE UPDATE OF image_file ON bubbles
                 BEGIN
                     SELECT RAISE(ABORT, 'injected bubble update failure');
                 END;",
            )
            .expect("造触发器必须成功");
        let err = st
            .add_image_bubble(&dir, "🖼️ 图片", &png_bytes(), Some("a.png"), "261022000000")
            .expect_err("注入失败必须返回错误");
        assert!(matches!(err, StorageError::Sqlite(_)));
        assert!(st.list_bubbles().expect("读取必须成功").is_empty());
        let folders = std::fs::read_dir(&dir)
            .expect("目录可读")
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("bubble_"))
            .count();
        assert_eq!(folders, 0, "回滚后不得残留 bubble_* 夹");
    }

    #[test]
    fn image_bubble_preview_named_file_keeps_original() {
        // FIX014.16：气泡图片原名 preview.png 时同样把原图改名。
        let st = storage();
        let dir = temp_images_dir();
        let item = st
            .add_image_bubble(
                &dir,
                "🖼️ 图片 preview.png",
                &png_bytes(),
                Some("preview.png"),
                "261022000000",
            )
            .expect("写入必须成功")
            .added_item();
        let (rel, _) = st
            .get_bubble_image_file(item.id)
            .expect("读取必须成功")
            .expect("应有文件");
        assert!(rel.ends_with("original.png"), "原图不得被预览覆盖");
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
    fn migration_rolls_back_all_rows_when_insert_fails() {
        // FIX014.4：第二行注入失败 → 整段迁移回滚；旧 note 列仍在，重试可完整迁移
        let st = pl010_storage_with_notes();
        st.conn
            .execute_batch(
                "CREATE TABLE note_blocks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    todo_id INTEGER NOT NULL,
                    sort_order INTEGER NOT NULL,
                    kind TEXT NOT NULL DEFAULT 'text',
                    text TEXT NOT NULL DEFAULT ''
                 );
                 CREATE TRIGGER fail_second_note_block
                 BEFORE INSERT ON note_blocks
                 WHEN NEW.todo_id = 2
                 BEGIN
                     SELECT RAISE(ABORT, 'injected failure');
                 END;",
            )
            .expect("造块表与故障触发器必须成功");
        st.migrate_note_to_blocks()
            .expect_err("第二行注入失败必须返回错误");
        let count: i64 = st
            .conn
            .query_row("SELECT COUNT(*) FROM note_blocks", [], |row| row.get(0))
            .expect("计数必须成功");
        assert_eq!(count, 0, "失败后不得留下半迁移块");
        assert!(
            st.column_set("todos")
                .expect("探列必须成功")
                .contains("note"),
            "迁移失败不得误删旧 note 列"
        );
        st.conn
            .execute_batch("DROP TRIGGER fail_second_note_block;")
            .expect("移除故障触发器必须成功");
        st.migrate_note_to_blocks()
            .expect("故障解除后必须可完整重试");
        let count: i64 = st
            .conn
            .query_row("SELECT COUNT(*) FROM note_blocks", [], |row| row.get(0))
            .expect("计数必须成功");
        assert_eq!(count, 5, "四行笔记按行拆块：1 + 2 + 1 + 1");
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
    fn set_note_blocks_missing_todo_is_not_found() {
        // FIX014.12：迟到保存不得给不存在的 todo 写孤儿块。
        let st = storage();
        let dir = temp_images_dir();
        let err = st
            .set_note_blocks(&dir, 999, &[nb_text(0, "孤儿")])
            .expect_err("不存在的 todo 必须拒绝");
        assert!(matches!(err, StorageError::NotFound(999)));
        assert!(st.note_blocks(999).expect("读块必须成功").is_empty());
    }

    #[test]
    fn set_note_blocks_rejects_duplicate_positive_ids() {
        // FIX014.12：重复正 id 会 last-wins、导致块集缩短；入口显式拒绝。
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let saved = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "甲"), nb_text(0, "乙")])
            .expect("写块必须成功");
        let duplicate = vec![saved[0].clone(), saved[0].clone()];
        let err = st
            .set_note_blocks(&dir, item.id, &duplicate)
            .expect_err("重复 id 必须拒绝");
        assert!(matches!(err, StorageError::ReorderMismatch(_)));
        assert_eq!(
            st.note_blocks(item.id).expect("读块必须成功").len(),
            2,
            "拒绝后原块集不得改变"
        );
    }

    /// PATCH002.72 C7：**旧 id 的行已被删除 ⇒ 按新块插入**（撤销"粘贴替换选区"要把被删的行写回；
    /// 原实现报 `NotFound` ⇒ 整次保存失败 ⇒ 撤销内容永远落不了库 ✗）
    #[test]
    fn set_note_blocks_reinserts_missing_text_id() {
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let saved = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "原文")])
            .expect("写块必须成功");
        let old_id = saved[0].id;
        // 用另一个块覆盖 ⇒ 差分删除会把 old_id 那一行删掉
        st.set_note_blocks(&dir, item.id, &[nb_text(0, "临时")])
            .expect("覆盖保存必须成功");
        // 把**旧 id** 写回 ⇒ 行已不存在 ⇒ 必须 upsert（原实现 NotFound ✗）
        let restored = st
            .set_note_blocks(&dir, item.id, &[nb_text(old_id, "原文")])
            .expect("旧 id 缺失时必须插入而不是拒绝");
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].text, "原文");
        assert!(restored[0].id > 0, "新行必须有 id");
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
    fn paste_parts_reuses_same_image_within_one_batch() {
        // FIX014.14：同一批两张同图，第二张必须命中批内 hash 缓存，不能落第二份。
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "文字")])
            .expect("写块必须成功");
        let parts = vec![
            NotePastePart::Image {
                bytes: png_bytes(),
                orig_name: None,
            },
            NotePastePart::Image {
                bytes: png_bytes(),
                orig_name: None,
            },
        ];
        let (blocks, _ids) = st
            .insert_note_parts(&dir, item.id, base[0].id, "", "", &parts, "261022000000")
            .expect("同批插入必须成功");
        let image_paths: Vec<&str> = blocks
            .iter()
            .filter(|b| b.kind == NoteBlockKind::Image)
            .filter_map(|b| b.image_file.as_deref())
            .collect();
        assert_eq!(image_paths.len(), 2);
        assert_eq!(image_paths[0], image_paths[1], "同批同图必须复用路径");
        assert_eq!(note_folder_count(&dir), 1, "盘上只能有一个 note_ 文件夹");
    }

    #[test]
    fn preview_named_original_does_not_overwrite_image() {
        // FIX014.16：原名 preview.png 时原图改 original.png，预览仍独立。
        let st = storage();
        let dir = temp_images_dir();
        let bytes = png_bytes();
        let preview = b"preview-bytes";
        let (_, rel) = st
            .write_note_image_prepared(
                &dir,
                &bytes,
                Some("preview.png"),
                "261022000000",
                Some(preview),
            )
            .expect("写图必须成功");
        assert!(rel.ends_with("original.png"), "原图必须改名");
        let folder = dir.join(rel.rsplit_once('/').expect("含子目录").0);
        assert_eq!(
            std::fs::read(dir.join(&rel)).expect("读原图必须成功"),
            bytes
        );
        assert_eq!(
            std::fs::read(folder.join("preview.png")).expect("读预览必须成功"),
            preview
        );
    }

    #[test]
    fn release_note_image_rejects_unsafe_paths() {
        // FIX014.17：空串 / 目录穿越不得 join 到 images 根或外部。
        let st = storage();
        let dir = temp_images_dir();
        let sentinel = dir.join("sentinel");
        std::fs::create_dir_all(&sentinel).expect("哨兵目录必须可建");
        assert!(matches!(
            st.release_note_image(&dir, ""),
            Err(StorageError::InvalidImagePath(_))
        ));
        assert!(matches!(
            st.release_note_image(&dir, "../sentinel"),
            Err(StorageError::InvalidImagePath(_))
        ));
        assert!(sentinel.is_dir(), "非法路径不得删除哨兵");
        assert!(is_safe_image_folder("bubble_261022_1", "bubble_"));
        assert!(!is_safe_image_folder("", "bubble_"));
        assert!(!is_safe_image_folder("../bubble_1", "bubble_"));
        assert!(!is_safe_image_folder("note_1", "bubble_"));
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

    /// 盘上 `note_*` 文件夹数（断言"校验在落盘之前"用）
    fn note_folder_count(dir: &std::path::Path) -> usize {
        std::fs::read_dir(dir)
            .map(|it| {
                it.flatten()
                    .filter(|e| {
                        e.path().is_dir() && e.file_name().to_string_lossy().starts_with("note_")
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    #[test]
    fn deleted_image_is_soft_kept_until_purge() {
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
                "文字",
                "",
                &png_bytes(),
                None,
                "261018000000",
            )
            .expect("插图必须成功");
        let image_id = with_img
            .iter()
            .find(|b| b.kind == NoteBlockKind::Image)
            .expect("必须有图块")
            .id;
        assert_eq!(note_folder_count(&dir), 1);

        // 删除图块：行软删除、文件保留，撤销可用旧 id 复活
        st.set_note_blocks(&dir, item.id, &[nb_text(base[0].id, "文字")])
            .expect("删图保存必须成功");
        assert!(st
            .note_blocks(item.id)
            .expect("读块必须成功")
            .iter()
            .all(|b| b.kind == NoteBlockKind::Text));
        assert_eq!(note_folder_count(&dir), 1, "关板前原图必须保留");

        let restored = st
            .set_note_blocks(
                &dir,
                item.id,
                &[nb_text(base[0].id, "文字"), nb_image(image_id)],
            )
            .expect("撤销恢复图块必须成功");
        assert_eq!(restored.len(), 2);
        assert!(restored.iter().any(|b| b.kind == NoteBlockKind::Image));

        // 再次删除并关板收口：硬删 + 文件释放
        st.set_note_blocks(&dir, item.id, &[nb_text(base[0].id, "文字")])
            .expect("再次删图必须成功");
        assert_eq!(
            st.purge_deleted_note_blocks(&dir, Some(item.id))
                .expect("关板收口必须成功"),
            1
        );
        assert_eq!(note_folder_count(&dir), 0, "关板后原图释放");
    }

    /// 图片块入参夹具（`set_note_blocks` 对 Image 只改 sort_order，故只需 id/kind）
    fn nb_image(id: i64) -> NoteBlock {
        NoteBlock {
            id,
            kind: NoteBlockKind::Image,
            text: String::new(),
            image_file: None,
        }
    }

    #[test]
    fn pending_image_placeholder_upgrades_and_keeps_text_block() {
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "")])
            .expect("写块必须成功");
        let url = "https://example.com/a.png".to_string();
        let parts = vec![NotePastePart::PendingImage { url: url.clone() }];
        let (blocks, part_ids) = st
            .insert_note_parts(&dir, item.id, base[0].id, "", "", &parts, "261022000000")
            .expect("占位插入必须成功");
        assert_eq!(part_ids.len(), 1, "part_ids 与 parts 同序");
        let placeholder = blocks
            .iter()
            .find(|b| b.text == PENDING_NOTE_IMAGE_TEXT)
            .expect("必须有下载中占位");
        assert_eq!(placeholder.kind, NoteBlockKind::Text);
        assert!(
            blocks
                .iter()
                .filter(|b| b.kind == NoteBlockKind::Text)
                .count()
                >= 2,
            "占位升级后仍要留一个文本块（占位自身不计）"
        );
        let updated = st
            .complete_pending_note_image(
                &dir,
                item.id,
                part_ids[0],
                &url,
                &png_bytes(),
                Some("pasted.png"),
                None,
                "261022000000",
            )
            .expect("后台落库必须成功")
            .expect("占位仍在，必须升级");
        assert_eq!(updated.kind, NoteBlockKind::Image);
        assert!(updated.image_file.is_some(), "图块必须有文件");
        let source: Option<String> = st
            .conn
            .query_row(
                "SELECT source_url FROM note_blocks WHERE id = ?1",
                [part_ids[0]],
                |row| row.get(0),
            )
            .expect("查 source_url");
        assert_eq!(source.as_deref(), Some(url.as_str()));
    }

    #[test]
    fn stale_placeholder_save_preserves_completed_image() {
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "")])
            .expect("写块必须成功");
        let url = "https://example.com/a.png".to_string();
        let parts = vec![NotePastePart::PendingImage { url: url.clone() }];
        let (_, part_ids) = st
            .insert_note_parts(&dir, item.id, base[0].id, "", "", &parts, "261022000001")
            .expect("占位插入必须成功");
        st.complete_pending_note_image(
            &dir,
            item.id,
            part_ids[0],
            &url,
            &png_bytes(),
            Some("pasted.png"),
            None,
            "261022000001",
        )
        .expect("后台落库")
        .expect("必须升级");

        // 迟到保存仍带“下载中…”占位 → 保图块
        let mut stale = st.note_blocks(item.id).expect("读块");
        for block in &mut stale {
            if block.id == part_ids[0] {
                block.kind = NoteBlockKind::Text;
                block.text = PENDING_NOTE_IMAGE_TEXT.to_string();
            }
        }
        let saved = st
            .set_note_blocks(&dir, item.id, &stale)
            .expect("迟到占位保存必须成功");
        let image = saved.iter().find(|b| b.id == part_ids[0]).expect("块仍在");
        assert_eq!(image.kind, NoteBlockKind::Image, "占位保存不得降级图块");
        assert!(image.image_file.is_some());

        // 迟到保存带原 URL → 同样保图块
        for block in &mut stale {
            if block.id == part_ids[0] {
                block.kind = NoteBlockKind::Text;
                block.text = url.clone();
            }
        }
        let saved = st
            .set_note_blocks(&dir, item.id, &stale)
            .expect("迟到 URL 保存必须成功");
        assert_eq!(
            saved
                .iter()
                .find(|b| b.id == part_ids[0])
                .expect("块仍在")
                .kind,
            NoteBlockKind::Image
        );

        // 用户真正改成其他文本 → 允许降级并释放图文件
        for block in &mut stale {
            if block.id == part_ids[0] {
                block.kind = NoteBlockKind::Text;
                block.text = "用户改过".to_string();
            }
        }
        let saved = st
            .set_note_blocks(&dir, item.id, &stale)
            .expect("真实编辑必须成功");
        let text = saved.iter().find(|b| b.id == part_ids[0]).expect("块仍在");
        assert_eq!(text.kind, NoteBlockKind::Text);
        assert_eq!(text.text, "用户改过");
        assert!(text.image_file.is_none(), "降级后文件引用清空");
    }

    #[test]
    fn failed_pending_image_degrades_to_url_and_guards_late_placeholder() {
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "")])
            .expect("写块必须成功");
        let url = "https://example.com/b.png".to_string();
        let parts = vec![NotePastePart::PendingImage { url: url.clone() }];
        let (_, part_ids) = st
            .insert_note_parts(&dir, item.id, base[0].id, "", "", &parts, "261022000002")
            .expect("占位插入必须成功");
        assert!(
            st.fail_pending_note_image(item.id, part_ids[0], &url)
                .expect("失败降级必须成功"),
            "占位仍在时必须命中"
        );
        let blocks = st.note_blocks(item.id).expect("读块");
        let failed = blocks.iter().find(|b| b.id == part_ids[0]).expect("块仍在");
        assert_eq!(failed.kind, NoteBlockKind::Text);
        assert_eq!(failed.text, url);

        // 迟到占位保存 → 保住 URL，不回到“下载中…”
        let mut stale = blocks;
        for block in &mut stale {
            if block.id == part_ids[0] {
                block.text = PENDING_NOTE_IMAGE_TEXT.to_string();
            }
        }
        let saved = st
            .set_note_blocks(&dir, item.id, &stale)
            .expect("迟到占位保存必须成功");
        assert_eq!(
            saved
                .iter()
                .find(|b| b.id == part_ids[0])
                .expect("块仍在")
                .text,
            url
        );
    }

    #[test]
    fn complete_pending_image_skips_after_user_edit() {
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "")])
            .expect("写块必须成功");
        let url = "https://example.com/c.png".to_string();
        let parts = vec![NotePastePart::PendingImage { url: url.clone() }];
        let (_, part_ids) = st
            .insert_note_parts(&dir, item.id, base[0].id, "", "", &parts, "261022000003")
            .expect("占位插入必须成功");
        let mut edited = st.note_blocks(item.id).expect("读块");
        for block in &mut edited {
            if block.id == part_ids[0] {
                block.text = "用户改过".to_string();
            }
        }
        st.set_note_blocks(&dir, item.id, &edited)
            .expect("编辑保存必须成功");
        let result = st
            .complete_pending_note_image(
                &dir,
                item.id,
                part_ids[0],
                &url,
                &png_bytes(),
                Some("pasted.png"),
                None,
                "261022000003",
            )
            .expect("后台任务不得报错");
        assert!(result.is_none(), "用户已改过 → 不覆盖");
    }

    #[test]
    fn startup_degrade_turns_pending_placeholder_into_url() {
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "")])
            .expect("写块必须成功");
        let url = "https://example.com/d.png".to_string();
        let parts = vec![NotePastePart::PendingImage { url: url.clone() }];
        let (_, part_ids) = st
            .insert_note_parts(&dir, item.id, base[0].id, "", "", &parts, "261022000004")
            .expect("占位插入必须成功");
        let count = st.degrade_pending_note_images().expect("启动清扫必须成功");
        assert_eq!(count, 1);
        let blocks = st.note_blocks(item.id).expect("读块");
        let degraded = blocks.iter().find(|b| b.id == part_ids[0]).expect("块仍在");
        assert_eq!(degraded.text, url);
        let source: Option<String> = st
            .conn
            .query_row(
                "SELECT source_url FROM note_blocks WHERE id = ?1",
                [part_ids[0]],
                |row| row.get(0),
            )
            .expect("查 source_url");
        assert!(source.is_none(), "降级后清空 source_url");
    }

    #[test]
    fn reopening_db_degrades_pending_placeholder() {
        let root = std::env::temp_dir().join(format!(
            "capsule-todo-reopen-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let db = root.join("todo.db");
        let images = root.join("images");
        let url = "https://example.com/e.png".to_string();
        let part_id;
        {
            let st = Storage::open(&db).expect("首次打开必须成功");
            std::fs::create_dir_all(&images).expect("图片目录");
            let item = st.add("任务").expect("写入必须成功");
            let base = st
                .set_note_blocks(&images, item.id, &[nb_text(0, "")])
                .expect("写块必须成功");
            let parts = vec![NotePastePart::PendingImage { url: url.clone() }];
            let (_, ids) = st
                .insert_note_parts(&images, item.id, base[0].id, "", "", &parts, "261022000005")
                .expect("占位插入必须成功");
            part_id = ids[0];
        }
        let st = Storage::open(&db).expect("重开必须成功");
        let item_id = st.list().expect("清单可读")[0].id;
        let blocks = st.note_blocks(item_id).expect("块可读");
        let degraded = blocks.iter().find(|b| b.id == part_id).expect("占位块仍在");
        assert_eq!(degraded.text, url, "启动清扫要把下载中占位换成 URL");
        assert_eq!(degraded.kind, NoteBlockKind::Text);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn storage_keeps_image_last_untouched() {
        // PATCH002.56 口径改定（2026-10-19）：末块不变量**由前端在保存点唯一负责** ——
        // storage 两条路径都**不自行补块**（否则"补 → 下次保存被差分删"会来回抖动）
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let base = st
            .set_note_blocks(&dir, item.id, &[nb_text(0, "A")])
            .expect("写块必须成功");
        let with_img = st
            .add_note_image_block(
                &dir,
                item.id,
                base[0].id,
                "A",
                "",
                &png_bytes(),
                None,
                "261019000000",
            )
            .expect("插图必须成功");
        let img_id = with_img
            .iter()
            .find(|b| b.kind == NoteBlockKind::Image)
            .expect("必有图块")
            .id;
        // 保存"末块是图"的块集 → 原样保留（不补）
        let saved = st
            .set_note_blocks(&dir, item.id, &[nb_text(base[0].id, "A"), nb_image(img_id)])
            .expect("保存必须成功");
        assert_eq!(saved.len(), 2, "storage 不自行补块");
        assert_eq!(
            saved.last().expect("非空").kind,
            NoteBlockKind::Image,
            "末块仍是图块"
        );
        // 重排同款：把图排到最后 → 同样不补
        st.reorder_note_blocks(item.id, &[base[0].id, img_id])
            .expect("重排必须成功");
        let after = st.note_blocks(item.id).expect("读块必须成功");
        assert_eq!(after.len(), 2, "重排后同样不补");
        assert_eq!(
            after.last().expect("非空").kind,
            NoteBlockKind::Image,
            "末块仍是图块"
        );
    }

    #[test]
    fn paste_parts_into_missing_block_fails_before_writing_files() {
        // PATCH002.46：目标块不存在（空笔记的幻影块 id=0 / 跨条目陈旧 id）→ 可读 NotFound，
        // 且**一个字节都不落盘**——此前"先落盘、后查块"会留下孤儿夹（PATCH002.41 的主因）
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let err = st
            .insert_note_parts(
                &dir,
                item.id,
                0,
                "",
                "",
                &[NotePastePart::Image {
                    bytes: png_bytes(),
                    orig_name: None,
                }],
                "261018000000",
            )
            .expect_err("目标块不存在必须报错");
        assert!(
            matches!(err, StorageError::NotFound(0)),
            "应报可读 NotFound（此前只有 QueryReturnedNoRows）"
        );
        assert_eq!(note_folder_count(&dir), 0, "校验在落盘之前 → 不该建文件夹");
    }

    #[test]
    fn insert_image_into_missing_block_fails_before_writing_files() {
        // PATCH002.46：单图插入同款——先校验后落盘
        let st = storage();
        let dir = temp_images_dir();
        let item = st.add("任务").expect("写入必须成功");
        let err = st
            .add_note_image_block(&dir, item.id, 0, "", "", &png_bytes(), None, "261018000000")
            .expect_err("目标块不存在必须报错");
        assert!(matches!(err, StorageError::NotFound(0)));
        assert_eq!(note_folder_count(&dir), 0, "不该建文件夹");
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
        // 删一个图块（软删除）：文件仍在（还有引用）
        let remaining = st
            .note_blocks(item.id)
            .expect("读块必须成功")
            .into_iter()
            .filter(|b| b.id != first_img_id)
            .collect::<Vec<_>>();
        st.set_note_blocks(&dir, item.id, &remaining)
            .expect("软删除图块必须成功");
        assert!(folder.is_dir(), "仍有引用 → 文件保留");
        // 删最后一个并关板收口：文件夹消失
        let text_only = st
            .note_blocks(item.id)
            .expect("读块必须成功")
            .into_iter()
            .filter(|b| b.kind == NoteBlockKind::Text)
            .collect::<Vec<_>>();
        st.set_note_blocks(&dir, item.id, &text_only)
            .expect("删最后图块必须成功");
        st.purge_deleted_note_blocks(&dir, Some(item.id))
            .expect("软删除收口必须成功");
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
