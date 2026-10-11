//! 玻璃背板（SYSTEMBACKDROP 聚焦联动实验）：DWMWA_SYSTEMBACKDROP_TYPE 属性级
//! 系统背板——聚焦 DWMSBT_TRANSIENTWINDOW（系统亚克力，Win11 22H2+），失焦
//! DWMSBT_NONE（背板移除，窗口回纯 alpha 透明）。
//!
//! 接入面与 AccentPolicy 同级（DWM 窗口属性），不初始化应用侧组合引擎——路线①
//! 实证的组合引擎/WebView2 同线程互斥风险在此不存在（AccentPolicy 同面已实证
//! 渲染正常）。系统背板零参数：观感由系统按主题定，不可调。
//! 切换失败落日志维持前态（容错白名单 PL001.3 同款纪律：材质纯装饰层不阻断主流程）。
//! Windows 专属；DwmSetWindowAttribute 为公开 API（dwmapi.lib 标准导入）。
//!
//! **形态约定（PL026）**：公开壳收 `&WebviewWindow`（模块内提句柄），非 Windows
//! 定义同名空桩——调用点零 `#[cfg]`；私有 `*_raw` 收裸句柄，保留裸句柄入口供
//! 将来非窗口调用者复用。本模块为 FFI 能力模块，豁免「纯逻辑模块禁 import tauri」，
//! 依据见 z.plan.md 附录 PL026。

use tauri::Window;

/// 窗口句柄提取抽象（公开壳统一入参）：`Window` 与 `WebviewWindow` 两种窗口类型
/// 均实现——调用点不必自行提句柄（自行提会因 `hwnd()` 平台限定而被迫写 `#[cfg]`）
pub trait WinHandle {
    /// 取原始窗口句柄（Windows；取不到 → None）
    #[cfg(target_os = "windows")]
    fn win_hwnd(&self) -> Option<isize>;
}

#[cfg(target_os = "windows")]
impl WinHandle for Window {
    fn win_hwnd(&self) -> Option<isize> {
        self.hwnd().ok().map(|h| h.0 as isize)
    }
}

#[cfg(target_os = "windows")]
impl WinHandle for tauri::WebviewWindow {
    fn win_hwnd(&self) -> Option<isize> {
        self.hwnd().ok().map(|h| h.0 as isize)
    }
}

/// 非 Windows：空实现（公开壳为空桩，不需要句柄）
#[cfg(not(target_os = "windows"))]
impl<T> WinHandle for T {}

/// DWMWA_SYSTEMBACKDROP_TYPE：系统背板档位属性（Win11 22H2+）
#[cfg(target_os = "windows")]
const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;

/// DWMSBT_NONE：无背板（窗口纯透明基线）
#[cfg(target_os = "windows")]
const DWMSBT_NONE: u32 = 1;

/// DWMSBT_TRANSIENTWINDOW：系统亚克力档（瞬态窗观感，实时模糊背后内容）
#[cfg(target_os = "windows")]
const DWMSBT_TRANSIENTWINDOW: u32 = 3;

/// DWMWA_USE_IMMERSIVE_DARK_MODE：窗口深色模式声明（背板按深色基调渲染）
#[cfg(target_os = "windows")]
const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;

/// DWMWA_BORDER_COLOR：窗口边框色属性（Win11 激活描边）
#[cfg(target_os = "windows")]
const DWMWA_BORDER_COLOR: u32 = 34;

/// DWMWA_WINDOW_CORNER_PREFERENCE：窗口圆角偏好属性（Win11；系统切角，
/// 托盘预览小窗直角暗角根治——CSS 圆角外的窗口直角区由 DWM 切圆）
#[cfg(target_os = "windows")]
const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;

/// DWMWCP_ROUND：系统标准圆角（半径随系统主题）
#[cfg(target_os = "windows")]
const DWMWCP_ROUND: u32 = 2;

/// DWMWA_COLOR_NONE：隐藏边框（激活态不再画强调色描边）
#[cfg(target_os = "windows")]
const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;

#[cfg(target_os = "windows")]
#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(hwnd: isize, attr: u32, value: *const u32, size: u32) -> i32;
}

// —— 私有纯 FFI 核（收裸句柄；供公开壳复用，亦保留裸句柄入口）——

/// 切换系统背板：聚焦传 true 挂亚克力，失焦传 false 回纯透明。
/// 失败落日志维持前态（下次焦点切换自动重试自愈）
#[cfg(target_os = "windows")]
fn set_focused_backdrop_raw(hwnd: isize, focused: bool) {
    let backdrop = if focused {
        DWMSBT_TRANSIENTWINDOW
    } else {
        DWMSBT_NONE
    };
    let hr = unsafe { DwmSetWindowAttribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, &backdrop, 4) };
    if hr != 0 {
        eprintln!("系统背板切换失败 hr={hr:#x}（维持前态）");
    }
}

/// 窗口框架风格一次性整定（setup 调用）：①按窗口主题声明深色模式——未声明时
/// DWM 背板按浅色基调渲染（深色主题系统上白雾化，2026-09-29 实测"变白"根因）；
/// ②隐藏激活边框——背板挂载后 DWM 按完整窗口框架渲染，Win11 活动描边显形
/// （实测蓝色一圈根因）
#[cfg(target_os = "windows")]
fn apply_frame_style_raw(hwnd: isize, dark: bool) {
    let dark_flag = dark as u32;
    let hr_dark =
        unsafe { DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark_flag, 4) };
    if hr_dark != 0 {
        eprintln!("深色模式声明失败 hr={hr_dark:#x}");
    }
    let border_none = DWMWA_COLOR_NONE;
    let hr_border = unsafe { DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, &border_none, 4) };
    if hr_border != 0 {
        eprintln!("边框隐藏失败 hr={hr_border:#x}");
    }
}

/// 窗口系统圆角（PL018，托盘预览小窗）：DWMWA_WINDOW_CORNER_PREFERENCE = ROUND——
/// DWM 直接把窗口矩形切圆，CSS 圆角外的直角暗角根治（Win11 原生浮窗同款）；
/// Win10 无此属性调用失败无害（忽略返回值）
#[cfg(target_os = "windows")]
fn apply_round_corners_raw(hwnd: isize) {
    let pref = DWMWCP_ROUND;
    let hr = unsafe { DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &pref, 4) };
    if hr != 0 {
        eprintln!("窗口圆角设置失败 hr={hr:#x}（Win10 无此属性，可忽略）");
    }
}

// —— 公开壳（收窗口；Windows 内部提句柄，其余平台空桩 → 调用点零 cfg）——

/// 切换系统背板（聚焦联动）：聚焦挂亚克力、失焦回纯透明；失败落日志维持前态
#[cfg(target_os = "windows")]
pub fn set_focused_backdrop<W: WinHandle>(window: &W, focused: bool) {
    if let Some(hwnd) = window.win_hwnd() {
        set_focused_backdrop_raw(hwnd, focused);
    }
}

/// 切换系统背板（非 Windows 空桩：无系统背板，静默 no-op）
#[cfg(not(target_os = "windows"))]
pub fn set_focused_backdrop<W: WinHandle>(_window: &W, _focused: bool) {}

/// 窗口框架风格整定（深色模式声明 + 边框隐藏）
#[cfg(target_os = "windows")]
pub fn apply_frame_style<W: WinHandle>(window: &W, dark: bool) {
    if let Some(hwnd) = window.win_hwnd() {
        apply_frame_style_raw(hwnd, dark);
    }
}

/// 窗口框架风格整定（非 Windows 空桩）
#[cfg(not(target_os = "windows"))]
pub fn apply_frame_style<W: WinHandle>(_window: &W, _dark: bool) {}

/// 窗口系统圆角（与两个托盘窗同款调用）
#[cfg(target_os = "windows")]
pub fn apply_round_corners<W: WinHandle>(window: &W) {
    if let Some(hwnd) = window.win_hwnd() {
        apply_round_corners_raw(hwnd);
    }
}

/// 窗口系统圆角（非 Windows 空桩）
#[cfg(not(target_os = "windows"))]
pub fn apply_round_corners<W: WinHandle>(_window: &W) {}
