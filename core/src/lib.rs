//! 应用装配：桌面固定（置顶 + 全屏让位）、位置记忆与模块注册。
//! 玻璃材质定案（2026-09-29 用户定案更新）：聚焦联动系统背板——聚焦挂 DWM
//! SYSTEMBACKDROP 亚克力、失焦切 NONE 回纯透明（glass_backdrop.rs），前端 30%
//! 分态纱保留（聚焦纱退 0%）；聚焦/失焦均发 window-focus 事件驱动前端纱态与交互。

pub mod bubble;
pub mod commands;
pub mod fullscreen;
#[cfg(target_os = "windows")]
pub mod glass_backdrop;
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
    let win_h = (400.0 * scale).round() as i32; // 与实验场卡片 300×400 一致（用户定案复刻）
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

/// 保存窗口位置到 configs/config.json；失败落日志不阻断关闭（容错白名单④，退出意图优先）。
/// max_bubbles 从运行时设置透传保留（设置与位置共存一份 config.json）
fn save_window_position(window: &tauri::Window, max_bubbles: u32) {
    let path = match paths::settings_path() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("设置路径解析失败（位置未保存）：{err}");
            return;
        }
    };
    match window.outer_position() {
        Ok(pos) => {
            if let Err(err) = settings::save(
                &path,
                &WindowSettings {
                    x: pos.x,
                    y: pos.y,
                    max_bubbles,
                },
            ) {
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
    // 运行时设置（PL014.2）：config.json 缺失回默认（白名单③）；损坏 JSON 严格报错；
    // max_bubbles 读路径钳制闭环（白名单⑤——手改文件越界静默收敛边界，与写路径同规）
    let mut app_settings = settings::load(&paths::settings_path()?)?.unwrap_or_default();
    app_settings.max_bubbles = settings::clamp_max_bubbles(app_settings.max_bubbles);

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
            settings: Mutex::new(app_settings),
        })
        .invoke_handler(tauri::generate_handler![
            commands::todo::todo_add,
            commands::todo::todo_toggle,
            commands::todo::todo_rename,
            commands::todo::todo_set_note,
            commands::todo::todo_remove,
            commands::todo::todo_list,
            commands::todo::todo_archive_list,
            commands::todo::todo_reorder,
            commands::bubble::bubble_capture,
            commands::bubble::bubble_list,
            commands::bubble::bubble_copy,
            commands::bubble::bubble_remove,
            commands::bubble::bubble_clear,
            commands::bubble::bubble_reorder,
            commands::whiteboard::whiteboard_load,
            commands::whiteboard::whiteboard_save,
            commands::settings::settings_get_max_bubbles,
            commands::settings::settings_set_max_bubbles,
        ])
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .expect("主窗口必须在 tauri.conf.json 中存在");
            // 位置记忆（PL003）：有存档且未越界 → 恢复；否则默认主屏右下距边 40px
            //（文件不存在回默认 = 白名单③；损坏 JSON 严格报错不在此列）。
            // 设置自 builder 前加载进 AppContext（PL014.2），此处直接读运行时副本
            let saved: settings::WindowSettings = match app.state::<AppContext>().lock_settings() {
                Ok(guard) => *guard, // WindowSettings: Copy
                Err(_) => return Err("运行时设置锁中毒".into()),
            };
            let position = match position_on_monitor(&window, saved.x, saved.y) {
                true => PhysicalPosition::new(saved.x, saved.y),
                false => default_position(&window)?,
            };
            window.set_position(position)?;
            // 窗口框架整定（SYSTEMBACKDROP 实验）：深色模式声明（背板基调对齐主题）
            // + 激活边框隐藏（Win11 活动描边显形修复）
            #[cfg(target_os = "windows")]
            {
                let dark = window
                    .theme()
                    .map(|t| t == tauri::Theme::Dark)
                    .unwrap_or(true);
                if let Ok(hwnd) = window.hwnd() {
                    glass_backdrop::apply_frame_style(hwnd.0 as isize, dark);
                }
            }
            // 全屏让位监视（PL003，Windows 实机验证平台）
            #[cfg(target_os = "windows")]
            fullscreen::spawn_fullscreen_watcher(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // 位置记忆：关闭时保存（拖动中不写盘）。max_bubbles 从运行时设置透传，
            // 与位置共存一份 config.json（PL014.2）
            if let WindowEvent::CloseRequested { .. } = event {
                // 锁中毒跳过保存落日志（AGENTS 容错白名单登记项）：默认值写盘会静默
                // 覆盖用户已存设置——位置同弃（下次启动回默认位），磁盘现值不动
                match window.app_handle().state::<AppContext>().lock_settings() {
                    Ok(guard) => save_window_position(window, guard.max_bubbles),
                    Err(err) => eprintln!("设置锁中毒，窗口位置与气泡上限未保存：{err:?}"),
                }
            }
            if let WindowEvent::Focused(focused) = event {
                #[cfg(target_os = "windows")]
                {
                    // 系统背板聚焦联动（SYSTEMBACKDROP 实验）：聚焦挂系统亚克力，
                    // 失焦回纯透明；失败落日志维持前态（下次焦点事件自愈）
                    if let Ok(hwnd) = window.hwnd() {
                        glass_backdrop::set_focused_backdrop(hwnd.0 as isize, *focused);
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
