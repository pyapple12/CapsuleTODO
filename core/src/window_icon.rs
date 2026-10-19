//! 窗口图标整定（Windows 专属，`lib.rs` setup 一次性整定）：把 exe 内嵌的**多帧**
//! 应用图标按系统图标尺寸取帧，装进窗口的两个图标槽，让任务栏 / Alt+Tab / 标题栏
//! 都拿到原生帧（PATCH002.59，来源 = 系列项目 CapsulePulse PATCH013）。
//!
//! **为什么必须自己设**：① Tauri 编译期只把 `icon.ico` 的**目录首帧**解码成**单张**
//! RGBA 当默认窗口图标（`tauri-codegen` 取 `icon_dir.entries()[0]`，**不是最大帧**）；
//! ② tao 把这张单帧**只塞进 `ICON_SMALL`**、并把 `ICON_BIG` **显式清空**
//! （`set_taskbar_icon(None)`）；③ ⇒ 窗口手上唯一那张位图 = 首帧尺寸，任务栏 /
//! Alt+Tab 需要更大尺寸时**只能放大它** ⇒ 糊（本仓 ico 首帧 32 ⇒ 100% DPI 只缩不放
//! 尚可，125%/150%/200% 缩放下任务栏需 30/36/48px 只能放大 ✗）；④ 同应用的控制台窗口
//! **没有** `WM_SETICON`，Windows 只能去 exe 的 `RT_GROUP_ICON` 里按尺寸挑帧 ⇒ 清晰
//! ——"同图标、两窗清晰度不同"即由此而来。
//!
//! **修法**（零新依赖：直连 user32/kernel32，同 `glass_backdrop.rs` 手法）：
//! `LoadImageW(本模块, IDI_APP_ICON, …, LR_DEFAULTSIZE)` 让 Windows 自己按系统图标
//! 尺寸（`SM_CXICON`，随 DPI = 32/40/48/64）从 `RT_GROUP_ICON` 挑帧，再把拿到的 HICON
//! **同时**设进 `ICON_BIG` 与 `ICON_SMALL`。**两槽刻意给同一张**：窗口槽位是单张位图，
//! 给小的那张必然有界面要放大（正是本 bug 的成因），一律用 ≥ 目标尺寸的帧则最差只是缩小。
//!
//! 形态（PL026）：公开壳收**泛型 `&impl WinHandle`**（与 `glass_backdrop` 同一句柄抽象），
//! 非 Windows 为**同名空桩** ⇒ 调用点零 `#[cfg]`。失败落日志维持前态——纯装饰层不阻断
//! 主流程（与 `glass_backdrop` 同一条容错纪律）。

use crate::glass_backdrop::WinHandle;

/// `IDI_APP_ICON`：应用图标资源**组号**（`IDI_APPLICATION` = 32512）。
/// `tauri-build` 写 `RT_GROUP_ICON` 时固定用该号（`tauri-winres` 沿用 `IDI_APPLICATION`）。
#[cfg(target_os = "windows")]
const IDI_APP_ICON: u16 = 32512;

/// `IMAGE_ICON`：`LoadImageW` 取的是图标（非光标 / 位图）。
#[cfg(target_os = "windows")]
const IMAGE_ICON: u32 = 1;

/// `LR_DEFAULTSIZE`：用系统图标尺寸（`SM_CXICON`，随 DPI 缩放）作为期望尺寸。
#[cfg(target_os = "windows")]
const LR_DEFAULTSIZE: u32 = 0x0040;

/// `LR_SHARED`：共享资源标志（**不可 `DestroyIcon`**，进程退出由系统回收）。
#[cfg(target_os = "windows")]
const LR_SHARED: u32 = 0x8000;

/// `WM_SETICON`：给窗口设大 / 小图标。
#[cfg(target_os = "windows")]
const WM_SETICON: u32 = 0x0080;

/// `ICON_BIG`：大图标槽——任务栏按钮与 Alt+Tab 用。
#[cfg(target_os = "windows")]
const ICON_BIG: usize = 1;

/// `ICON_SMALL`：小图标槽——标题栏与任务管理器用（Tauri 默认只塞这一槽）。
#[cfg(target_os = "windows")]
const ICON_SMALL: usize = 0;

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    /// 取模块句柄（传空指针 = 本进程 exe）。
    fn GetModuleHandleW(name: *const u16) -> isize;
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    /// 从模块资源里按期望尺寸取图标（返回 HICON；0 = 失败）。
    fn LoadImageW(
        hinst: isize,
        name: *const u16,
        type_: u32,
        cx: i32,
        cy: i32,
        fuload: u32,
    ) -> isize;
    /// 给窗口发消息（此处只用于 `WM_SETICON`）。
    fn SendMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
}

/// 给窗口装多帧应用图标（`ICON_BIG` + `ICON_SMALL` 同设同一张）。
/// 非 Windows 为空实现（同一签名，调用点零 `#[cfg]`）。失败落日志维持前态。
pub fn apply_app_icon<W: WinHandle>(window: &W) {
    #[cfg(target_os = "windows")]
    {
        let Some(hwnd) = window.win_hwnd() else {
            crate::diag::warn_diag("窗口句柄获取失败（应用图标未整定）");
            return;
        };
        let name = IDI_APP_ICON as usize as *const u16;
        let hicon = unsafe {
            LoadImageW(
                GetModuleHandleW(std::ptr::null()),
                name,
                IMAGE_ICON,
                0,
                0,
                LR_DEFAULTSIZE | LR_SHARED,
            )
        };
        if hicon == 0 {
            crate::diag::warn_diag(
                "应用图标资源加载失败（窗口图标维持 Tauri 单帧，任务栏可能偏糊）",
            );
            return;
        }
        unsafe {
            SendMessageW(hwnd, WM_SETICON, ICON_BIG, hicon);
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL, hicon);
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = window;
}
