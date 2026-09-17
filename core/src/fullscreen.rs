//! 全屏让位（PL003）：前台窗口全屏覆盖板子所在显示器时摘除置顶（被盖住），退出恢复。
//! 判定为纯函数（注入矩形可单测）；Win32 经 extern 直连（零新依赖，沿 dwmapi 模式）。
//! 判法定案：逐边包含法（Chromium 同款）而非面积占比——最大化窗口只到工作区、必留
//! 任务栏条，包含法不会把它误判为全屏；面积法在小任务栏显示器上会误判（占比可 >95%）。

/// 屏幕坐标矩形（像素）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i64,
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
}

/// 逐边比对容差（像素）：吸收 DPI 缩放取整与全屏窗 1px 级偏移
pub const EDGE_TOLERANCE_PX: i64 = 8;

/// 判定 win 是否全屏覆盖 monitor：四边均贴到显示器边缘（容差内，覆盖含任务栏区）。
/// 最大化窗口缺任务栏条（典型 40px > 容差），不会被误判
pub fn is_covering(win: Rect, monitor: Rect) -> bool {
    let monitor_area = (monitor.right - monitor.left) * (monitor.bottom - monitor.top);
    if monitor_area <= 0 {
        return false;
    }
    win.left <= monitor.left + EDGE_TOLERANCE_PX
        && win.top <= monitor.top + EDGE_TOLERANCE_PX
        && win.right >= monitor.right - EDGE_TOLERANCE_PX
        && win.bottom >= monitor.bottom - EDGE_TOLERANCE_PX
}

#[cfg(target_os = "windows")]
mod win32 {
    /// Win32 RECT（物理像素）
    #[repr(C)]
    #[derive(Default)]
    pub struct RectPx {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }

    /// MONITORINFO（GetMonitorInfoW 入参，cbSize 必须先填）
    #[repr(C)]
    pub struct MonitorInfo {
        pub cb_size: u32,
        pub rc_monitor: RectPx,
        pub rc_work: RectPx,
        pub dw_flags: u32,
    }

    // user32 直连声明（全屏让位监视所需最小面）
    #[link(name = "user32")]
    extern "system" {
        pub fn GetForegroundWindow() -> isize;
        pub fn GetWindowRect(hwnd: isize, rect: *mut RectPx) -> i32;
        pub fn MonitorFromWindow(hwnd: isize, flags: u32) -> isize;
        pub fn GetMonitorInfoW(monitor: isize, info: *mut MonitorInfo) -> i32;
    }

    /// 取 hwnd 最近的显示器
    pub const MONITOR_DEFAULTTONEAREST: u32 = 2;
}

/// 全屏监视轮询间隔（毫秒）：1s 粒度足够（全屏进出为秒级场景），成本可忽略
#[cfg(target_os = "windows")]
const WATCH_INTERVAL_MS: u64 = 1000;

/// 启动全屏让位后台线程（Windows）：前台窗全屏覆盖板子所在显示器 → 摘 topmost（被盖住），
/// 退出全屏 → 挂回置顶；状态变化才调用，避免每秒重设。
/// 轮询失败维持当前置顶态并只在失败态翻转时落日志（容错白名单②）
#[cfg(target_os = "windows")]
pub fn spawn_fullscreen_watcher(app: tauri::AppHandle) {
    use tauri::Manager;

    let Some(window) = app.get_webview_window("main") else {
        eprintln!("全屏让位监视未启动：主窗口不存在");
        return;
    };
    let hwnd = match window.hwnd() {
        Ok(h) => h.0 as isize,
        Err(err) => {
            eprintln!("全屏让位监视未启动：获取主窗句柄失败：{err}");
            return;
        }
    };
    std::thread::spawn(move || {
        // 当前是否已摘除置顶（状态变化才调用 set_always_on_top）
        let mut yielded = false;
        // 上轮轮询是否成功（失败只报一次，防日志刷屏）
        let mut polling = true;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(WATCH_INTERVAL_MS));
            let win_rect = unsafe {
                let foreground = win32::GetForegroundWindow();
                if foreground == 0 {
                    continue;
                }
                let mut rect = win32::RectPx::default();
                if win32::GetWindowRect(foreground, &mut rect) == 0 {
                    if polling {
                        eprintln!("全屏监视 GetWindowRect 失败（维持当前置顶态）");
                        polling = false;
                    }
                    continue;
                }
                rect
            };
            let monitor_rect = unsafe {
                let monitor = win32::MonitorFromWindow(hwnd, win32::MONITOR_DEFAULTTONEAREST);
                let mut info = win32::MonitorInfo {
                    cb_size: std::mem::size_of::<win32::MonitorInfo>() as u32,
                    rc_monitor: win32::RectPx::default(),
                    rc_work: win32::RectPx::default(),
                    dw_flags: 0,
                };
                if win32::GetMonitorInfoW(monitor, &mut info) == 0 {
                    if polling {
                        eprintln!("全屏监视 GetMonitorInfoW 失败（维持当前置顶态）");
                        polling = false;
                    }
                    continue;
                }
                info.rc_monitor
            };
            polling = true;
            let covering = is_covering(
                Rect {
                    left: win_rect.left as i64,
                    top: win_rect.top as i64,
                    right: win_rect.right as i64,
                    bottom: win_rect.bottom as i64,
                },
                Rect {
                    left: monitor_rect.left as i64,
                    top: monitor_rect.top as i64,
                    right: monitor_rect.right as i64,
                    bottom: monitor_rect.bottom as i64,
                },
            );
            if covering != yielded {
                // set_always_on_top 内部派发主线程，工作线程可直调；失败维持前态下轮重试
                if let Err(err) = window.set_always_on_top(!covering) {
                    eprintln!("全屏让位置顶切换失败（维持前态）：{err}");
                    continue;
                }
                yielded = covering;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1000×700 测试显示器
    const MON: Rect = Rect {
        left: 0,
        top: 0,
        right: 1000,
        bottom: 700,
    };

    #[test]
    fn exact_fullscreen_is_covering() {
        assert!(is_covering(MON, MON));
    }

    #[test]
    fn within_tolerance_is_covering() {
        // 四边各内缩 5px（≤ 容差 8）仍判全屏（DPI 取整冗余）
        let win = Rect {
            left: 5,
            top: 5,
            right: 995,
            bottom: 695,
        };
        assert!(is_covering(win, MON));
    }

    #[test]
    fn maximized_window_not_covering() {
        // 最大化窗口 = 工作区（底部留 40px 任务栏条，> 容差 8）→ 不判全屏（判法定案关键用例）
        let win = Rect {
            left: 0,
            top: 0,
            right: 1000,
            bottom: 660,
        };
        assert!(!is_covering(win, MON));
    }

    #[test]
    fn small_window_not_covering() {
        let win = Rect {
            left: 100,
            top: 100,
            right: 400,
            bottom: 400,
        };
        assert!(!is_covering(win, MON));
    }

    #[test]
    fn no_overlap_not_covering() {
        let win = Rect {
            left: 2000,
            top: 2000,
            right: 3000,
            bottom: 3000,
        };
        assert!(!is_covering(win, MON));
    }

    #[test]
    fn degenerate_monitor_not_covering() {
        let mon = Rect {
            left: 0,
            top: 0,
            right: 1000,
            bottom: 0,
        };
        assert!(!is_covering(MON, mon));
    }
}
