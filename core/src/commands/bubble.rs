//! 气泡命令：捕获剪贴板/列表/复制回/删除/清空（操作即落库，单一事实源 = db）。
//! clipboard 读写留命令薄壳（不可注入直测），文本链路核心抽自由函数直测内存库。

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use super::{AppContext, CommandError};
use crate::bubble::{
    validate_bubble_text, validate_image_png, BubbleError, BubbleItem, BubbleSnapshot,
};
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

/// 图片气泡占位文案（用户定案格式 `🖼 截图 {yyMMddHHmmss}` 连写）：
/// Windows GetLocalTime 直连取本地时区（零依赖，kernel32 薄壳同 hotkey/capture
/// FFI 先例）；延后平台退 UTC 推算（civil 算法，标签可读性达标非时区承诺）
#[cfg(target_os = "windows")]
pub fn image_bubble_label() -> String {
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
        "🖼 截图 {:02}{:02}{:02}{:02}{:02}{:02}",
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

/// 图片气泡占位文案（非 Windows：UTC 推算，延后平台占位实现）
#[cfg(not(target_os = "windows"))]
pub fn image_bubble_label() -> String {
    let secs = crate::storage::system_now() / 1000;
    let sod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    format!(
        "🖼 截图 {:02}{:02}{:02}{:02}{:02}{:02}",
        y % 100,
        m,
        d,
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

/// 捕获剪贴板图片为新气泡（PNG 校验 + 入库；**无去重**——每次截图都是新内容，
/// PL024 定案）
pub fn bubble_capture_image_core(
    png: &[u8],
    ctx: &AppContext,
) -> Result<BubbleCaptureOutcome, CommandError> {
    validate_image_png(png)?;
    let label = image_bubble_label();
    let storage = ctx.lock_storage()?;
    match storage.add_image_bubble(&label, png)? {
        BubbleAddOutcome::Added(item) => Ok(BubbleCaptureOutcome::Added { item }),
        BubbleAddOutcome::Duplicate => Ok(BubbleCaptureOutcome::Duplicate),
    }
}

/// 图片捕获收尾（热键路径）：校验入库 + 广播；失败静默落日志（反馈静默定案
/// 与文本路径对齐；图片路径无 Duplicate 分支）
fn finish_quiet_capture_image(app: &AppHandle, png: &[u8]) {
    let ctx = app.state::<AppContext>();
    match bubble_capture_image_core(png, &ctx) {
        Ok(BubbleCaptureOutcome::Added { .. }) => {
            if let Err(err) = app.emit("bubble-changed", ()) {
                eprintln!("bubble-changed 事件发送失败：{err}");
            }
        }
        Err(err) => eprintln!("热键图片捕获失败：{err}"),
        Ok(BubbleCaptureOutcome::Duplicate) => {}
    }
}

/// Windows 圈选直达编排（PL016.3，PL024 图优先分叉；PL024.8x 变化优先重构）：
/// **变化优先 + 快照兜底**——先快照 {文本, 图}，合成 Ctrl+C 让前台复制用户选中的
/// 项（文件/文本），轮询只认相对快照的新内容（新图或新文本）；超时无变化才兜底
/// 收快照图（截图场景：合成 Ctrl+C 是 no-op，快照图即截图）。不再「剪贴板有图就
/// 直接收」——那会让剪贴板里的旧图短路掉用户新选中的项
#[cfg(target_os = "windows")]
fn capture_quiet_windows(app: &AppHandle) {
    let before_text = app.clipboard().read_text().ok();
    let before_image = crate::clipboard_image::read_clipboard_image();
    // 终端边界守卫（PL016.4）：conhost/Windows Terminal 无选区时 Ctrl+C =
    // 中断信号（SIGINT），不能合成——但快照已有图（终端里截图）仍可直接收
    if crate::capture::foreground_is_console() {
        if let Some(png) = before_image {
            finish_quiet_capture_image(app, &png);
        }
        return;
    }
    crate::capture::synthesize_ctrl_c();
    let start = std::time::Instant::now();
    let picked = crate::capture::wait_clipboard_capture(
        before_text.as_deref(),
        before_image.as_deref(),
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
        crate::capture::CaptureHit::Image(png) => finish_quiet_capture_image(app, &png),
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
    if let Some(png) = crate::clipboard_image::read_clipboard_image() {
        return bubble_capture_image_core(&png, &ctx);
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

/// 气泡图片按需取数（详情板 data URL；**列表载荷不带图**——PL024 载荷分层定案，
/// 本命令只在 kind = Image 时被前端调用）。文本气泡/不存在严格报错
#[tauri::command]
pub fn bubble_get_image(id: i64, ctx: State<'_, AppContext>) -> Result<String, CommandError> {
    bubble_get_image_core(id, &ctx)
}

/// bubble_get_image 核心实现：取 PNG 字节 → base64 data URL
pub fn bubble_get_image_core(id: i64, ctx: &AppContext) -> Result<String, CommandError> {
    let storage = ctx.lock_storage()?;
    let png = storage.get_bubble_image(id)?.ok_or(CommandError::Bubble(
        BubbleError::UnsupportedFormat.to_string(),
    ))?;
    Ok(format!(
        "data:image/png;base64,{}",
        crate::clipboard_image::base64_encode(&png)
    ))
}

/// 复制回取数路由结果（PL024 按 kind 分派；不跨 IPC 无需 Serialize）
#[derive(Debug)]
pub enum BubbleCopyPayload {
    /// 文本气泡原文
    Text(String),
    /// 图片气泡 PNG 字节（写回三格式由剪贴板 FFI 层裁决）
    Image(Vec<u8>),
}

/// 复制回取数路由核心（直测面；写剪贴板动作留命令薄壳不直测）：
/// 文本回原文、图片回 PNG 字节；图片列缺席（防御面）严格报错
pub fn bubble_copy_payload_core(
    id: i64,
    ctx: &AppContext,
) -> Result<BubbleCopyPayload, CommandError> {
    let storage = ctx.lock_storage()?;
    let item = storage.get_bubble(id)?;
    match item.kind {
        crate::bubble::BubbleKind::Text => Ok(BubbleCopyPayload::Text(item.text)),
        crate::bubble::BubbleKind::Image => match storage.get_bubble_image(id)? {
            Some(png) => Ok(BubbleCopyPayload::Image(png)),
            None => Err(CommandError::Bubble(
                BubbleError::UnsupportedFormat.to_string(),
            )),
        },
    }
}

/// 复制气泡内容回剪贴板（捕获→粘贴走→清理闭环的回程；PL024 按 kind 分叉——
/// 文本走插件写文本，图片走 Win32 三格式写入[PNG 注册格式 + CF_DIB + CF_DIBV5]，
/// 用户主动覆盖剪贴板无恢复逻辑）
#[tauri::command]
pub fn bubble_copy(
    id: i64,
    app: AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    match bubble_copy_payload_core(id, &ctx)? {
        BubbleCopyPayload::Text(text) => app
            .clipboard()
            .write_text(&text)
            .map_err(|err| CommandError::Clipboard(format!("写入剪贴板失败：{err}")))?,
        BubbleCopyPayload::Image(png) => crate::clipboard_image::write_clipboard_image(&png)
            .map_err(|err| CommandError::Clipboard(format!("图片写回剪贴板失败：{err}")))?,
    }
    Ok(())
}

/// 删除单条气泡（不存在严格报错）
#[tauri::command]
pub fn bubble_remove(id: i64, ctx: State<'_, AppContext>) -> Result<(), CommandError> {
    bubble_remove_core(id, &ctx)
}

/// bubble_remove 核心实现：删除条目
pub fn bubble_remove_core(id: i64, ctx: &AppContext) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.remove_bubble(id)?)
}

/// 一键清空气泡，返回清除条数（满 5 提醒后的清理出口）
#[tauri::command]
pub fn bubble_clear(ctx: State<'_, AppContext>) -> Result<usize, CommandError> {
    bubble_clear_core(&ctx)
}

/// bubble_clear 核心实现：清空
pub fn bubble_clear_core(ctx: &AppContext) -> Result<usize, CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.clear_bubbles()?)
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
        // 图片捕获入库 + 占位文案格式（🖼 截图 + 12 位数字）+ 同图去重（PL024.8x）
        let ctx = test_context();
        let png = png_magic_bytes();
        let a = bubble_capture_image_core(&png, &ctx)
            .expect("合法图片必须成功")
            .added_item();
        let dup = bubble_capture_image_core(&png, &ctx).expect("去重判定必须成功");
        assert!(
            matches!(dup, BubbleCaptureOutcome::Duplicate),
            "同图第二次应判重复"
        );
        let digits = a
            .text
            .chars()
            .skip("🖼 截图 ".chars().count())
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
    fn capture_image_rejects_garbage_bytes() {
        // 魔数校验在命令层可见（严格抛错主线；Bubble 变体承载文案）
        let ctx = test_context();
        let err = bubble_capture_image_core(b"garbage", &ctx).expect_err("垃圾字节必须被拒");
        assert!(
            matches!(err, CommandError::Bubble(ref msg) if msg.contains("图片格式暂不支持")),
            "文案必须可见且指向格式"
        );
    }

    #[test]
    fn get_image_core_data_url_and_strict_errors() {
        // 图片气泡 → data URL 前缀；文本气泡/不存在严格报错
        let ctx = test_context();
        let item = bubble_capture_image_core(&png_magic_bytes(), &ctx)
            .expect("合法图片必须成功")
            .added_item();
        let url = bubble_get_image_core(item.id, &ctx).expect("取数必须成功");
        assert!(url.starts_with("data:image/png;base64,"));
        let text_item = bubble_capture_core("文本", &ctx)
            .expect("写入必须成功")
            .added_item();
        let err = bubble_get_image_core(text_item.id, &ctx).expect_err("文本气泡必须报错");
        assert!(matches!(err, CommandError::Bubble(ref msg) if msg.contains("图片格式暂不支持")));
        assert!(bubble_get_image_core(99, &ctx).is_err());
    }

    /// 最小合法 PNG 形态（魔数 + 载荷；storage 测试同款语义）
    fn png_magic_bytes() -> Vec<u8> {
        vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 7, 8, 9]
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
        // PL024.6 迁移为取数路由面：文本 → Text；图片 → Image；不存在 → NotFound
        let ctx = test_context();
        let item = bubble_capture_core("片段", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        assert!(matches!(
            bubble_copy_payload_core(item.id, &ctx).expect("回读必须成功"),
            BubbleCopyPayload::Text(ref t) if t == "片段"
        ));
        let img = bubble_capture_image_core(&png_magic_bytes(), &ctx)
            .expect("图片捕获必须成功")
            .added_item();
        assert!(matches!(
            bubble_copy_payload_core(img.id, &ctx).expect("取图必须成功"),
            BubbleCopyPayload::Image(ref png) if png == &png_magic_bytes()
        ));
        let err = bubble_copy_payload_core(99, &ctx).expect_err("不存在必须报错");
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
        bubble_remove_core(a.id, &ctx).expect("刚添加的条目必须存在");
        assert_eq!(
            bubble_list_core(&ctx).expect("读命令必须成功").items.len(),
            1
        );
        let cleared = bubble_clear_core(&ctx).expect("清空必须成功");
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
