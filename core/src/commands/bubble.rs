//! 气泡命令：捕获剪贴板/列表/复制回/删除/清空（操作即落库，单一事实源 = db）。
//! clipboard 读写留命令薄壳（不可注入直测），文本链路核心抽自由函数直测内存库。

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use super::{AppContext, CommandError};
use crate::bubble::{validate_bubble_text, BubbleError, BubbleItem, BubbleSnapshot};
use crate::storage::BubbleAddOutcome;

/// 热键触发路径（PL015.5）：热键线程直调（实现注记：未走 run_on_main_thread——
/// 捕获流程仅读剪贴板 + 入库，AppContext 有锁保护、剪贴板插件跨线程安全，
/// 行为等价，沿 PL003.3 先例）。Windows 走圈选直达编排（PL016.3），其余平台
/// 直读剪贴板；全程失败落日志静默（反馈静默定案：桌面常驻面板即所见）
pub fn bubble_capture_from_clipboard_quiet(app: &AppHandle) {
    #[cfg(target_os = "windows")]
    {
        capture_quiet_windows(app);
    }
    #[cfg(not(target_os = "windows"))]
    {
        capture_quiet_direct(app);
    }
}

/// 图片气泡占位文案（PL025 按来源分派）：文件来源 = `🖼️ 图片 <文件名>`；
/// 截图来源 = `🖼️ 截图 {yyMMddHHmmss}`（emoji 🖼️ = U+1F5BC + U+FE0F）
pub fn image_label(file_name: Option<&str>) -> String {
    match file_name {
        Some(name) => format!("🖼️ 图片 {name}"),
        None => screenshot_label(),
    }
}

/// 截图占位文案 `🖼️ 截图 {yyMMddHHmmss}`
pub fn screenshot_label() -> String {
    format!("🖼️ 截图 {}", local_stamp())
}

/// 本地时间戳 `yyMMddHHmmss`（PL025 命名与标签共用）：Windows GetLocalTime 直连取本地
/// 时区（零依赖，kernel32 薄壳同 hotkey/capture FFI 先例）；非 Windows 退 UTC civil 推算
#[cfg(target_os = "windows")]
pub fn local_stamp() -> String {
    #[repr(C)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        millis: u16,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetLocalTime(out: *mut SystemTime);
    }
    let mut t = SystemTime {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        millis: 0,
    };
    unsafe { GetLocalTime(&mut t) };
    format!(
        "{:02}{:02}{:02}{:02}{:02}{:02}",
        t.year % 100,
        t.month,
        t.day,
        t.hour,
        t.minute,
        t.second
    )
}

/// epoch 天数 → (年, 月, 日)（Howard Hinnant civil 算法，延后平台 UTC 标签用）
#[cfg(not(target_os = "windows"))]
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 本地时间戳（非 Windows：UTC 推算，延后平台占位实现）
#[cfg(not(target_os = "windows"))]
pub fn local_stamp() -> String {
    let secs = crate::storage::system_now() / 1000;
    let sod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    format!(
        "{:02}{:02}{:02}{:02}{:02}{:02}",
        y % 100,
        m,
        d,
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

/// 捕获剪贴板图片为新气泡（PL025 落盘化）：格式嗅探校验 + 落盘入库（按内容去重）。
/// ext 由文件名推（截图 = png）；source = 'file' | 'screenshot'
pub fn bubble_capture_image_core(
    images_dir: &std::path::Path,
    captured: &crate::clipboard_image::CapturedImage,
    ctx: &AppContext,
) -> Result<BubbleCaptureOutcome, CommandError> {
    crate::clipboard_image::validate_image_bytes(&captured.bytes)?;
    let label = image_label(captured.file_name.as_deref());
    let stamp = local_stamp();
    let storage = ctx.lock_storage()?;
    match storage.add_image_bubble(
        images_dir,
        &label,
        &captured.bytes,
        captured.file_name.as_deref(),
        &stamp,
    )? {
        BubbleAddOutcome::Added(item) => Ok(BubbleCaptureOutcome::Added { item }),
        BubbleAddOutcome::Duplicate => Ok(BubbleCaptureOutcome::Duplicate),
    }
}

/// 图片捕获收尾（热键路径）：落盘入库 + 广播；失败静默落日志（反馈静默定案
/// 与文本路径对齐）
fn finish_quiet_capture_image(app: &AppHandle, captured: &crate::clipboard_image::CapturedImage) {
    let ctx = app.state::<AppContext>();
    let images_dir = match crate::paths::images_dir() {
        Ok(d) => d,
        Err(err) => {
            eprintln!("热键图片捕获失败：图片目录解析失败：{err}");
            return;
        }
    };
    match bubble_capture_image_core(&images_dir, captured, &ctx) {
        Ok(BubbleCaptureOutcome::Added { .. }) => {
            if let Err(err) = app.emit("bubble-changed", ()) {
                eprintln!("bubble-changed 事件发送失败：{err}");
            }
        }
        Err(err) => eprintln!("热键图片捕获失败：{err}"),
        Ok(BubbleCaptureOutcome::Duplicate) => {}
    }
}

/// Windows 圈选直达编排（PL016.3，PL024 图优先分叉；PL024.8x 变化优先重构；
/// PL025 图片携带来源名）：**变化优先 + 快照兜底**——先快照 {文本, 图}，合成
/// Ctrl+C 让前台复制用户选中的项（文件/文本），轮询只认相对快照的新内容；超时
/// 无变化才兜底收快照图（截图场景）。不再「剪贴板有图就直接收」——那会让旧图
/// 短路掉用户新选中的项
#[cfg(target_os = "windows")]
fn capture_quiet_windows(app: &AppHandle) {
    let before_text = app.clipboard().read_text().ok();
    let before_image = crate::clipboard_image::read_clipboard_image();
    let before_image_bytes = before_image.as_ref().map(|c| c.bytes.clone());
    // 终端边界守卫（PL016.4）：conhost/Windows Terminal 无选区时 Ctrl+C =
    // 中断信号（SIGINT），不能合成——但快照已有图（终端里截图）仍可直接收
    if crate::capture::foreground_is_console() {
        if let Some(captured) = &before_image {
            finish_quiet_capture_image(app, captured);
        }
        return;
    }
    crate::capture::synthesize_ctrl_c();
    let start = std::time::Instant::now();
    let picked = crate::capture::wait_clipboard_capture(
        before_text.as_deref(),
        before_image_bytes.as_deref(),
        crate::capture::WAIT_TIMEOUT_MS,
        crate::capture::POLL_INTERVAL_MS,
        || start.elapsed().as_millis() as u64,
        || app.clipboard().read_text().ok(),
        crate::clipboard_image::read_clipboard_image,
    );
    let Some(hit) = picked else {
        return;
    };
    // 恢复原剪贴板（圈选复制动作污染了用户剪贴板；失败落日志不阻断——捕获已
    // 成立，且下次粘贴拿到的是刚圈选的文本损失为零）；原本无文本（None）无可恢复
    if let Some(old) = &before_text {
        if let Err(err) = app.clipboard().write_text(old) {
            eprintln!("热键捕获：原剪贴板恢复失败：{err}");
        }
    }
    match hit {
        crate::capture::CaptureHit::Text(text) => finish_quiet_capture(app, &text),
        crate::capture::CaptureHit::Image { bytes, file_name } => {
            let captured = crate::clipboard_image::CapturedImage { bytes, file_name };
            finish_quiet_capture_image(app, &captured);
        }
    }
}

/// 非 Windows 直读剪贴板捕获（无合成能力平台保持旧行为）
#[cfg(not(target_os = "windows"))]
fn capture_quiet_direct(app: &AppHandle) {
    match app.clipboard().read_text() {
        Ok(text) => finish_quiet_capture(app, &text),
        Err(err) => eprintln!("热键捕获：剪贴板读取失败（{err}）"),
    }
}

/// 捕获收尾（两平台共用）：校验 + 去重 + 入库；新增发 bubble-changed 驱动失焦
/// 实时刷新（PL016.1，emit 失败落日志——热键反馈静默定案不变；Duplicate 列表
/// 未变不发；Err 静默落日志）
fn finish_quiet_capture(app: &AppHandle, text: &str) {
    let ctx = app.state::<AppContext>();
    match bubble_capture_core(text, &ctx) {
        Ok(BubbleCaptureOutcome::Added { .. }) => {
            if let Err(err) = app.emit("bubble-changed", ()) {
                eprintln!("bubble-changed 事件发送失败：{err}");
            }
        }
        Ok(BubbleCaptureOutcome::Duplicate) => {}
        Err(err) => eprintln!("热键捕获失败：{err}"),
    }
}

/// 捕获结果（跨 IPC，PL015.5 去重）：added 携带新条目；duplicate 供前端捕获钮切换
/// "重复捕获，无效！"占字态（热键路径静默）
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BubbleCaptureOutcome {
    Added { item: BubbleItem },
    Duplicate,
}

impl BubbleCaptureOutcome {
    /// 测试辅助：取新增条目（Duplicate panic——调用方应先分支；生产路径一律 match）
    #[cfg(test)]
    pub fn added_item(self) -> BubbleItem {
        match self {
            BubbleCaptureOutcome::Added { item } => item,
            BubbleCaptureOutcome::Duplicate => panic!("气泡捕获意外重复"),
        }
    }
}

/// 捕获剪贴板为新气泡（PL024 图优先分叉：剪贴板有图捕获图片气泡，无图回落
/// 文本链路；文本无/空文本严格报错，重复内容返回 Duplicate 不入库——PL015.5
/// 去重与热键路径同规）
#[tauri::command]
pub fn bubble_capture(
    app: AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<BubbleCaptureOutcome, CommandError> {
    if let Some(captured) = crate::clipboard_image::read_clipboard_image() {
        let images_dir = crate::paths::images_dir()
            .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
        return bubble_capture_image_core(&images_dir, &captured, &ctx);
    }
    let text = app
        .clipboard()
        .read_text()
        .map_err(|err| CommandError::Clipboard(format!("剪贴板读取失败：{err}")))?;
    bubble_capture_core(&text, &ctx)
}

/// bubble_capture 核心实现：校验 + 去重裁决 + 入库
pub fn bubble_capture_core(
    text: &str,
    ctx: &AppContext,
) -> Result<BubbleCaptureOutcome, CommandError> {
    validate_bubble_text(text)?;
    let storage = ctx.lock_storage()?;
    match storage.add_bubble(text.trim())? {
        BubbleAddOutcome::Added(item) => Ok(BubbleCaptureOutcome::Added { item }),
        BubbleAddOutcome::Duplicate => Ok(BubbleCaptureOutcome::Duplicate),
    }
}

/// 气泡列表（sort_order 升序拖拽序，新捕获排头插入）
#[tauri::command]
pub fn bubble_list(ctx: State<'_, AppContext>) -> Result<BubbleSnapshot, CommandError> {
    bubble_list_core(&ctx)
}

/// bubble_list 核心实现：出快照（升序拖拽序；满额提醒显隐由前端本地阈值裁决，
/// FIX004.23 删 Rust 侧 remind 死值）
pub fn bubble_list_core(ctx: &AppContext) -> Result<BubbleSnapshot, CommandError> {
    let storage = ctx.lock_storage()?;
    let items = storage.list_bubbles()?;
    Ok(BubbleSnapshot { items })
}

/// 气泡图片落盘绝对路径（详情板 asset 协议直读；**列表载荷不带图**——PL025 载荷
/// 落盘化，本命令只在 kind = Image 时被前端调用）。文本气泡/不存在严格报错；文件
/// 缺失（数据目录被清/损坏）→ 专用文案（前端失效态）
#[tauri::command]
pub fn bubble_get_image_path(id: i64, ctx: State<'_, AppContext>) -> Result<String, CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    bubble_get_image_path_core(id, &images_dir, &ctx)
}

/// bubble_get_image_path 核心实现：image_file → 绝对路径（缺失严格报错）
pub fn bubble_get_image_path_core(
    id: i64,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<String, CommandError> {
    let storage = ctx.lock_storage()?;
    let (rel, _source) = storage
        .get_bubble_image_file(id)?
        .ok_or(CommandError::Bubble(
            BubbleError::UnsupportedFormat.to_string(),
        ))?;
    let abs = images_dir.join(&rel);
    if !abs.is_file() {
        return Err(CommandError::Bubble("图片文件已移动或删除".to_string()));
    }
    // PL025 预览优先：大图有 preview.png 则回预览（详情秒开）；无则原图
    if let Some(preview) = abs.parent().map(|dir| dir.join("preview.png")) {
        if preview.is_file() {
            return Ok(preview.to_string_lossy().into_owned());
        }
    }
    Ok(abs.to_string_lossy().into_owned())
}

/// 双击详情图 → 系统默认程序打开**原图**（PL025）：解析原图绝对路径（非预览）→ ShellExecuteW
#[tauri::command]
pub fn bubble_open_image(id: i64, ctx: State<'_, AppContext>) -> Result<(), CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    bubble_open_image_core(id, &images_dir, &ctx)
}

/// bubble_open_image 核心实现：原图绝对路径 → 系统默认程序打开（缺失严格报错）
pub fn bubble_open_image_core(
    id: i64,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    let (rel, _source) = storage
        .get_bubble_image_file(id)?
        .ok_or(CommandError::Bubble(
            BubbleError::UnsupportedFormat.to_string(),
        ))?;
    let abs = images_dir.join(&rel);
    if !abs.is_file() {
        return Err(CommandError::Bubble("图片文件已移动或删除".to_string()));
    }
    open_path_with_default(&abs).map_err(CommandError::Bubble)
}

/// 用系统默认程序打开文件（PL025）：Windows `ShellExecuteW`（返回 >32 = 成功约定）；
/// 非 Windows 报暂不支持（延后基线）
#[cfg(target_os = "windows")]
pub fn open_path_with_default(path: &std::path::Path) -> Result<(), String> {
    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: isize,
            op: *const u16,
            file: *const u16,
            params: *const u16,
            dir: *const u16,
            show: i32,
        ) -> isize;
    }
    let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
    let op = wide("open");
    let file = wide(&path.to_string_lossy());
    const SW_SHOWNORMAL: i32 = 1;
    let ret = unsafe {
        ShellExecuteW(
            0,
            op.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if ret <= 32 {
        return Err(format!("系统打开失败（ShellExecute 返回 {ret}）"));
    }
    Ok(())
}

/// 非 Windows：打开原图暂不支持
#[cfg(not(target_os = "windows"))]
pub fn open_path_with_default(_path: &std::path::Path) -> Result<(), String> {
    Err("当前平台暂不支持".to_string())
}

/// 复制回取数路由结果（PL025 按 kind + 来源分派；不跨 IPC 无需 Serialize）
#[derive(Debug)]
pub enum BubbleCopyPayload {
    /// 文本气泡原文
    Text(String),
    /// 文件图（来源 'file'）→ 把落盘副本放剪贴板（CF_HDROP）
    ImageFile(std::path::PathBuf),
    /// 截图（来源 'screenshot'）→ PNG 字节（写回三格式由剪贴板 FFI 层裁决）
    ImagePng(Vec<u8>),
}

/// 复制回取数路由核心（直测面；写剪贴板动作留命令薄壳不直测）：文本回原文；
/// 图片按来源分叉（file → 副本绝对路径；screenshot → PNG 字节）；文件缺失/列缺席
/// 严格报错（前端失效态）
pub fn bubble_copy_payload_core(
    id: i64,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<BubbleCopyPayload, CommandError> {
    let storage = ctx.lock_storage()?;
    let item = storage.get_bubble(id)?;
    match item.kind {
        crate::bubble::BubbleKind::Text => Ok(BubbleCopyPayload::Text(item.text)),
        crate::bubble::BubbleKind::Image => {
            let (rel, source) = storage
                .get_bubble_image_file(id)?
                .ok_or(CommandError::Bubble(
                    BubbleError::UnsupportedFormat.to_string(),
                ))?;
            let abs = images_dir.join(&rel);
            if !abs.is_file() {
                return Err(CommandError::Bubble("图片文件已移动或删除".to_string()));
            }
            if source == "file" {
                Ok(BubbleCopyPayload::ImageFile(abs))
            } else {
                let bytes = std::fs::read(&abs)
                    .map_err(|err| CommandError::Storage(format!("图片读取失败：{err}")))?;
                Ok(BubbleCopyPayload::ImagePng(bytes))
            }
        }
    }
}

/// 复制气泡内容回剪贴板（捕获→粘贴走→清理闭环的回程；PL025 按 kind + 来源分叉——
/// 文本走插件写文本；文件图走 CF_HDROP 文件引用（原样）；截图走 Win32 三格式写入
/// [PNG 注册格式 + CF_DIB + CF_DIBV5]，用户主动覆盖剪贴板无恢复逻辑）
#[tauri::command]
pub fn bubble_copy(
    id: i64,
    app: AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    match bubble_copy_payload_core(id, &images_dir, &ctx)? {
        BubbleCopyPayload::Text(text) => app
            .clipboard()
            .write_text(&text)
            .map_err(|err| CommandError::Clipboard(format!("写入剪贴板失败：{err}")))?,
        BubbleCopyPayload::ImageFile(path) => {
            crate::clipboard_image::write_clipboard_files(&[path.as_path()])
                .map_err(|err| CommandError::Clipboard(format!("文件写回剪贴板失败：{err}")))?
        }
        BubbleCopyPayload::ImagePng(bytes) => crate::clipboard_image::write_clipboard_image(&bytes)
            .map_err(|err| CommandError::Clipboard(format!("图片写回剪贴板失败：{err}")))?,
    }
    Ok(())
}

/// 删除单条气泡（命令薄壳：解析图片目录 + 调核心 + 广播；不存在严格报错）
#[tauri::command]
pub fn bubble_remove(
    id: i64,
    app: AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    bubble_remove_core(id, &images_dir, &ctx)?;
    emit_bubble_changed(&app);
    Ok(())
}

/// bubble_remove 核心实现：删除条目 + 删落盘文件（best-effort）
pub fn bubble_remove_core(
    id: i64,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    if let Some(rel) = storage.remove_bubble(id)? {
        remove_image_folder(images_dir, &rel);
    }
    Ok(())
}

/// 一键清空气泡，返回清除条数（满 5 提醒后的清理出口）
#[tauri::command]
pub fn bubble_clear(app: AppHandle, ctx: State<'_, AppContext>) -> Result<usize, CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    let removed = bubble_clear_core(&images_dir, &ctx)?;
    emit_bubble_changed(&app);
    Ok(removed)
}

/// bubble_clear 核心实现：清空 + 删全部落盘文件（best-effort）
pub fn bubble_clear_core(
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<usize, CommandError> {
    let storage = ctx.lock_storage()?;
    let (removed, rels) = storage.clear_bubbles()?;
    for rel in rels {
        remove_image_folder(images_dir, &rel);
    }
    Ok(removed)
}

/// 递归删气泡图片文件夹（best-effort：失败落日志不阻断删行；不存在忽略）
fn remove_image_folder(images_dir: &std::path::Path, folder: &str) {
    let path = images_dir.join(folder);
    if path.is_dir() {
        if let Err(err) = std::fs::remove_dir_all(&path) {
            eprintln!("图片文件夹删除失败（{path:?}）：{err}");
        }
    }
}

/// 广播气泡变更（PL025.8.2：详情删除/清空后驱动气泡页列表重拉；emit 失败落日志不阻断，沿捕获同规）
fn emit_bubble_changed(app: &AppHandle) {
    if let Err(err) = app.emit("bubble-changed", ()) {
        eprintln!("bubble-changed 事件发送失败：{err}");
    }
}

/// 气泡重排（拖拽落点提交）：ids = 全量气泡的目标顺序（一致性校验在 storage 层）
#[tauri::command]
pub fn bubble_reorder(ids: Vec<i64>, ctx: State<'_, AppContext>) -> Result<(), CommandError> {
    bubble_reorder_core(&ids, &ctx)
}

/// bubble_reorder 核心实现：透传 storage.reorder_bubbles（校验+事务在存储层）
pub fn bubble_reorder_core(ids: &[i64], ctx: &AppContext) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.reorder_bubbles(ids)?)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

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
    fn capture_trims_and_persists() {
        let ctx = test_context();
        let item = bubble_capture_core("  复制的内容  ", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        let snapshot = bubble_list_core(&ctx).expect("读命令必须成功");
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].id, item.id);
        assert_eq!(snapshot.items[0].text, "复制的内容");
    }

    #[test]
    fn capture_image_persists_with_label_and_dedupes() {
        // 图片捕获落盘入库 + 截图占位文案（🖼️ 截图 + 12 位数字）+ 同图去重（PL025）
        let ctx = test_context();
        let dir = temp_images_dir();
        let captured = screenshot_capture(fixture_png());
        let a = bubble_capture_image_core(&dir, &captured, &ctx)
            .expect("合法图片必须成功")
            .added_item();
        let dup = bubble_capture_image_core(&dir, &captured, &ctx).expect("去重判定必须成功");
        assert!(
            matches!(dup, BubbleCaptureOutcome::Duplicate),
            "同图第二次应判重复"
        );
        let digits = a
            .text
            .chars()
            .skip("🖼️ 截图 ".chars().count())
            .collect::<String>();
        assert_eq!(digits.len(), 12, "yyMMddHHmmss 连写 12 位");
        assert!(digits.chars().all(|c| c.is_ascii_digit()), "占位尾全数字");
        let view = bubble_list_core(&ctx).expect("读命令必须成功");
        assert_eq!(view.items.len(), 1, "同图去重后仅一条");
        assert!(view
            .items
            .iter()
            .all(|it| it.kind == crate::bubble::BubbleKind::Image));
    }

    #[test]
    fn capture_image_file_source_labels_with_name() {
        // 文件来源 → 文案 `🖼️ 图片 <文件名>` + image_source='file'
        let ctx = test_context();
        let dir = temp_images_dir();
        let captured = crate::clipboard_image::CapturedImage {
            bytes: fixture_png(),
            file_name: Some("示例.png".to_string()),
        };
        let item = bubble_capture_image_core(&dir, &captured, &ctx)
            .expect("合法图片必须成功")
            .added_item();
        assert_eq!(item.text, "🖼️ 图片 示例.png");
        let (_, source) = ctx
            .lock_storage()
            .expect("锁必须可拿")
            .get_bubble_image_file(item.id)
            .expect("读取必须成功")
            .expect("应有文件");
        assert_eq!(source, "file");
    }

    #[test]
    fn capture_image_rejects_garbage_bytes() {
        // 格式嗅探在命令层可见（严格抛错主线；Bubble 变体承载文案）
        let ctx = test_context();
        let dir = temp_images_dir();
        let err = bubble_capture_image_core(&dir, &screenshot_capture(b"garbage".to_vec()), &ctx)
            .expect_err("垃圾字节必须被拒");
        assert!(
            matches!(err, CommandError::Bubble(ref msg) if msg.contains("图片格式暂不支持")),
            "文案必须可见且指向格式"
        );
    }

    #[test]
    fn get_image_path_and_strict_errors() {
        // 图片气泡 → 落盘绝对路径；文本气泡/不存在严格报错
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = bubble_capture_image_core(&dir, &screenshot_capture(fixture_png()), &ctx)
            .expect("合法图片必须成功")
            .added_item();
        let path = bubble_get_image_path_core(item.id, &dir, &ctx).expect("取路径必须成功");
        assert!(path.ends_with(".png"), "路径 = 落盘文件");
        assert!(std::path::Path::new(&path).is_file(), "路径文件存在");
        let text_item = bubble_capture_core("文本", &ctx)
            .expect("写入必须成功")
            .added_item();
        let err =
            bubble_get_image_path_core(text_item.id, &dir, &ctx).expect_err("文本气泡必须报错");
        assert!(matches!(err, CommandError::Bubble(ref msg) if msg.contains("图片格式暂不支持")));
        assert!(bubble_get_image_path_core(99, &dir, &ctx).is_err());
    }

    /// 临时图片目录（测试用，落系统临时目录，禁触真实 data/images）
    fn temp_images_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "capsule-todo-cmd-images-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("临时图片目录必须可建");
        dir
    }

    /// 截图来源捕获（file_name=None）
    fn screenshot_capture(bytes: Vec<u8>) -> crate::clipboard_image::CapturedImage {
        crate::clipboard_image::CapturedImage {
            bytes,
            file_name: None,
        }
    }

    /// 独立夹具：image crate 直接编码 1×1 PNG（合法格式，供格式嗅探过闸）
    fn fixture_png() -> Vec<u8> {
        let img = image::RgbaImage::new(1, 1);
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .expect("夹具编码必须成功");
        out
    }

    #[test]
    fn capture_duplicate_outcome_and_list_unchanged() {
        // PL015.5 去重：重复文本返回 Duplicate（非错误），列表不变；trim 后命中查重
        let ctx = test_context();
        bubble_capture_core("重复片段", &ctx)
            .expect("首次必须成功")
            .added_item();
        let second = bubble_capture_core("  重复片段  ", &ctx).expect("重复不是错误");
        assert!(matches!(second, BubbleCaptureOutcome::Duplicate));
        let snapshot = bubble_list_core(&ctx).expect("读命令必须成功");
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].text, "重复片段");
    }

    #[test]
    fn capture_blank_is_visible_error() {
        let ctx = test_context();
        let err = bubble_capture_core("   ", &ctx).expect_err("空文本必须被拒");
        // FIX009.8：校验拒绝归 Bubble 专属变体（原错桶 Clipboard 已归位）
        assert!(matches!(err, CommandError::Bubble(_)));
    }

    #[test]
    fn text_core_reads_back_and_missing_errors() {
        // PL025 取数路由面：文本 → Text；截图 → ImagePng；不存在 → NotFound
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = bubble_capture_core("片段", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        assert!(matches!(
            bubble_copy_payload_core(item.id, &dir, &ctx).expect("回读必须成功"),
            BubbleCopyPayload::Text(ref t) if t == "片段"
        ));
        let img = bubble_capture_image_core(&dir, &screenshot_capture(fixture_png()), &ctx)
            .expect("图片捕获必须成功")
            .added_item();
        assert!(matches!(
            bubble_copy_payload_core(img.id, &dir, &ctx).expect("取图必须成功"),
            BubbleCopyPayload::ImagePng(ref png) if png == &fixture_png()
        ));
        let err = bubble_copy_payload_core(99, &dir, &ctx).expect_err("不存在必须报错");
        assert!(matches!(err, CommandError::Storage(_)));
    }

    #[test]
    fn remove_and_clear_shrink_list() {
        let ctx = test_context();
        let a = bubble_capture_core("一", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        bubble_capture_core("二", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        let dir = temp_images_dir();
        bubble_remove_core(a.id, &dir, &ctx).expect("刚添加的条目必须存在");
        assert_eq!(
            bubble_list_core(&ctx).expect("读命令必须成功").items.len(),
            1
        );
        let cleared = bubble_clear_core(&dir, &ctx).expect("清空必须成功");
        assert_eq!(cleared, 1);
        let snapshot = bubble_list_core(&ctx).expect("读命令必须成功");
        assert!(snapshot.items.is_empty());
    }

    // —— PL013.2 重排命令 ——

    #[test]
    fn reorder_core_roundtrip_and_mismatch() {
        let ctx = test_context();
        let a = bubble_capture_core("一", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        let b = bubble_capture_core("二", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        let c = bubble_capture_core("三", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        bubble_reorder_core(&[c.id, a.id, b.id], &ctx).expect("全量重排必须成功");
        let ids: Vec<i64> = bubble_list_core(&ctx)
            .expect("读命令必须成功")
            .items
            .into_iter()
            .map(|it| it.id)
            .collect();
        assert_eq!(ids, vec![c.id, a.id, b.id]);
        // 部分集 / 幽灵 id：错误跨命令层可见
        assert!(bubble_reorder_core(&[a.id], &ctx).is_err());
        assert!(bubble_reorder_core(&[a.id, 99], &ctx).is_err());
    }
}
