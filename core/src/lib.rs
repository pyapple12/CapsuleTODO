//! 应用装配：DWM 焦点联动玻璃材质、桌面固定（置顶 + 全屏让位）、位置记忆与模块注册。
//! 材质定案（沿 CapsulePulse PL010/PL011）：平时纯 alpha 透明常驻（不挂背板），
//! 聚焦瞬间挂 DWM Acrylic 真磨砂（DWMSBT_TRANSIENTWINDOW），失焦即刻撤回（DWMSBT_NONE）。

pub mod bubble;
pub mod commands;
pub mod fullscreen;
pub mod paths;
pub mod settings;
pub mod storage;
pub mod todo;
pub mod whiteboard;

use std::sync::Mutex;

use tauri::{Emitter, Manager, PhysicalPosition, WebviewWindow, WindowEvent};

use commands::AppContext;
use settings::WindowSettings;
use storage::Storage;

/// DWM 窗口属性编号：DWMWA_SYSTEMBACKDROP_TYPE（系统背板材质，Windows 11 22H2+；无官方 Rust 绑定，extern 直连魔数）
#[cfg(target_os = "windows")]
const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;
/// 背板材质：Acrylic 磨砂（DWMSBT_TRANSIENTWINDOW，微软终端同款）
#[cfg(target_os = "windows")]
const DWMSBT_TRANSIENTWINDOW: u32 = 3;
/// 背板材质：无（失焦撤回磨砂；注意非 0——0 是 DWMSBT_AUTO，材质由系统自作主张）
#[cfg(target_os = "windows")]
const DWMSBT_NONE: u32 = 1;

// DwmSetWindowAttribute 直连声明（dwmapi.lib；第四参数 = 值类型字节数，u32 为 4）
#[cfg(target_os = "windows")]
#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(hwnd: isize, attr: u32, value: *const u32, size: u32) -> i32;
}

/// 设置窗口 DWM 背板材质；HRESULT 非零严格报错（是否降级由调用方按容错白名单决定）
#[cfg(target_os = "windows")]
fn set_backdrop(window: &tauri::Window, kind: u32) -> Result<(), Box<dyn std::error::Error>> {
    let hwnd = window.hwnd()?.0 as isize;
    let result = unsafe { DwmSetWindowAttribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, &kind, 4) };
    if result == 0 {
        Ok(())
    } else {
        Err(format!("DwmSetWindowAttribute 失败：HRESULT={result}").into())
    }
}

/// 默认落位：主屏右下距边 40px（窗口物理尺寸按当前缩放比换算）
fn default_position(
    window: &WebviewWindow,
) -> Result<PhysicalPosition<i32>, Box<dyn std::error::Error>> {
    const MARGIN_PX: i32 = 40;
    let monitor = window
        .primary_monitor()?
        .ok_or("无主显示器，无法计算默认落位")?;
    let pos = monitor.position();
    let size = monitor.size();
    let scale = window.scale_factor()?;
    let win_w = (300.0 * scale).round() as i32;
    let win_h = (400.0 * scale).round() as i32;
    Ok(PhysicalPosition::new(
        pos.x + size.width as i32 - win_w - MARGIN_PX,
        pos.y + size.height as i32 - win_h - MARGIN_PX,
    ))
}

/// 保存的位置是否落在任一显示器范围内（越界兜底：显示器拓扑变化后回默认位）
fn position_on_monitor(window: &WebviewWindow, x: i32, y: i32) -> bool {
    window
        .available_monitors()
        .map(|monitors| {
            monitors.iter().any(|m| {
                let pos = m.position();
                let size = m.size();
                x >= pos.x
                    && x < pos.x + size.width as i32
                    && y >= pos.y
                    && y < pos.y + size.height as i32
            })
        })
        .unwrap_or(false)
}

/// 保存窗口位置到 configs/config.json；失败落日志不阻断关闭（容错白名单④，退出意图优先）
fn save_window_position(window: &tauri::Window) {
    let path = match paths::settings_path() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("设置路径解析失败（位置未保存）：{err}");
            return;
        }
    };
    match window.outer_position() {
        Ok(pos) => {
            if let Err(err) = settings::save(&path, &WindowSettings { x: pos.x, y: pos.y }) {
                eprintln!("窗口位置保存失败：{err}");
            }
        }
        Err(err) => eprintln!("窗口位置读取失败（位置未保存）：{err}"),
    }
}

/// 应用装配入口：构建 Tauri builder 并启动主窗口，初始化失败向上传播。
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 运行时数据双落址（PL002）：dev = 仓库根 / release = exe 同级；库打开失败启动即报错
    let db = paths::db_path()?;
    let storage = Storage::open(&db).map_err(|err| format!("清单库打开失败（{db:?}）：{err}"))?;

    tauri::Builder::default()
        // 单实例（PL003）：builder 首位注册；二次启动唤起已运行实例的主窗口
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                if let Err(err) = window.show() {
                    eprintln!("单实例唤起 show 失败：{err}");
                }
                if let Err(err) = window.set_focus() {
                    eprintln!("单实例唤起聚焦失败：{err}");
                }
            }
        }))
        // 剪贴板（PL004）：仅 Rust 侧读写，不经 ACL
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppContext {
            storage: Mutex::new(storage),
        })
        .invoke_handler(tauri::generate_handler![
            commands::todo::todo_add,
            commands::todo::todo_toggle,
            commands::todo::todo_remove,
            commands::todo::todo_list,
            commands::bubble::bubble_capture,
            commands::bubble::bubble_list,
            commands::bubble::bubble_copy,
            commands::bubble::bubble_remove,
            commands::bubble::bubble_clear,
        ])
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .expect("主窗口必须在 tauri.conf.json 中存在");
            // 位置记忆（PL003）：有存档且未越界 → 恢复；否则默认主屏右下距边 40px
            //（文件不存在回默认 = 白名单③；损坏 JSON 严格报错不在此列）
            let saved = settings::load(&paths::settings_path()?)?;
            let position = match saved {
                Some(saved) if position_on_monitor(&window, saved.x, saved.y) => {
                    PhysicalPosition::new(saved.x, saved.y)
                }
                _ => default_position(&window)?,
            };
            window.set_position(position)?;
            // 全屏让位监视（PL003，Windows 实机验证平台）
            #[cfg(target_os = "windows")]
            fullscreen::spawn_fullscreen_watcher(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // 位置记忆：关闭时保存（拖动中不写盘）
            if let WindowEvent::CloseRequested { .. } = event {
                save_window_position(window);
            }
            if let WindowEvent::Focused(focused) = event {
                #[cfg(target_os = "windows")]
                {
                    // 焦点联动材质：聚焦挂 Acrylic 磨砂、失焦撤回透明。
                    // 材质为纯装饰层——切换失败落日志维持前态，不中断主流程
                    //（容错白名单①，下次焦点切换自动重试自愈）
                    let kind = if *focused {
                        DWMSBT_TRANSIENTWINDOW
                    } else {
                        DWMSBT_NONE
                    };
                    if let Err(err) = set_backdrop(window, kind) {
                        eprintln!("焦点联动材质切换失败（维持前态）：{err}");
                    }
                    // 前端分态纱随事件翻转（.focused class）；发送失败由下次焦点事件纠正
                    if let Err(err) = window.emit("window-focus", focused) {
                        eprintln!("window-focus 事件发送失败：{err}");
                    }
                }
                #[cfg(not(target_os = "windows"))]
                let _ = (focused, window);
            }
        })
        .run(tauri::generate_context!())?;
    Ok(())
}
