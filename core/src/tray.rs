//! 托盘菜单句柄与偏好切换逻辑（PL018）：托盘右键菜单的贴边/置顶勾选项句柄
//! 存储（CheckMenuItem 句柄收进 state，供菜单事件与 prefs-changed 监听翻转）+
//! 菜单点击的落库/生效/同步处理 + hover 预览窗显隐联动。勾选态唯一事实源 =
//! AppContext + config.json；三入口（设置板 / 托盘菜单 / 事件回显）同源同步。
//! 预览窗显隐 = 单一常驻守候线程 + 状态机（Idle/Armed/Shown/Suppressed）：
//! Enter 武装（图标上持续停留 500ms 才显示，瞬间划过不打扰），离开两区域
//! 400ms 隐藏；右键锁死期探测本进程菜单窗口（#32768）存亡，菜单收起才解除
//! ——期间 Enter 一律忽略（菜单开着时托盘 Enter 仍会触发，实测事实），
//! 解除后需新 Enter 重新武装（收菜单时恰停在图标上不出窗）。显隐只由守候
//! 线程执行（show/hide 唯一调用者），事件处理只写状态——杜绝"线程被换掉
//! 后没人管窗"的孤儿窗竞态（多线程代数作废结构的固有缺陷）。

use std::sync::Mutex;
use tauri::menu::CheckMenuItem;
use tauri::{Emitter, Manager};

use crate::commands::AppContext;

/// 预览窗宽度（逻辑像素，用户口头调参单一来源：创建与 resize 共用）
pub const PREVIEW_WIDTH: f64 = 200.0;

/// 守候相位（预览显隐状态机）：Idle 无事；Armed 已武装（Enter 已触发，图标上
/// 持续停留满 500ms 才显示）；Shown 显示中（离开两区域 400ms 隐藏）；
/// Suppressed 右键锁死（菜单活跃期，探测到菜单收起才解除，解除后需新 Enter
/// 重新武装——收菜单时恰停在图标上不出窗）
/// （Debug 供测试 assert_eq 失败打印）
#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Idle,
    /// since_ms = 武装时刻（Unix 毫秒）
    Armed {
        since_ms: u64,
    },
    /// miss = 连续离开两区域的采样步数（100ms 一步，满 4 步 = 400ms 隐藏）
    Shown {
        miss: u32,
    },
    Suppressed,
}

/// 守候单步的窗口动作（只由守候线程执行——显隐的唯一调用者；Debug 供测试）
#[derive(Debug, Clone, Copy, PartialEq)]
enum WatchAction {
    None,
    Show,
    Hide,
}

/// 守候状态（唯一事实源）：相位 + 托盘图标矩形锚点（定位 / resize 后底缘锚定
/// / 光标判定共用）。事件处理（Enter/右键）与守候线程经 WATCH 互斥锁读写
struct WatchState {
    phase: Phase,
    tray_top: i32,
    tray_right: i32,
    tray_bottom: i32,
}

static WATCH: Mutex<WatchState> = Mutex::new(WatchState {
    phase: Phase::Idle,
    tray_top: 0,
    tray_right: 0,
    tray_bottom: 0,
});

/// WATCH 取锁（锁中毒恢复取值——持锁线程 panic 后内部数据仍完整，取值优于卡死）
fn lock_watch() -> std::sync::MutexGuard<'static, WatchState> {
    WATCH.lock().unwrap_or_else(|e| e.into_inner())
}

/// 当前时刻（Unix 毫秒）——Armed 停留计时的时钟源
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

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

/// 守候状态机单步（纯函数，单测直测）：输入当前相位/时刻/光标位置/菜单存亡/
/// 窗口可见性，输出新相位与窗口动作。守候线程每 100ms 调用一次；显隐动作只
/// 出自这里
fn step_watch(
    phase: Phase,
    now_ms: u64,
    in_tray: bool,
    in_window: bool,
    menu_open: bool,
    window_visible: bool,
) -> (Phase, WatchAction) {
    match phase {
        Phase::Idle => (phase, WatchAction::None),
        Phase::Armed { since_ms } => {
            if menu_open {
                return (Phase::Suppressed, WatchAction::None); // 武装期右键：锁死
            }
            if window_visible {
                // 上一周期残留的可见窗：武装期窗必须隐藏（新周期从隐藏态计时）
                return (Phase::Armed { since_ms }, WatchAction::Hide);
            }
            if !in_tray {
                return (Phase::Idle, WatchAction::None); // 延迟期内离开：取消出现
            }
            if now_ms.saturating_sub(since_ms) >= 500 {
                return (Phase::Shown { miss: 0 }, WatchAction::Show);
            }
            (phase, WatchAction::None) // 未满 0.5s，继续守候
        }
        Phase::Shown { miss } => {
            if menu_open {
                return (Phase::Suppressed, WatchAction::Hide); // 显示中右键：收窗锁死
            }
            if in_window || in_tray {
                return (Phase::Shown { miss: 0 }, WatchAction::None); // 两区域任一在内，守住
            }
            let miss = miss + 1;
            if miss >= 4 {
                (Phase::Idle, WatchAction::Hide) // 连续 400ms 离开两区域：确已离开
            } else {
                (Phase::Shown { miss }, WatchAction::None)
            }
        }
        Phase::Suppressed => {
            if menu_open {
                (phase, WatchAction::None) // 菜单还开着：保持锁死
            } else {
                (Phase::Idle, WatchAction::None) // 菜单已收：解除（需新 Enter 重新武装）
            }
        }
    }
}

/// 托盘 Enter（预览延迟出现）：菜单开着则整个事件忽略（实测：菜单开着时托盘
/// Enter 仍会触发——此时武装 = 菜单旁弹出预览窗，即旧 bug）；否则记录图标
/// 矩形锚点并武装（显示延迟到守候线程"持续停留 500ms"判定）。残留可见窗
/// 不在此处收（主线程不碰预览窗，跨线程窗口操作有死锁面）——守候线程下一拍
/// 依"Armed ∧ 可见 → Hide"自愈。
/// x/y/w/h 为托盘图标矩形物理坐标（spike 实测 50×50 含 padding）
pub fn on_tray_enter(rx: i32, ry: i32, rw: i32, rh: i32) {
    if menu_is_open() {
        return; // 菜单开着：不解锁、不武装（解除只走守候线程的菜单存亡探测）
    }
    let mut st = lock_watch();
    st.tray_top = ry;
    st.tray_right = rx + rw;
    st.tray_bottom = ry + rh;
    st.phase = Phase::Armed { since_ms: now_ms() };
}

/// 托盘右键按下：锁死（Suppressed）+ 立即收窗——菜单活跃期的压制由守候线程
/// 探测菜单窗口存亡接手（右键到菜单弹出有毫秒级窗口期，锁死先落）
pub fn on_tray_right_button(app: &tauri::AppHandle) {
    lock_watch().phase = Phase::Suppressed;
    if let Some(w) = app.get_webview_window("tray-preview") {
        let _ = w.hide();
    }
}

/// 按托盘锚点定位预览窗（显示前调用）：右对齐托盘图标、默认上方，屏幕上缘
/// 放不下翻托盘下方（任务栏在顶部场景）。定位用实际窗尺寸（高度随内容动态
/// 调整，常量会让少条目时窗体悬空）；锚点未记录（全 0）则跳过。
/// 锚点参数化（非直接读 WATCH）——守候线程持锁调用，避免重入死锁
fn anchor_preview(w: &tauri::WebviewWindow, tray_top: i32, tray_right: i32, tray_bottom: i32) {
    if tray_top == 0 && tray_right == 0 {
        return;
    }
    let Ok(size) = w.outer_size() else {
        return;
    };
    const MARGIN: i32 = 8;
    let px = tray_right - size.width as i32; // 右对齐托盘图标
    let mut py = tray_top - size.height as i32 - MARGIN; // 默认托盘上方
    if py < 0 {
        py = tray_bottom + MARGIN; // 翻到下方
    }
    if let Err(err) = w.set_position(tauri::PhysicalPosition::new(px, py)) {
        eprintln!("预览窗定位失败：{err}");
    }
}

/// resize 后底缘锚定托盘重算位置（PL018，用户定案收缩方向）：窗高变化时
/// 下缘保持在"托盘顶 − 间距"，顶缘移动（往下缩/往上长都贴托盘方向）；
/// 托盘锚点未记录（启动后未 hover 过）则跳过（无锚定基准，此时窗也从未显示）
pub fn reanchor_to_tray(w: &tauri::WebviewWindow) {
    let (tray_top, tray_right, tray_bottom) = {
        let st = lock_watch();
        (st.tray_top, st.tray_right, st.tray_bottom)
    };
    anchor_preview(w, tray_top, tray_right, tray_bottom);
}

/// 预览守候线程（单一常驻，setup 启动一次永不退出）：100ms 步进驱动状态机，
/// 是 show/hide 的唯一调用者——事件处理只写状态，杜绝多线程换代的孤儿窗。
/// 窗口查询与动作全在锁外执行（见循环内注释），锁内只做纯 FFI 与状态结算
pub fn spawn_preview_watcher(app: &tauri::AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let Some(w) = handle.get_webview_window("tray-preview") else {
            continue; // 预览窗尚未就绪：守候不退出（单一常驻）
        };
        // 窗口查询与动作全部在锁外：Tauri 2 非主线程的窗口调用（含查询 is_visible/
        // outer_position/outer_size，走事件循环消息）都是同步等待主线程应答——持锁
        // 等待遇主线程等锁 = 互等死锁（实测两轮：set_position/show 持锁挂死一次、
        // is_visible 持锁首触即挂）。锁内只留纯 FFI（GetCursorPos/EnumWindows 不
        // 派发消息）与状态结算
        let visible = w.is_visible().unwrap_or(false);
        let in_window = visible && cursor_in_window(&w);
        let (action, anchors) = {
            let mut st = lock_watch();
            // 菜单探测门控：仅 Armed/Suppressed 相位做（EnumWindows 有成本）
            let menu_open = match st.phase {
                Phase::Armed { .. } | Phase::Suppressed => menu_is_open(),
                _ => false,
            };
            let in_tray = cursor_in_tray(st.tray_top, st.tray_right);
            let (new_phase, action) = step_watch(
                st.phase.clone(),
                now_ms(),
                in_tray,
                in_window,
                menu_open,
                visible,
            );
            st.phase = new_phase;
            (action, (st.tray_top, st.tray_right, st.tray_bottom))
        };
        match action {
            WatchAction::Show => {
                anchor_preview(&w, anchors.0, anchors.1, anchors.2);
                if let Err(err) = w.show() {
                    eprintln!("预览窗显示失败：{err}");
                }
            }
            WatchAction::Hide => {
                if let Err(err) = w.hide() {
                    eprintln!("预览窗隐藏失败：{err}");
                }
            }
            WatchAction::None => {}
        }
    });
}

/// 本进程的弹出菜单（Win32 菜单窗口类 #32768）此刻是否开着——右键菜单没有
/// "关闭"事件，此探测是菜单存亡的唯一硬事实源（替代旧方案"鼠标回托盘 =
/// 菜单已收"的启发式误判：实测鼠标回图标时菜单完全可以还开着）。
/// EnumWindows 全局枚举 + pid 过滤 + 类名比对，判定核心拆纯函数供单测
fn menu_is_open() -> bool {
    #[cfg(target_os = "windows")]
    {
        /// 枚举上下文（lparam 透传给回调）
        struct Probe {
            found: bool,
        }
        #[link(name = "user32")]
        extern "system" {
            fn EnumWindows(
                proc: extern "system" fn(isize, *mut std::ffi::c_void) -> i32,
                lparam: *mut std::ffi::c_void,
            ) -> i32;
            fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
            fn GetClassNameW(hwnd: isize, buf: *mut u16, max_count: i32) -> i32;
        }
        extern "system" fn probe_proc(hwnd: isize, lparam: *mut std::ffi::c_void) -> i32 {
            let ctx = unsafe { &mut *(lparam as *mut Probe) };
            let mut pid = 0u32;
            unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
            let mut buf = [0u16; 16];
            let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
            let class = if n > 0 {
                String::from_utf16_lossy(&buf[..n as usize])
            } else {
                String::new()
            };
            if is_menu_window(pid, &class) {
                ctx.found = true;
            }
            1 // 继续枚举
        }
        let mut ctx = Probe { found: false };
        unsafe { EnumWindows(probe_proc, &mut ctx as *mut Probe as *mut std::ffi::c_void) };
        ctx.found
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// 枚举回调的单窗口判定（纯函数）：本进程 + 窗口类 #32768（Win32 弹出菜单）
fn is_menu_window(pid: u32, class: &str) -> bool {
    pid == std::process::id() && class == "#32768"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 菜单窗口判定_仅本进程且类名匹配() {
        let pid = std::process::id();
        assert!(is_menu_window(pid, "#32768"));
        assert!(!is_menu_window(4242, "#32768")); // 他进程的菜单不算
        assert!(!is_menu_window(pid, "Shell_TrayWnd")); // 本进程非菜单窗口不算
    }

    #[test]
    fn 空闲期_任何输入都不动作() {
        assert_eq!(
            step_watch(Phase::Idle, 1000, true, true, true, true),
            (Phase::Idle, WatchAction::None)
        );
    }

    #[test]
    fn 武装期_未满停留_继续守候() {
        let phase = Phase::Armed { since_ms: 1000 };
        assert_eq!(
            step_watch(phase.clone(), 1300, true, false, false, false),
            (phase, WatchAction::None)
        );
    }

    #[test]
    fn 武装期_残留可见窗_收掉并保持武装() {
        assert_eq!(
            step_watch(
                Phase::Armed { since_ms: 1000 },
                1200,
                true,
                false,
                false,
                true
            ),
            (Phase::Armed { since_ms: 1000 }, WatchAction::Hide)
        );
    }

    #[test]
    fn 武装期_离开图标_取消出现() {
        assert_eq!(
            step_watch(
                Phase::Armed { since_ms: 1000 },
                1200,
                false,
                false,
                false,
                false
            ),
            (Phase::Idle, WatchAction::None)
        );
    }

    #[test]
    fn 武装期_停留满五百毫秒_显示() {
        assert_eq!(
            step_watch(
                Phase::Armed { since_ms: 1000 },
                1500,
                true,
                false,
                false,
                false
            ),
            (Phase::Shown { miss: 0 }, WatchAction::Show)
        );
    }

    #[test]
    fn 武装期_菜单开_转锁死不出窗() {
        assert_eq!(
            step_watch(
                Phase::Armed { since_ms: 1000 },
                1200,
                true,
                false,
                true,
                false
            ),
            (Phase::Suppressed, WatchAction::None)
        );
    }

    #[test]
    fn 维持期_在预览窗或图标上_守住并清零() {
        for (in_tray, in_window) in [(true, false), (false, true)] {
            assert_eq!(
                step_watch(
                    Phase::Shown { miss: 3 },
                    5000,
                    in_tray,
                    in_window,
                    false,
                    true
                ),
                (Phase::Shown { miss: 0 }, WatchAction::None)
            );
        }
    }

    #[test]
    fn 维持期_离开计数累积_满四步隐藏() {
        assert_eq!(
            step_watch(Phase::Shown { miss: 2 }, 5000, false, false, false, true),
            (Phase::Shown { miss: 3 }, WatchAction::None)
        );
        assert_eq!(
            step_watch(Phase::Shown { miss: 3 }, 5000, false, false, false, true),
            (Phase::Idle, WatchAction::Hide)
        );
    }

    #[test]
    fn 维持期_菜单开_收窗转锁死() {
        assert_eq!(
            step_watch(Phase::Shown { miss: 0 }, 5000, true, false, true, true),
            (Phase::Suppressed, WatchAction::Hide)
        );
    }

    #[test]
    fn 锁死期_菜单存亡决定保持或解除() {
        assert_eq!(
            step_watch(Phase::Suppressed, 5000, true, false, true, false),
            (Phase::Suppressed, WatchAction::None)
        );
        // 菜单收起：解除回空闲（不自动武装——需新 Enter 重新移入才恢复出现条件）
        assert_eq!(
            step_watch(Phase::Suppressed, 5000, true, false, false, false),
            (Phase::Idle, WatchAction::None)
        );
    }
}
