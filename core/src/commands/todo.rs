//! Todo 命令：清单增删勾查改（操作即落库，单一事实源 = db；核心抽自由函数直测）。
//! PL010 扩容：todo_rename/todo_set_note 新命令 + todo_list 附龄期裁决（业务在 Rust）。

use tauri::{Emitter, Manager, State};

use super::{AppContext, CommandError};
use crate::todo::{
    age_level, validate_note_blocks, validate_text, AgeLevel, NoteBlock, NoteBlockKind, TodoError,
    TodoItem, ARCHIVE_RETENTION_MS, MAX_NOTE_BLOCKS,
};

/// 变更广播（PL018.6 问题一修复；FIX006.1 载荷 origin 方案）：清单数据任何变更后
/// 由命令层发 todo-changed，载荷 = 发起窗 label——**前端自判早退**（发起窗刷新走
/// 自身组件链时序，300ms 主拍不被截断；跨窗照常刷新）。
/// 为什么不用 emit_filter：tauri 2.x 的 match_any_or_filter 语义下，JS listen()
/// 缺省 target=Any **恒通过无视 filter**（listener.rs 实证）——FIX005.5 的过滤
/// 方案对 JS 监听者整个无效，故改载荷显式 origin（详见 AGENTS 环境陷阱节）
fn emit_todo_changed(window: &tauri::Window, app: &tauri::AppHandle) {
    if let Err(err) = app.emit("todo-changed", window.label().to_string()) {
        crate::diag::warn_diag(&format!("todo-changed 广播失败：{err}"));
    }
}

/// 添加待办（文本校验 + 落库；拒绝经 CommandError 跨进程可见）→ 广播变更
#[tauri::command]
pub fn todo_add(
    window: tauri::Window,
    text: String,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<TodoItem, CommandError> {
    let item = todo_add_core(&text, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(item)
}

/// todo_add 核心实现：业务校验 + trim 落库（与 rename/bubble 同规——首尾空白不入库）
pub fn todo_add_core(text: &str, ctx: &AppContext) -> Result<TodoItem, CommandError> {
    validate_text(text)?;
    let storage = ctx.lock_storage()?;
    Ok(storage.add(text.trim())?)
}

/// 勾选翻转（不存在经 NotFound 严格报错）→ 广播变更
#[tauri::command]
pub fn todo_toggle(
    window: tauri::Window,
    id: i64,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<TodoItem, CommandError> {
    let item = todo_toggle_core(id, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(item)
}

/// todo_toggle 核心实现：翻转完成态（done_at 随语义落库：勾选置 now、退回清 NULL）
pub fn todo_toggle_core(id: i64, ctx: &AppContext) -> Result<TodoItem, CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.toggle(id)?)
}

/// 改标题（清单行内编辑/详情板标题共用；文本校验同 add）→ 广播变更（预览窗同步）
#[tauri::command]
pub fn todo_rename(
    window: tauri::Window,
    id: i64,
    text: String,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<TodoItem, CommandError> {
    let item = todo_rename_core(id, &text, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(item)
}

/// todo_rename 核心实现：文本校验 + 更新
pub fn todo_rename_core(id: i64, text: &str, ctx: &AppContext) -> Result<TodoItem, CommandError> {
    validate_text(text)?;
    let storage = ctx.lock_storage()?;
    Ok(storage.rename(id, text.trim())?)
}

/// 删除待办（不存在严格报错）→ 广播变更
#[tauri::command]
pub fn todo_remove(
    window: tauri::Window,
    id: i64,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    todo_remove_core(id, &images_dir, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(())
}

/// todo_remove 核心实现：删除条目 + PL027 连带释放其详情图文件（块行随删，清扫按
/// 引用归零删 `note_*` 夹；失败落日志不阻断——文件残留代价小于阻断删除）
pub fn todo_remove_core(
    id: i64,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    storage.remove(id)?;
    if let Err(err) = storage.sweep_orphan_note_images(images_dir) {
        crate::diag::warn_diag(&format!(
            "删条目后详情图清扫失败（best-effort 不阻断）：{err}"
        ));
    }
    Ok(())
}

/// 清单视图条目：TodoItem + 龄期档位（龄期裁决在 Rust 侧，前端零业务——A001 dim 12
/// "阈值裁决不漂移"同款纪律）
#[derive(Debug, serde::Serialize)]
pub struct TodoView {
    /// 条目本体（扁平展开字段）
    #[serde(flatten)]
    pub item: TodoItem,
    /// 龄期提醒档位（按当前时刻裁决）
    pub age_level: AgeLevel,
}

/// 清单排序视图（未完成在前按 sort_order 升序（拖拽序）、id 兜底，已完成在后；逐条附龄期档位）
#[tauri::command]
pub fn todo_list(ctx: State<'_, AppContext>) -> Result<Vec<TodoView>, CommandError> {
    todo_list_core(&ctx)
}

/// todo_list 核心实现：出排序视图 + 逐条龄期裁决（时间源 = storage.now）
pub fn todo_list_core(ctx: &AppContext) -> Result<Vec<TodoView>, CommandError> {
    let storage = ctx.lock_storage()?;
    let now = storage.now_ms();
    Ok(storage
        .list()?
        .into_iter()
        .map(|item| TodoView {
            age_level: age_level(item.created_at, now),
            item,
        })
        .collect())
}

/// 归档视图（已完成条目，done_at 倒序——最新完成的在最上）
#[tauri::command]
pub fn todo_archive_list(ctx: State<'_, AppContext>) -> Result<Vec<TodoItem>, CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    todo_archive_list_core(&ctx, &images_dir)
}

/// todo_archive_list 核心实现：归档读路径惰性清扫（PATCH001）+ 出归档视图。
/// 清扫挂读路径 = 打卡软件模式在常驻板上的等价形态：本命令调用点天然覆盖 App 挂载
/// （App.vue 启动拉归档）与每次清单变更（onListChanged 三源齐拉），超期条目在出视图
/// 前已删、前端永无中间态，零新线程零定时器；删除数 > 0 落一行日志对账。
/// PL027：回收连带释放其详情图文件（块行随删，清扫按引用归零删 `note_*` 夹）
pub fn todo_archive_list_core(
    ctx: &AppContext,
    images_dir: &std::path::Path,
) -> Result<Vec<TodoItem>, CommandError> {
    let storage = ctx.lock_storage()?;
    let cutoff = storage.now_ms() - ARCHIVE_RETENTION_MS;
    let purged = storage.purge_expired_done(cutoff)?;
    if purged > 0 {
        crate::diag::console(&format!("归档回收：清理 {purged} 条超期条目"));
        if let Err(err) = storage.sweep_orphan_note_images(images_dir) {
            crate::diag::warn_diag(&format!(
                "归档回收后详情图清扫失败（best-effort 不阻断）：{err}"
            ));
        }
    }
    Ok(storage.list_done()?)
}

/// 清单重排（拖拽落点提交）：ids = 未完成条目的目标顺序（全量集，一致性校验在 storage 层）
/// → 广播变更（FIX005.5 补齐：拖拽排序原漏发致预览窗 stale）
#[tauri::command]
pub fn todo_reorder(
    window: tauri::Window,
    ids: Vec<i64>,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    todo_reorder_core(&ids, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(())
}

/// todo_reorder 核心实现：透传 storage.reorder_todos（校验+事务在存储层）
pub fn todo_reorder_core(ids: &[i64], ctx: &AppContext) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.reorder_todos(ids)?)
}

// —— PL027 详情块命令（一段一行一块 + 图块混排；业务在存储层，本层只做取数/广播/把关）——

/// 块视图（跨 IPC）：图块带**绝对路径**（前端 `convertFileSrc` 直用）；大图预览优先
#[derive(Debug, Clone, serde::Serialize)]
pub struct NoteBlockView {
    /// 块唯一标识（本地新块在保存前为 0）
    pub id: i64,
    /// 块类型
    pub kind: NoteBlockKind,
    /// 文本内容（图块恒空串）
    pub text: String,
    /// 图片绝对路径（图块专用；文本块 = None）——有 `preview.png` 时回预览（详情秒开）
    pub image_path: Option<String>,
}

/// 块保存入参（前端回传：id > 0 = 既有块、0 = 新块；图块文件信息在库内，无需回传）
#[derive(Debug, serde::Deserialize)]
pub struct NoteBlockInput {
    /// 块 id（0 = 新块）
    pub id: i64,
    /// 块类型
    pub kind: NoteBlockKind,
    /// 文本内容
    pub text: String,
}

/// 粘贴插图结果（跨 IPC）：命中图 → 新块列表（前端整体替换本地态）；未命中 → 回落文本粘贴
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NotePasteOutcome {
    /// 命中剪贴板图片并已插入
    Image { blocks: Vec<NoteBlockView> },
    /// 剪贴板无图片（前端回落文本粘贴）
    NoImage,
}

/// 远端图后台任务（PATCH002.76）：block_id = 占位文本块 id；url 已规范化
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingImageJob {
    /// 占位文本块 id
    pub block_id: i64,
    /// 规范化后的 http(s) URL
    pub url: String,
}

/// HTML 粘贴结果（PATCH002.76）：outcome 立即回前端；pending 由前端随后启动后台下载
#[derive(Debug, serde::Serialize)]
pub struct PasteHtmlResult {
    /// 粘贴结果（占位块已入库）
    pub outcome: NotePasteOutcome,
    /// 待后台下载的远端图任务
    pub pending: Vec<PendingImageJob>,
}

/// 后台图片就绪事件（PATCH002.76）：前端按 id/url 校验后把占位替换为图块
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NoteImageReady {
    todo_id: i64,
    block: NoteBlockView,
    url: String,
}

/// 后台图片失败事件（PATCH002.76）：前端把“下载中…”占位替换为 URL 文本
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NoteImageFailed {
    todo_id: i64,
    block_id: i64,
    url: String,
}

/// 块 → 视图（解析绝对路径 + 预览优先）
fn to_block_view(block: NoteBlock, images_dir: &std::path::Path) -> NoteBlockView {
    let image_path = block.image_file.as_ref().map(|rel| {
        let abs = images_dir.join(rel);
        match abs.parent().map(|dir| dir.join("preview.png")) {
            Some(preview) if preview.is_file() => preview.to_string_lossy().into_owned(),
            _ => abs.to_string_lossy().into_owned(),
        }
    });
    NoteBlockView {
        id: block.id,
        kind: block.kind,
        text: block.text,
        image_path,
    }
}

/// 读该条目的详情块（开板时按需拉，**不进清单载荷**）
#[tauri::command]
pub fn note_blocks_list(
    todo_id: i64,
    ctx: State<'_, AppContext>,
) -> Result<Vec<NoteBlockView>, CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    note_blocks_list_core(todo_id, &images_dir, &ctx)
}

/// note_blocks_list 核心实现：出块视图（预览优先）
pub fn note_blocks_list_core(
    todo_id: i64,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<Vec<NoteBlockView>, CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage
        .note_blocks(todo_id)?
        .into_iter()
        .map(|b| to_block_view(b, images_dir))
        .collect())
}

/// 全量覆盖保存详情块（前端防抖 300ms 后调用）→ 广播变更
#[tauri::command]
pub fn note_blocks_save(
    window: tauri::Window,
    todo_id: i64,
    blocks: Vec<NoteBlockInput>,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<Vec<NoteBlockView>, CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    let saved = note_blocks_save_core(todo_id, &blocks, &images_dir, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(saved)
}

/// note_blocks_save 核心实现：不变量校验（单点在 todo::validate_note_blocks）+ 差分落库
pub fn note_blocks_save_core(
    todo_id: i64,
    blocks: &[NoteBlockInput],
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<Vec<NoteBlockView>, CommandError> {
    let model: Vec<NoteBlock> = blocks
        .iter()
        .map(|b| NoteBlock {
            id: b.id,
            kind: b.kind,
            text: b.text.clone(),
            image_file: None,
        })
        .collect();
    validate_note_blocks(&model)?;
    let storage = ctx.lock_storage()?;
    let saved = storage.set_note_blocks(images_dir, todo_id, &model)?;
    Ok(saved
        .into_iter()
        .map(|b| to_block_view(b, images_dir))
        .collect())
}

/// FIX014.2：关板 finalize——硬删该条目软删除的图块并释放原图；无图行时零操作。
#[tauri::command]
pub fn note_blocks_purge_deleted(
    todo_id: i64,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    note_blocks_purge_deleted_core(todo_id, &images_dir, &ctx)
}

/// note_blocks_purge_deleted 核心实现（关板/启动共用的存储层收口）。
pub fn note_blocks_purge_deleted_core(
    todo_id: i64,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    storage.purge_deleted_note_blocks(images_dir, Some(todo_id))?;
    Ok(())
}

/// 粘贴插图（**唯一插图入口**）：命中剪贴板图片 → 落盘/复用 + 光标处切分插入 → 广播
/// 粘贴剪贴板图片（PATCH002.71：读剪贴板 / 解码 / 落盘 / 预览全部移到阻塞线程，
/// 不再冻结 UI 主线程）
#[tauri::command]
pub async fn note_paste_image(
    window: tauri::Window,
    todo_id: i64,
    block_id: i64,
    before: String,
    after: String,
    app: tauri::AppHandle,
) -> Result<NotePasteOutcome, CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    let task_app = app.clone();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let ctx = task_app.state::<AppContext>();
        note_paste_image_core(todo_id, block_id, &before, &after, &images_dir, &ctx)
    })
    .await
    .map_err(|err| CommandError::Storage(format!("图片粘贴任务失败：{err}")))??;
    if matches!(outcome, NotePasteOutcome::Image { .. }) {
        emit_todo_changed(&window, &app);
    }
    Ok(outcome)
}

/// 剪贴板读取薄壳（不可注入直测）：无图 → `NoImage`（前端回落文本粘贴）
pub fn note_paste_image_core(
    todo_id: i64,
    block_id: i64,
    before: &str,
    after: &str,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<NotePasteOutcome, CommandError> {
    match crate::clipboard_image::read_clipboard_image() {
        Some(captured) => {
            note_insert_image_core(todo_id, block_id, before, after, &captured, images_dir, ctx)
        }
        None => Ok(NotePasteOutcome::NoImage),
    }
}

/// 插图核心（**可注入字节直测**）：格式校验 → 块数上限把关 → 存储层落盘/复用 + 切分插入。
/// `before` / `after` 由前端按光标/选区本地切好（免 UTF-16 ↔ 字节换算）
pub fn note_insert_image_core(
    todo_id: i64,
    block_id: i64,
    before: &str,
    after: &str,
    captured: &crate::clipboard_image::CapturedImage,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<NotePasteOutcome, CommandError> {
    crate::clipboard_image::validate_image_bytes(&captured.bytes)?;
    let stamp = crate::util::local_stamp();
    let storage = ctx.lock_storage()?;
    // 插入后块数 = 现有 + 2（原块复用为 before 文本块）——超限直接拒（不写盘不插块）
    if storage.note_blocks(todo_id)?.len() + 2 > MAX_NOTE_BLOCKS {
        return Err(CommandError::Todo(TodoError::TooManyBlocks.to_string()));
    }
    let blocks = storage.add_note_image_block(
        images_dir,
        todo_id,
        block_id,
        before,
        after,
        &captured.bytes,
        captured.file_name.as_deref(),
        &stamp,
    )?;
    Ok(NotePasteOutcome::Image {
        blocks: blocks
            .into_iter()
            .map(|b| to_block_view(b, images_dir))
            .collect(),
    })
}

/// 详情块重排（图块拖拽落点提交）→ 广播变更
#[tauri::command]
pub fn note_reorder(
    window: tauri::Window,
    todo_id: i64,
    ids: Vec<i64>,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    note_reorder_core(todo_id, &ids, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(())
}

/// note_reorder 核心实现：透传 storage.reorder_note_blocks（集合一致性校验在存储层）
pub fn note_reorder_core(todo_id: i64, ids: &[i64], ctx: &AppContext) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.reorder_note_blocks(todo_id, ids)?)
}

/// 双击图块 → 系统默认程序打开**原图**（跳过预览）
#[tauri::command]
pub fn note_open_image(block_id: i64, ctx: State<'_, AppContext>) -> Result<(), CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    note_open_image_core(block_id, &images_dir, &ctx)
}

/// note_open_image 核心实现：块 → 原图绝对路径（**跳过 preview.png**）→ 系统默认程序打开；
/// 文本块 / 文件缺失严格报错
pub fn note_open_image_core(
    block_id: i64,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    let block = storage.get_note_block(block_id)?;
    let rel = block
        .image_file
        .ok_or_else(|| CommandError::Todo("该块不是图片块".to_string()))?;
    let abs = images_dir.join(&rel);
    if !abs.is_file() {
        return Err(CommandError::Todo("图片文件已移动或删除".to_string()));
    }
    crate::util::open_path_with_default(&abs).map_err(CommandError::Todo)
}

// —— 复制选区 / 图块（PATCH002.24：纯文本 + CF_HTML + 首图三格式，一次写入剪贴板）——

/// 复制入参：文本块原文 + 图块 id（文件信息在库内，路径不过 IPC）
#[derive(Debug, serde::Deserialize)]
pub struct NoteCopyBlock {
    /// 块类型
    pub kind: NoteBlockKind,
    /// 文本内容（图块恒空串）
    pub text: String,
    /// 块 id（图块取文件用；0 = 尚未落库 → 无图可带）
    pub block_id: i64,
}

/// CF_HTML 内联图体积上限（单张 / 合计）：超限的图降级为文本占位，**不阻断复制**
const COPY_INLINE_MAX_ONE: u64 = 3 * 1024 * 1024;
const COPY_INLINE_MAX_ALL: u64 = 8 * 1024 * 1024;

/// 落盘图片 → CF_HTML 内联 data URL 的 MIME（与落盘格式白名单一致；未知回落 PNG）
fn image_mime(name: &str) -> &'static str {
    match std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        _ => "image/png",
    }
}

/// HTML 文本转义（`&`/`<`/`>`/`"`）；换行由调用方转 `<br>`
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 首图三格式所需数据（PATCH002.71：**只解码一次**，PNG / DIB / DIBV5 全部同源）
pub struct NoteCopyFirstImage {
    /// PNG 注册格式载荷（源为 PNG 原样直通；其余按旧 `to_png` 同路径重编码）
    pub png: Vec<u8>,
    /// 已解码像素（行序 RGBA）
    pub rgba: (u32, u32, Vec<u8>),
}

/// 复制载荷（**锁外**构建的纯函数产出：CF_HTML + 纯文本 + 首图数据）
pub struct NoteCopyPayload {
    pub html: String,
    pub text: String,
    pub first: Option<NoteCopyFirstImage>,
}

/// 首图 → 三格式数据：**1 次解码**（PNG 源直通；其余解码后按旧口径重编码，
/// 与原 `to_png` 逐字节一致）——PATCH002.71 的核心省时点
fn first_image_formats(bytes: &[u8]) -> Result<NoteCopyFirstImage, CommandError> {
    crate::clipboard_image::validate_image_bytes(bytes)?;
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        let (w, h, rgba) = crate::clipboard_image::decode_to_rgba(bytes)
            .map_err(|err| CommandError::Clipboard(format!("图片解码失败：{err}")))?;
        return Ok(NoteCopyFirstImage {
            png: bytes.to_vec(),
            rgba: (w, h, rgba),
        });
    }
    let img = image::load_from_memory(bytes)
        .map_err(|err| CommandError::Clipboard(format!("图片解码失败：{err}")))?;
    let (w, h) = (img.width(), img.height());
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)
        .map_err(|err| CommandError::Clipboard(format!("图片转 PNG 失败：{err}")))?;
    Ok(NoteCopyFirstImage {
        png: out.into_inner(),
        rgba: (w, h, img.to_rgba8().into_raw()),
    })
}

/// 收集图块对应的图片相对路径（**锁内只此一步**；PATCH002.71 出锁后不再持库）
fn copy_image_rels(
    blocks: &[NoteCopyBlock],
    storage: &crate::storage::Storage,
) -> Vec<Option<String>> {
    blocks
        .iter()
        .map(|block| {
            if !matches!(block.kind, NoteBlockKind::Image) || block.block_id <= 0 {
                return None;
            }
            storage
                .get_note_block(block.block_id)
                .ok()
                .and_then(|b| b.image_file)
        })
        .collect()
}

/// 构建复制载荷（**入参自带块集与图块路径、不读剪贴板、不持库 → 可直测**）：
/// 拼纯文本 + CF_HTML（应用内图**内联 base64 data URL**，超上限降级占位）+ 首图三格式数据。
/// 图块文件缺失 → 该图降级占位，不中断整体复制
pub fn note_copy_payload(
    blocks: &[NoteCopyBlock],
    rels: &[Option<String>],
    images_dir: &std::path::Path,
) -> Result<NoteCopyPayload, CommandError> {
    use base64::Engine as _;
    let mut texts: Vec<String> = Vec::new();
    let mut htmls: Vec<String> = Vec::new();
    let mut first: Option<NoteCopyFirstImage> = None;
    let mut inline_total: u64 = 0;
    for (i, block) in blocks.iter().enumerate() {
        if matches!(block.kind, NoteBlockKind::Text) {
            texts.push(block.text.clone());
            htmls.push(escape_html(&block.text).replace('\n', "<br>"));
            continue;
        }
        texts.push("🖼 图片".to_string());
        let Some(rel) = rels.get(i).and_then(|rel| rel.as_deref()) else {
            htmls.push("🖼 图片".to_string());
            continue;
        };
        let abs = images_dir.join(rel);
        let Ok(bytes) = std::fs::read(&abs) else {
            htmls.push("🖼 图片".to_string()); // 文件缺失 → 占位，不中断整体复制
            continue;
        };
        if first.is_none() {
            first = Some(first_image_formats(&bytes)?);
        }
        let len = bytes.len() as u64;
        if len > COPY_INLINE_MAX_ONE || inline_total + len > COPY_INLINE_MAX_ALL {
            htmls.push("🖼 图片".to_string()); // 超限降级（免剪贴板爆量）
            continue;
        }
        inline_total += len;
        let name = abs
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.png");
        htmls.push(format!(
            "<img src=\"data:{};base64,{}\" alt=\"图片\">",
            image_mime(name),
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        ));
    }
    Ok(NoteCopyPayload {
        html: crate::clipboard_image::build_cf_html(&htmls.join("<br>")),
        text: texts.join("\n"),
        first,
    })
}

/// 复制串行锁（PATCH002.71：异步化后两次复制并发会抢剪贴板，串行化保证后一次完整覆盖）
static COPY_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 复制选区 / 图块到系统剪贴板（PATCH002.71：**锁外构建 + 阻塞线程写剪贴板** ——
/// 重活（读文件 / 解码 / base64 / DIB 构建 / 写剪贴板）不再冻结 UI，也不再堵自动保存）
#[tauri::command]
pub async fn note_copy_selection(
    blocks: Vec<NoteCopyBlock>,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    // 锁内只收集图块文件（毫秒级）；`MutexGuard` 不是 Send ⇒ 必须出锁再 await
    let rels = {
        let storage = ctx.lock_storage()?;
        copy_image_rels(&blocks, &storage)
    };
    tauri::async_runtime::spawn_blocking(move || {
        let _serial = match COPY_SERIAL.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(), // 中毒恢复取值（复制不携状态，安全）
        };
        let NoteCopyPayload { html, text, first } = note_copy_payload(&blocks, &rels, &images_dir)?;
        let (png, pixels) = match first {
            Some(image) => (Some(image.png), Some(image.rgba)),
            None => (None, None),
        };
        crate::clipboard_image::write_clipboard_rich_with_pixels(
            &html,
            png.as_deref(),
            pixels,
            &text,
        )
        .map_err(|err| CommandError::Clipboard(format!("写入剪贴板失败：{err}")))
    })
    .await
    .map_err(|err| CommandError::Clipboard(format!("复制任务失败：{err}")))?
}

// —— 粘贴含图 HTML（PATCH002.25：CF_HTML 多图 → 文本块 / 图块按原序插入）——

/// `data:` 图体积上限（超限降级为文本链接，不阻断其余段）
const PASTE_HTML_MAX_ONE: usize = 10 * 1024 * 1024;

/// 粘贴含图 HTML（PATCH002.71：base64 解码 / 落盘 / 预览移到阻塞线程，不再冻结 UI）
#[tauri::command]
pub async fn note_paste_html(
    window: tauri::Window,
    todo_id: i64,
    block_id: i64,
    before: String,
    after: String,
    html: String,
    app: tauri::AppHandle,
) -> Result<PasteHtmlResult, CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    let task_app = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let ctx = task_app.state::<AppContext>();
        note_paste_html_core(todo_id, block_id, &before, &after, &html, &images_dir, &ctx)
    })
    .await
    .map_err(|err| CommandError::Storage(format!("HTML 粘贴任务失败：{err}")))??;
    if matches!(result.outcome, NotePasteOutcome::Image { .. }) {
        emit_todo_changed(&window, &app);
    }
    Ok(result)
}

/// 构造 HTML 粘贴后的最终块集（FIX014.6）：只读查询，不落盘、不写库。
/// `insert_note_parts` 会按 before/after/parts 重组块序列；这里按同一语义预览，
/// 供 `validate_note_blocks` 在**写入前**拦截超限。
fn preview_html_final_blocks(
    storage: &crate::storage::Storage,
    todo_id: i64,
    block_id: i64,
    before: &str,
    after: &str,
    parts: &[crate::storage::NotePastePart],
) -> Result<Vec<NoteBlock>, CommandError> {
    let current = storage.note_blocks(todo_id)?;
    let index = current
        .iter()
        .position(|block| block.id == block_id)
        .ok_or_else(|| CommandError::Storage(format!("目标块不存在：{block_id}")))?;
    let follows = current.len().saturating_sub(index + 1);
    let keep_before = !before.trim().is_empty();
    let keep_after = !after.trim().is_empty();
    let last_is_image = matches!(
        parts.last(),
        Some(crate::storage::NotePastePart::Image { .. })
            | Some(crate::storage::NotePastePart::PendingImage { .. })
    );

    let mut final_blocks: Vec<NoteBlock> = current[..index].to_vec();
    if keep_before {
        final_blocks.push(NoteBlock {
            id: 0,
            kind: NoteBlockKind::Text,
            text: before.to_string(),
            image_file: None,
        });
    }
    for part in parts {
        final_blocks.push(match part {
            crate::storage::NotePastePart::Text(text) => NoteBlock {
                id: 0,
                kind: NoteBlockKind::Text,
                text: text.clone(),
                image_file: None,
            },
            // 远端占位升级后是图块；按图块口径校验，避免把占位文案算进文本长度
            crate::storage::NotePastePart::Image { .. }
            | crate::storage::NotePastePart::PendingImage { .. } => NoteBlock {
                id: 0,
                kind: NoteBlockKind::Image,
                text: String::new(),
                image_file: None,
            },
        });
    }
    if keep_after {
        final_blocks.push(NoteBlock {
            id: 0,
            kind: NoteBlockKind::Text,
            text: after.to_string(),
            image_file: None,
        });
    }
    let mut needs_blank = !keep_after && follows == 0 && last_is_image;
    if !final_blocks.iter().any(|b| b.kind == NoteBlockKind::Text) {
        needs_blank = true;
    }
    if needs_blank {
        final_blocks.push(NoteBlock {
            id: 0,
            kind: NoteBlockKind::Text,
            text: String::new(),
            image_file: None,
        });
    }
    final_blocks.extend_from_slice(&current[index + 1..]);
    Ok(final_blocks)
}

/// 粘贴核心（**不读剪贴板、不联网 → 可直测**）：HTML → 片段序列 → 多段插入。
/// `data:` 图直解入库；http(s) 图先落 `🖼 下载中…` 占位 + 返回 pending 任务，
/// 由前端随后调 `note_fetch_pending_images` 后台下载（PATCH002.76）；其他 scheme /
/// 相对路径降级文本。段落内残留换行按行再切（块不变量：文本段无换行）。
/// 恒回 `Image { blocks }`（前端一律整体替换本地块）；片段全空 → `NoImage` 交前端回落文本粘贴
pub fn note_paste_html_core(
    todo_id: i64,
    block_id: i64,
    before: &str,
    after: &str,
    html: &str,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<PasteHtmlResult, CommandError> {
    let mut parts: Vec<crate::storage::NotePastePart> = Vec::new();
    for part in crate::html_paste::parse(html) {
        match part {
            crate::html_paste::HtmlPart::Text(text) => {
                for line in text.split('\n') {
                    let line = line.trim();
                    if !line.is_empty() {
                        parts.push(crate::storage::NotePastePart::Text(line.to_string()));
                    }
                }
            }
            crate::html_paste::HtmlPart::Image(src) => {
                if let Some((bytes, orig_name)) = decode_data_url(&src) {
                    if bytes.len() <= PASTE_HTML_MAX_ONE
                        && crate::clipboard_image::validate_image_bytes(&bytes).is_ok()
                    {
                        parts.push(crate::storage::NotePastePart::Image { bytes, orig_name });
                    } else {
                        // data: 超限 / 解码失败 / 非法字节（SVG、截断 PNG…）→ 短占位
                        // （**判定而非传播**：PATCH002.46 前这里用 `?` 把单段失败升格整条）
                        parts.push(crate::storage::NotePastePart::Text(paste_fallback_text(
                            &src,
                        )));
                    }
                    continue;
                }
                match crate::image_fetch::normalize_url(&src) {
                    Some(url) => parts.push(crate::storage::NotePastePart::PendingImage { url }),
                    // 其他 scheme / 相对路径 / 非法 URL → 原样文本链接
                    None => parts.push(crate::storage::NotePastePart::Text(paste_fallback_text(
                        &src,
                    ))),
                }
            }
        }
    }
    if parts.is_empty() {
        return Ok(PasteHtmlResult {
            outcome: NotePasteOutcome::NoImage,
            pending: Vec::new(),
        });
    }
    let stamp = crate::util::local_stamp();
    let storage = ctx.lock_storage()?;
    // +3：before / after / 空文本兜底（含远端占位升级后口径）各留一格
    if storage.note_blocks(todo_id)?.len() + parts.len() + 3 > MAX_NOTE_BLOCKS {
        return Err(CommandError::Todo(TodoError::TooManyBlocks.to_string()));
    }
    // FIX014.6：在写文件/写库前按最终块集统一校验（总长 / 块数 / 换行）；
    // 超限直接拒绝，前端静默中止，不留下任何半成品。
    let final_blocks =
        preview_html_final_blocks(&storage, todo_id, block_id, before, after, &parts)?;
    validate_note_blocks(&final_blocks).map_err(|err| CommandError::Todo(err.to_string()))?;
    let (blocks, part_ids) =
        storage.insert_note_parts(images_dir, todo_id, block_id, before, after, &parts, &stamp)?;
    let pending = parts
        .iter()
        .zip(part_ids)
        .filter_map(|(part, block_id)| match part {
            crate::storage::NotePastePart::PendingImage { url } => Some(PendingImageJob {
                block_id,
                url: url.clone(),
            }),
            _ => None,
        })
        .collect();
    Ok(PasteHtmlResult {
        outcome: NotePasteOutcome::Image {
            blocks: blocks
                .into_iter()
                .map(|b| to_block_view(b, images_dir))
                .collect(),
        },
        pending,
    })
}

/// 启动远端图后台下载（PATCH002.76）：**立即返回**，不阻塞粘贴请求。
/// 前端在应用完占位块后调用；每张成功/失败各发一个事件，失败不影响其他段。
#[tauri::command]
pub async fn note_fetch_pending_images(
    todo_id: i64,
    jobs: Vec<PendingImageJob>,
    app: tauri::AppHandle,
) -> Result<(), CommandError> {
    if jobs.is_empty() {
        return Ok(());
    }
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    // 只接规范化 http(s)；上限沿块数上限（防异常前端载荷）
    let jobs: Vec<PendingImageJob> = jobs
        .into_iter()
        .filter_map(|mut job| {
            let normalized = crate::image_fetch::normalize_url(&job.url)?;
            job.url = normalized;
            Some(job)
        })
        .take(MAX_NOTE_BLOCKS)
        .collect();
    if jobs.is_empty() {
        return Ok(());
    }
    let task_app = app.clone();
    // 后台任务脱离命令生命周期；JoinHandle 丢弃不取消任务
    std::mem::drop(tauri::async_runtime::spawn_blocking(move || {
        run_pending_image_jobs(task_app, todo_id, jobs, images_dir);
    }));
    Ok(())
}

/// 后台任务主体：并发取图（模块内限 4/10s/10MB/20s/50MB）→ 逐张条件更新 → 发事件。
/// **失败一律降级**：能更新库就把占位换成 URL；库锁失败也发失败事件，让前端先降级、
/// 下一次保存再落库（不把“下载中…”永久留在板上）。
fn run_pending_image_jobs(
    app: tauri::AppHandle,
    todo_id: i64,
    jobs: Vec<PendingImageJob>,
    images_dir: std::path::PathBuf,
) {
    let urls: Vec<String> = jobs.iter().map(|job| job.url.clone()).collect();
    let fetched = crate::image_fetch::fetch_all(&urls);
    let stamp = crate::util::local_stamp();
    let ctx = app.state::<AppContext>();

    for (job, result) in jobs.into_iter().zip(fetched) {
        let bytes = match result {
            Ok(bytes) => bytes,
            Err(err) => {
                degrade_pending_image(
                    &app,
                    ctx.inner(),
                    todo_id,
                    &job,
                    &format!("下载失败：{err}"),
                );
                continue;
            }
        };
        if bytes.len() > crate::image_fetch::MAX_IMAGE_BYTES {
            degrade_pending_image(&app, ctx.inner(), todo_id, &job, "下载图片超过单张上限");
            continue;
        }
        let Some(ext) = crate::clipboard_image::image_extension(&bytes) else {
            degrade_pending_image(&app, ctx.inner(), todo_id, &job, "下载内容不是受支持图片");
            continue;
        };
        // 预览在 storage 锁外生成（PATCH002.76：锁内只落盘，别堵 300ms 自动保存）
        let preview = crate::clipboard_image::generate_preview(&bytes);
        let orig_name = format!("pasted.{ext}");
        let outcome = match ctx.lock_storage() {
            Ok(storage) => storage.complete_pending_note_image(
                &images_dir,
                todo_id,
                job.block_id,
                &job.url,
                &bytes,
                Some(orig_name.as_str()),
                preview.as_deref(),
                &stamp,
            ),
            Err(err) => {
                crate::diag::warn_diag(&format!("图片后台任务取库锁失败（{}）：{err}", job.url));
                // 库锁失败：先让前端降级，下一次保存再落库
                emit_note_image_failed(&app, todo_id, job.block_id, &job.url);
                continue;
            }
        };
        match outcome {
            Ok(Some(block)) => {
                let view = to_block_view(block, &images_dir);
                if let Err(err) = app.emit(
                    "note-image-ready",
                    NoteImageReady {
                        todo_id,
                        block: view,
                        url: job.url.clone(),
                    },
                ) {
                    crate::diag::warn_diag(&format!("note-image-ready 广播失败：{err}"));
                }
            }
            Ok(None) => {
                // 用户已编辑/删除占位 → 不覆盖；刚落下的文件由清扫回收
            }
            Err(err) => {
                crate::diag::warn_diag(&format!("图片后台落库失败（{}）：{err}", job.url));
                degrade_pending_image(&app, ctx.inner(), todo_id, &job, "图片后台落库失败");
            }
        }
    }
}

/// 失败降级：占位文本 → URL（条件更新，用户已改则不动），并通知前端。
fn degrade_pending_image(
    app: &tauri::AppHandle,
    ctx: &AppContext,
    todo_id: i64,
    job: &PendingImageJob,
    reason: &str,
) {
    crate::diag::warn_diag(&format!("HTML 粘贴图片降级（{}）：{reason}", job.url));
    let updated = ctx
        .lock_storage()
        .ok()
        .and_then(|storage| {
            storage
                .fail_pending_note_image(todo_id, job.block_id, &job.url)
                .ok()
        })
        .unwrap_or(false);
    // updated = false 可能是用户已编辑/已删除；前端会按本地文本是否为占位自判
    if !updated {
        crate::diag::warn_diag(&format!(
            "图片占位未降级（用户已编辑/删除或库不可用）：{}",
            job.url
        ));
    }
    emit_note_image_failed(app, todo_id, job.block_id, &job.url);
}

/// 失败事件（前端把“下载中…”占位替换为 URL 文本）
fn emit_note_image_failed(app: &tauri::AppHandle, todo_id: i64, block_id: i64, url: &str) {
    if let Err(err) = app.emit(
        "note-image-failed",
        NoteImageFailed {
            todo_id,
            block_id,
            url: url.to_string(),
        },
    ) {
        crate::diag::warn_diag(&format!("note-image-failed 广播失败：{err}"));
    }
}

/// 图片段无法入块时的降级文案（PATCH002.46）：`data:` 载荷是 base64 巨串，落成文本块会变成
/// 一整块乱码 → 用短占位（沿 `note_copy_selection` 的超限占位口径）；其余（http(s)/相对
/// 路径）原样留作文本链接（`.25` 的下载链路补齐后此处会自然改走图块）
fn paste_fallback_text(src: &str) -> String {
    if src.starts_with("data:") {
        "🖼 图片".to_string()
    } else {
        src.to_string()
    }
}

/// `data:` URL → 字节 + 内层名提示（仅 `;base64,` 形态；其余形态 None）
fn decode_data_url(src: &str) -> Option<(Vec<u8>, Option<String>)> {
    use base64::Engine as _;
    let rest = src.strip_prefix("data:")?;
    let (meta, payload) = rest.split_once(',')?;
    if !meta.contains(";base64") {
        return None;
    }
    let mime = meta.split(';').next().unwrap_or("image/png");
    let ext = match mime {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        _ => "png",
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .ok()?;
    Some((bytes, Some(format!("pasted.{ext}"))))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::storage::Storage;

    /// 测试上下文：内存库（禁触真实用户数据）
    fn test_context() -> AppContext {
        AppContext {
            storage: Mutex::new(Storage::open_in_memory().expect("内存库必须可开")),
            settings: Mutex::new(crate::settings::WindowSettings::default()),
        }
    }

    #[test]
    fn add_then_list_returns_sorted_view() {
        let ctx = test_context();
        let a = todo_add_core("任务一", &ctx).expect("合法文本必须成功");
        let b = todo_add_core("任务二", &ctx).expect("合法文本必须成功");
        todo_toggle_core(a.id, &ctx).expect("刚添加的条目必须存在");
        let view = todo_list_core(&ctx).expect("读命令必须成功");
        let ids: Vec<i64> = view.iter().map(|v| v.item.id).collect();
        assert_eq!(ids, vec![b.id, a.id]);
    }

    #[test]
    fn add_blank_text_is_visible_error() {
        let ctx = test_context();
        let err = todo_add_core("   ", &ctx).expect_err("空文本必须被拒");
        assert!(matches!(err, CommandError::Todo(_)));
    }

    #[test]
    fn toggle_missing_id_is_visible_error() {
        let ctx = test_context();
        let err = todo_toggle_core(99, &ctx).expect_err("不存在条目必须报错");
        assert!(matches!(err, CommandError::Storage(_)));
    }

    #[test]
    fn remove_then_list_shrinks() {
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = todo_add_core("任务一", &ctx).expect("合法文本必须成功");
        todo_remove_core(item.id, &dir, &ctx).expect("刚添加的条目必须存在");
        assert!(todo_list_core(&ctx).expect("读命令必须成功").is_empty());
    }

    // —— PL010.3 新命令 ——

    #[test]
    fn rename_roundtrip_and_validation() {
        let ctx = test_context();
        let item = todo_add_core("旧名", &ctx).expect("合法文本必须成功");
        let renamed = todo_rename_core(item.id, " 新名 ", &ctx).expect("改名必须成功");
        assert_eq!(renamed.text, "新名", "rename 走 trim 存储语义");
        let err = todo_rename_core(item.id, "", &ctx).expect_err("空文本必须被拒");
        assert!(matches!(err, CommandError::Todo(_)));
    }

    #[test]
    fn add_trims_surrounding_whitespace() {
        // FIX003.10：add 与 rename 同规 trim 落库
        let ctx = test_context();
        let item = todo_add_core("  任务一  ", &ctx).expect("合法文本必须成功");
        assert_eq!(item.text, "任务一", "add 走 trim 存储语义");
    }

    #[test]
    fn list_attaches_age_level_from_storage_clock() {
        // 可变时钟真断言（FIX003.10）：建行于 T0 → 拨至 +25h 断言 Yellow → 拨至
        // +49h 断言 Red（命令层龄期裁决全链，替代旧恒 None 弱断言）
        let t0 = 1_700_000_000_000i64;
        let clock = Arc::new(std::sync::atomic::AtomicI64::new(t0));
        let ctx = AppContext {
            storage: Mutex::new(
                Storage::open_in_memory_with_now(Arc::new({
                    let clock = Arc::clone(&clock);
                    move || clock.load(std::sync::atomic::Ordering::SeqCst)
                }))
                .expect("内存库必须可开"),
            ),
            settings: Mutex::new(crate::settings::WindowSettings::default()),
        };
        let item = todo_add_core("老任务", &ctx).expect("合法文本必须成功");
        let hour = 3_600_000i64;
        clock.store(t0 + 25 * hour, std::sync::atomic::Ordering::SeqCst);
        let view = todo_list_core(&ctx).expect("读命令必须成功");
        assert_eq!(view[0].age_level, AgeLevel::Yellow, "超 24h 升 Yellow");
        clock.store(t0 + 49 * hour, std::sync::atomic::Ordering::SeqCst);
        let view = todo_list_core(&ctx).expect("读命令必须成功");
        assert_eq!(view[0].age_level, AgeLevel::Red, "超 48h 升 Red");
        let _ = item;
    }

    #[test]
    fn archive_list_purges_expired_before_view() {
        // PATCH001：归档读路径惰性清扫全链——超期条目在出视图前已删，前端永无中间态
        let t0 = 1_700_000_000_000i64;
        let clock = Arc::new(std::sync::atomic::AtomicI64::new(t0));
        let ctx = AppContext {
            storage: Mutex::new(
                Storage::open_in_memory_with_now(Arc::new({
                    let clock = Arc::clone(&clock);
                    move || clock.load(std::sync::atomic::Ordering::SeqCst)
                }))
                .expect("内存库必须可开"),
            ),
            settings: Mutex::new(crate::settings::WindowSettings::default()),
        };
        let item = todo_add_core("老条目", &ctx).expect("合法文本必须成功");
        todo_toggle_core(item.id, &ctx).expect("勾选必须成功"); // done_at = t0
        clock.store(
            t0 + crate::todo::ARCHIVE_RETENTION_MS + 1,
            std::sync::atomic::Ordering::SeqCst,
        );
        let dir = temp_images_dir();
        let view = todo_archive_list_core(&ctx, &dir).expect("归档读必须成功");
        assert!(view.is_empty(), "超期条目在出视图前已回收");
    }

    // —— PL013.2 重排命令 ——

    #[test]
    fn reorder_core_roundtrip_and_mismatch() {
        let ctx = test_context();
        let a = todo_add_core("甲", &ctx).expect("合法文本必须成功");
        let b = todo_add_core("乙", &ctx).expect("合法文本必须成功");
        let c = todo_add_core("丙", &ctx).expect("合法文本必须成功");
        todo_reorder_core(&[c.id, b.id, a.id], &ctx).expect("全量重排必须成功");
        let ids: Vec<i64> = todo_list_core(&ctx)
            .expect("读命令必须成功")
            .into_iter()
            .map(|v| v.item.id)
            .collect();
        assert_eq!(ids, vec![c.id, b.id, a.id]);
        // 部分集 / 幽灵 id：错误跨命令层可见
        assert!(todo_reorder_core(&[a.id], &ctx).is_err());
        assert!(todo_reorder_core(&[a.id, b.id, 99], &ctx).is_err());
    }

    // —— PL027 详情块命令 ——

    /// 临时图片目录（禁触真实 data/images）
    fn temp_images_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "capsule-todo-note-images-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("临时图片目录必须可建");
        dir
    }

    /// 独立夹具：image crate 编码 1×1 PNG（合法格式，供格式嗅探过闸）
    fn fixture_png() -> Vec<u8> {
        let img = image::RgbaImage::new(1, 1);
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .expect("夹具编码必须成功");
        out
    }

    fn input(id: i64, kind: NoteBlockKind, text: &str) -> NoteBlockInput {
        NoteBlockInput {
            id,
            kind,
            text: text.to_string(),
        }
    }

    #[test]
    fn note_blocks_save_roundtrip_and_validation() {
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = todo_add_core("任务", &ctx).expect("合法文本必须成功");
        let saved = note_blocks_save_core(
            item.id,
            &[
                input(0, NoteBlockKind::Text, "第一行"),
                input(0, NoteBlockKind::Text, "第二行"),
            ],
            &dir,
            &ctx,
        )
        .expect("保存必须成功");
        assert_eq!(saved.len(), 2);
        assert!(saved.iter().all(|b| b.kind == NoteBlockKind::Text));
        // has_note 口径（PL027：任一块非空）
        let view = todo_list_core(&ctx).expect("读命令必须成功");
        assert!(view[0].item.has_note, "有文本块 → has_note 亮");
        // 不变量：文本块含换行 → 可见报错
        let err = note_blocks_save_core(
            item.id,
            &[input(0, NoteBlockKind::Text, "a\nb")],
            &dir,
            &ctx,
        )
        .expect_err("换行必须被拒");
        assert!(matches!(err, CommandError::Todo(_)));
        // 不变量：仅图块（无文本块）→ 可见报错
        let err = note_blocks_save_core(item.id, &[input(0, NoteBlockKind::Image, "")], &dir, &ctx)
            .expect_err("缺文本块必须被拒");
        assert!(matches!(err, CommandError::Todo(_)));
        // 读回（文本块无图路径）
        let listed = note_blocks_list_core(item.id, &dir, &ctx).expect("读块必须成功");
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|b| b.image_path.is_none()));
    }

    #[test]
    fn note_insert_image_splits_and_reuses() {
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = todo_add_core("任务", &ctx).expect("合法文本必须成功");
        let saved = note_blocks_save_core(
            item.id,
            &[input(0, NoteBlockKind::Text, "前半后半")],
            &dir,
            &ctx,
        )
        .expect("保存必须成功");
        let captured = crate::clipboard_image::CapturedImage {
            bytes: fixture_png(),
            file_name: None,
        };
        // 块内选区/光标处切分：块变 3 个（before / 图 / after）
        let out =
            note_insert_image_core(item.id, saved[0].id, "前半", "后半", &captured, &dir, &ctx)
                .expect("插图必须成功");
        let NotePasteOutcome::Image { blocks } = out else {
            panic!("必须命中图");
        };
        assert_eq!(blocks.len(), 3, "块变 3 个");
        assert_eq!(blocks[0].text, "前半");
        assert_eq!(blocks[2].text, "后半");
        assert!(
            blocks[1]
                .image_path
                .as_deref()
                .unwrap_or("")
                .ends_with(".png"),
            "图块带绝对路径"
        );
        // 同字节第二次插入 → 复用同一文件（域内引用计数）
        let again = note_insert_image_core(item.id, blocks[0].id, "", "", &captured, &dir, &ctx)
            .expect("再插必须成功");
        let NotePasteOutcome::Image { blocks: second } = again else {
            panic!("必须命中图");
        };
        let paths: Vec<String> = second.iter().filter_map(|b| b.image_path.clone()).collect();
        assert_eq!(paths.len(), 2, "两个图块");
        assert_eq!(paths[0], paths[1], "同内容复用同一文件");
    }

    #[test]
    fn paste_html_degrades_unsupported_image_without_aborting() {
        // PATCH002.46：单段失败不得升格为整条失败（`validate_image_bytes?` 的旧行为）
        // ——网页含 SVG（image crate 不认）时：文本与正常图照进，SVG 降级为短占位
        use base64::Engine as _;
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = todo_add_core("任务", &ctx).expect("合法文本必须成功");
        let saved =
            note_blocks_save_core(item.id, &[input(0, NoteBlockKind::Text, "")], &dir, &ctx)
                .expect("保存必须成功");
        let png = base64::engine::general_purpose::STANDARD.encode(fixture_png());
        let svg = base64::engine::general_purpose::STANDARD
            .encode("<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>");
        let html = format!(
            "<p>前文</p><img src=\"data:image/svg+xml;base64,{svg}\">\
             <img src=\"data:image/png;base64,{png}\"><p>后文</p>"
        );
        let out = note_paste_html_core(item.id, saved[0].id, "", "", &html, &dir, &ctx)
            .expect("含 SVG 也不得整条失败");
        let NotePasteOutcome::Image { blocks } = out.outcome else {
            panic!("必须命中图");
        };
        let texts: Vec<&str> = blocks.iter().map(|b| b.text.as_str()).collect();
        assert!(texts.contains(&"前文"), "SVG 之前的文本要进");
        assert!(texts.contains(&"后文"), "SVG 之后的文本要进");
        assert!(
            texts.contains(&"🖼 图片"),
            "SVG 降级为短占位（不是 base64 巨串）"
        );
        assert!(
            blocks.iter().any(|b| b.image_path.is_some()),
            "正常 PNG 仍要入块"
        );
    }

    #[test]
    fn paste_html_rejects_overlong_note_before_write() {
        // FIX014.6：总长超限必须在写库前拒绝；原块保持不动
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = todo_add_core("任务", &ctx).expect("合法文本必须成功");
        let saved = note_blocks_save_core(
            item.id,
            &[input(0, NoteBlockKind::Text, "原文")],
            &dir,
            &ctx,
        )
        .expect("保存必须成功");
        let html = format!("<p>{}</p>", "字".repeat(crate::todo::MAX_NOTE_LEN + 1));
        let err = note_paste_html_core(item.id, saved[0].id, "", "", &html, &dir, &ctx)
            .expect_err("超过总长必须在写入前拒绝");
        assert!(matches!(err, CommandError::Todo(_)));
        let blocks = ctx
            .lock_storage()
            .expect("取库必须成功")
            .note_blocks(item.id)
            .expect("读块必须成功");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "原文", "拒绝后原内容不得被改动");
    }

    #[test]
    fn paste_html_returns_pending_jobs_without_network() {
        // PATCH002.76：http(s) 图只落占位 + pending，不在粘贴命令里联网
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = todo_add_core("任务", &ctx).expect("合法文本必须成功");
        let saved =
            note_blocks_save_core(item.id, &[input(0, NoteBlockKind::Text, "")], &dir, &ctx)
                .expect("保存必须成功");
        let html = "<p>前</p><img src=\"https://example.com/a.png?x=1&amp;y=2\"><p>后</p>";
        let out = note_paste_html_core(item.id, saved[0].id, "", "", html, &dir, &ctx)
            .expect("占位粘贴必须成功");
        let PasteHtmlResult { outcome, pending } = out;
        assert_eq!(pending.len(), 1, "一张 http 图对应一个后台任务");
        assert_eq!(
            pending[0].url, "https://example.com/a.png?x=1&y=2",
            "src 实体要解、URL 要规范化"
        );
        let NotePasteOutcome::Image { blocks } = outcome else {
            panic!("必须命中图占位");
        };
        assert!(
            blocks.iter().any(|b| {
                b.id == pending[0].block_id
                    && b.kind == NoteBlockKind::Text
                    && b.text == crate::storage::PENDING_NOTE_IMAGE_TEXT
            }),
            "pending.block_id 必须指向下载中占位块"
        );
        assert!(blocks.iter().any(|b| b.text == "前"));
        assert!(blocks.iter().any(|b| b.text == "后"));
    }

    #[test]
    fn note_reorder_core_roundtrip_and_mismatch() {
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = todo_add_core("任务", &ctx).expect("合法文本必须成功");
        let saved = note_blocks_save_core(
            item.id,
            &[
                input(0, NoteBlockKind::Text, "甲"),
                input(0, NoteBlockKind::Text, "乙"),
            ],
            &dir,
            &ctx,
        )
        .expect("保存必须成功");
        let ids: Vec<i64> = saved.iter().map(|b| b.id).collect();
        note_reorder_core(item.id, &[ids[1], ids[0]], &ctx).expect("重排必须成功");
        let listed = note_blocks_list_core(item.id, &dir, &ctx).expect("读块必须成功");
        assert_eq!(listed[0].text, "乙");
        assert!(
            note_reorder_core(item.id, &[ids[0]], &ctx).is_err(),
            "部分集拒绝"
        );
    }

    #[test]
    fn note_open_image_rejects_text_block_and_missing() {
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = todo_add_core("任务", &ctx).expect("合法文本必须成功");
        let saved = note_blocks_save_core(
            item.id,
            &[input(0, NoteBlockKind::Text, "文字")],
            &dir,
            &ctx,
        )
        .expect("保存必须成功");
        let err = note_open_image_core(saved[0].id, &dir, &ctx).expect_err("文本块必须报错");
        assert!(matches!(err, CommandError::Todo(ref msg) if msg.contains("不是图片块")));
        assert!(
            note_open_image_core(999, &dir, &ctx).is_err(),
            "不存在块报错"
        );
    }

    // —— PATCH002.71：复制载荷（锁外构建、纯函数可直测）——

    #[test]
    fn copy_payload_text_only_has_no_image_formats() {
        let dir = temp_images_dir();
        let blocks = [NoteCopyBlock {
            kind: NoteBlockKind::Text,
            text: "甲".to_string(),
            block_id: 0,
        }];
        let payload = note_copy_payload(&blocks, &[None], &dir).expect("构建必须成功");
        assert_eq!(payload.text, "甲");
        assert!(
            payload.html.contains("<!--StartFragment-->甲"),
            "片段必须进 CF_HTML"
        );
        assert!(payload.first.is_none(), "纯文本不带图片格式");
    }

    #[test]
    fn copy_payload_inlines_image_and_keeps_png_passthrough() {
        let dir = temp_images_dir();
        let sub = dir.join("note_test_1");
        std::fs::create_dir_all(&sub).expect("子目录必须可建");
        let png = fixture_png();
        std::fs::write(sub.join("a.png"), &png).expect("夹具必须可写");
        let blocks = [
            NoteCopyBlock {
                kind: NoteBlockKind::Text,
                text: "前".to_string(),
                block_id: 0,
            },
            NoteCopyBlock {
                kind: NoteBlockKind::Image,
                text: String::new(),
                block_id: 7,
            },
        ];
        let rels = [None, Some("note_test_1/a.png".to_string())];
        let payload = note_copy_payload(&blocks, &rels, &dir).expect("构建必须成功");
        assert!(payload.html.contains("data:image/png;base64,"), "图要内联");
        let first = payload.first.expect("首图必须解码");
        assert_eq!(first.png, png, "PNG 源必须原样直通");
        assert_eq!((first.rgba.0, first.rgba.1), (1, 1), "像素尺寸对");
    }

    #[test]
    fn copy_payload_missing_file_degrades_to_placeholder() {
        let dir = temp_images_dir();
        let blocks = [NoteCopyBlock {
            kind: NoteBlockKind::Image,
            text: String::new(),
            block_id: 9,
        }];
        let rels = [Some("note_missing/a.png".to_string())];
        let payload = note_copy_payload(&blocks, &rels, &dir).expect("缺文件不中断");
        assert!(payload.html.contains("🖼 图片"), "缺文件降级占位");
        assert!(payload.first.is_none(), "无图不带图片格式");
    }
}
