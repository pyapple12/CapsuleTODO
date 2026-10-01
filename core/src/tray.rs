//! 托盘菜单句柄与偏好切换逻辑（PL018）：托盘右键菜单的贴边/置顶勾选项句柄
//! 存储（CheckMenuItem 句柄收进 state，供菜单事件与 prefs-changed 监听翻转）+
//! 菜单点击的落库/生效/同步处理 + hover 预览窗显隐联动。勾选态唯一事实源 =
//! AppContext + config.json；三入口（设置板 / 托盘菜单 / 事件回显）同源同步。

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::menu::CheckMenuItem;
use tauri::{Emitter, Manager};

use crate::commands::AppContext;

/// 预览窗悬停取消标志：true = 鼠标在预览窗内（延迟隐藏线程见此退避）
static PREVIEW_HOVER_CANCELLED: AtomicBool = AtomicBool::new(false);

/// 预览窗宽度（逻辑像素，用户口头调参单一来源：创建与 resize 共用）
pub const PREVIEW_WIDTH: f64 = 200.0;

/// 预览悬停代际（PL018 bug1 修复）：每次 Enter 递增——托盘图标边缘抖动会产生
/// Leave→Enter 快速交替，Leave 排队的延迟隐藏若不识代数会在用户仍悬停时误隐
/// （Enter 不取消排队 hide 的缺陷）；隐藏线程到点时代数不符即退避
static PREVIEW_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 最后一次托盘 Enter 的图标矩形顶缘（物理 y）——预览窗 resize 后底缘锚定
/// 重算的基准（托盘顶 - 窗高 - 间距 = 顶缘，保持贴托盘方向收缩）
static LAST_TRAY_TOP: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// 最后一次托盘 Enter 的图标矩形右缘（物理 x）——resize 后右对齐重算基准
static LAST_TRAY_RIGHT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// 托盘菜单勾选项句柄（setup 时 manage，on_menu_event 与 prefs-changed 监听取用）
pub struct TrayMenuItems {
    /// 贴边吸附勾选项
    pub snap: CheckMenuItem<tauri::Wry>,
    /// 窗口置顶勾选项
    pub top: CheckMenuItem<tauri::Wry>,
}

/// 托盘菜单勾选点击（PL018.2）：读当前值取反 → 复用命令核心落库 → 即时生效
/// （置顶 = set_always_on_top；吸附 = 事件层自动）→ 勾选态从事实源重读翻转 →
/// emit prefs-changed 全量广播（设置板监听回显）。任一步失败落日志，勾选态
/// 由下次事实源读取纠正（不做本地回滚——ctx 未变即事实未变）
pub fn on_pref_menu(app: &tauri::AppHandle, id: &str) {
    let on = match id {
        "tray-snap" => !snap_current(app),
        "tray-top" => !top_current(app),
        _ => return,
    };
    let path = match crate::paths::settings_path() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("托盘开关切换失败（定位设置文件）：{err}");
            return;
        }
    };
    let ctx = app.state::<AppContext>();
    let result = match id {
        "tray-snap" => {
            crate::commands::settings::settings_set_snap_to_edge_core(on, &path, &ctx).map(|_| ())
        }
        "tray-top" => crate::commands::settings::settings_set_always_on_top_core(on, &path, &ctx)
            .map(|_| {
                if let Some(w) = app.get_webview_window("main") {
                    if let Err(err) = w.set_always_on_top(on) {
                        eprintln!("托盘置顶切换生效失败：{err}");
                    }
                }
            }),
        _ => return,
    };
    if let Err(err) = result {
        eprintln!("托盘开关切换失败（落库未变）：{err}");
        return;
    }
    // 勾选态从事实源重读翻转（避免组合判断）+ 全量广播
    let (snap, top) = prefs_current(app);
    let items = app.state::<TrayMenuItems>();
    let _ = items.snap.set_checked(snap);
    let _ = items.top.set_checked(top);
    if let Err(err) = app.emit(
        "prefs-changed",
        serde_json::json!({ "always_on_top": top, "snap_to_edge": snap }),
    ) {
        eprintln!("prefs-changed 广播失败：{err}");
    }
}

/// 贴边吸附当前值（锁失败按默认开——与托盘构建缺省语义一致）
fn snap_current(app: &tauri::AppHandle) -> bool {
    app.state::<AppContext>()
        .lock_settings()
        .map(|s| s.snap_to_edge)
        .unwrap_or(true)
}

/// 窗口置顶当前值
fn top_current(app: &tauri::AppHandle) -> bool {
    app.state::<AppContext>()
        .lock_settings()
        .map(|s| s.always_on_top)
        .unwrap_or(true)
}

/// 全量当前偏好（snap, top）
fn prefs_current(app: &tauri::AppHandle) -> (bool, bool) {
    (snap_current(app), top_current(app))
}

/// 托盘 Enter（PL018.5）：记录图标矩形锚点（resize 后底缘锚定用）→ 按矩形
/// 定位预览窗（右对齐托盘、默认上方，屏幕上缘放不下翻下方）→ 显示。
/// x/y/w/h 为托盘图标矩形物理坐标（spike 实测 50×50 含 padding）
pub fn on_tray_enter(app: &tauri::AppHandle, rx: i32, ry: i32, rw: i32, rh: i32) {
    PREVIEW_HOVER_CANCELLED.store(false, Ordering::Relaxed);
    PREVIEW_GEN.fetch_add(1, Ordering::Relaxed); // 新悬停周期：作废排队中的延迟隐藏
    LAST_TRAY_TOP.store(ry, Ordering::Relaxed);
    LAST_TRAY_RIGHT.store(rx + rw, Ordering::Relaxed);
    let Some(w) = app.get_webview_window("tray-preview") else {
        return;
    };
    const MARGIN: i32 = 8;
    // 定位用实际窗尺寸（高度随内容动态调整，常量会让少条目时窗体悬空）
    let Ok(vsize) = w.outer_size() else {
        return;
    };
    let pw = vsize.width as i32;
    let ph = vsize.height as i32;
    let px = rx + rw - pw; // 右对齐托盘图标
    let mut py = ry - ph - MARGIN; // 默认托盘上方
    if py < 0 {
        py = ry + rh + MARGIN; // 任务栏在顶部 → 翻到下方
    }
    if let Err(err) = w.set_position(tauri::PhysicalPosition::new(px, py)) {
        eprintln!("预览窗定位失败：{err}");
    }
    if let Err(err) = w.show() {
        eprintln!("预览窗显示失败：{err}");
    }
    // 守候循环启动（bug1 修复）：Leave 事件慢速移出时可能丢失——由循环持续
    // 评估"鼠标离开预览窗与托盘图标两区域"替代一次性 Leave 触发隐藏
    let gen = PREVIEW_GEN.load(Ordering::Relaxed);
    spawn_hover_watch(app, gen);
}

/// resize 后底缘锚定托盘重算位置（PL018，用户定案收缩方向）：窗高变化时
/// 下缘保持在"托盘顶 − 间距"，顶缘移动（往下缩/往上长都贴托盘方向）；
/// 托盘锚点未记录（启动后未 hover 过）则跳过（无锚定基准，此时窗也从未显示）
pub fn reanchor_to_tray(w: &tauri::WebviewWindow) {
    let tray_top = LAST_TRAY_TOP.load(Ordering::Relaxed);
    let tray_right = LAST_TRAY_RIGHT.load(Ordering::Relaxed);
    if tray_top == 0 && tray_right == 0 {
        return;
    }
    let Ok(size) = w.outer_size() else {
        return;
    };
    const MARGIN: i32 = 8;
    let px = tray_right - size.width as i32;
    let py = tray_top - size.height as i32 - MARGIN;
    if let Err(err) = w.set_position(tauri::PhysicalPosition::new(px, py)) {
        eprintln!("预览窗锚定重算失败：{err}");
    }
}

/// 托盘 Leave（PL018.5）：慢速移出时该事件可能不触发——隐藏职责已由 Enter 启动
/// 的守候循环（watch_hover_loop）全条件覆盖，此处仅复位取消标志
pub fn on_tray_leave(app: &tauri::AppHandle) {
    PREVIEW_HOVER_CANCELLED.store(false, Ordering::Relaxed);
    let gen = PREVIEW_GEN.load(Ordering::Relaxed);
    spawn_hover_watch(app, gen);
}

/// 预览窗悬停状态上报（PL018.5，命令 tray_preview_hover 调用）：
/// true = mouseenter（鼠标进预览窗，守候循环见此退避，显隐交 mouseleave）；
/// false = mouseleave（重新起守候循环）
pub fn set_preview_hover(hovering: bool, app: &tauri::AppHandle) {
    PREVIEW_HOVER_CANCELLED.store(hovering, Ordering::Relaxed);
    if !hovering {
        let gen = PREVIEW_GEN.load(Ordering::Relaxed);
        spawn_hover_watch(app, gen);
    }
}

/// 守候循环（bug1 修复，替代一次性到点判定）：Enter/离开预览窗时启动，200ms
/// 步进持续评估—— Leave 事件丢失（慢速移出）不再导致永挂；鼠标连续 400ms
/// 不在预览窗与托盘图标两区域内 → 隐藏；期间新 Enter（代数变化）或进预览窗
/// → 线程退避，显隐交新周期
fn spawn_hover_watch(app: &tauri::AppHandle, gen: u64) {
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut miss = 0u32;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(200));
            if gen != PREVIEW_GEN.load(Ordering::Relaxed) {
                return; // 新 Enter 已接管，本线程退避
            }
            if PREVIEW_HOVER_CANCELLED.load(Ordering::Relaxed) {
                return; // 鼠标在预览窗内，显隐交 mouseleave 路径
            }
            let Some(w) = handle.get_webview_window("tray-preview") else {
                return;
            };
            if cursor_in_window(&w) {
                miss = 0;
                continue; // 鼠标在预览窗上（mouseenter 丢失兜底），守候
            }
            let tray_top = LAST_TRAY_TOP.load(Ordering::Relaxed);
            let tray_right = LAST_TRAY_RIGHT.load(Ordering::Relaxed);
            if cursor_in_tray(tray_top, tray_right) {
                miss = 0;
                continue; // 鼠标还在托盘图标上，守候
            }
            miss += 1;
            if miss >= 2 {
                // 连续 400ms 离开两区域：确已离开，隐藏
                if let Err(err) = w.hide() {
                    eprintln!("预览窗隐藏失败：{err}");
                }
                return;
            }
        }
    });
}

/// 光标是否落在托盘图标矩形内（守候循环判定：图标矩形由 Enter 记录）
fn cursor_in_tray(tray_top: i32, tray_right: i32) -> bool {
    #[cfg(target_os = "windows")]
    {
        #[repr(C)]
        struct Point {
            x: i32,
            y: i32,
        }
        #[link(name = "user32")]
        extern "system" {
            fn GetCursorPos(point: *mut Point) -> i32;
        }
        let mut pt = Point { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut pt) } == 0 {
            return false;
        }
        // 图标矩形：右缘/顶缘已知，按 spike 实测 50×60 反推左/下
        pt.x >= tray_right - 50 && pt.x <= tray_right && pt.y >= tray_top && pt.y <= tray_top + 60
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (tray_top, tray_right);
        false
    }
}

/// 光标是否落在预览窗矩形内（bug2 兜底守卫：窗 hide/show 切换期 webview 的
/// mouseenter 可能丢失，物理位置不依赖事件）
fn cursor_in_window(w: &tauri::WebviewWindow) -> bool {
    #[cfg(target_os = "windows")]
    {
        #[repr(C)]
        struct Point {
            x: i32,
            y: i32,
        }
        #[link(name = "user32")]
        extern "system" {
            fn GetCursorPos(point: *mut Point) -> i32;
        }
        let mut pt = Point { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut pt) } == 0 {
            return false;
        }
        let Ok(pos) = w.outer_position() else {
            return false;
        };
        let Ok(size) = w.outer_size() else {
            return false;
        };
        pt.x >= pos.x
            && pt.x < pos.x + size.width as i32
            && pt.y >= pos.y
            && pt.y < pos.y + size.height as i32
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}
