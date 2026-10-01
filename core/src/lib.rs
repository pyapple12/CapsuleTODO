//! 应用装配：桌面固定（置顶 + 全屏让位）、位置记忆与模块注册。
//! 玻璃材质定案（2026-09-29 用户定案更新）：聚焦联动系统背板——聚焦挂 DWM
//! SYSTEMBACKDROP 亚克力、失焦切 NONE 回纯透明（glass_backdrop.rs），前端 30%
//! 分态纱保留（聚焦纱退 0%）；聚焦/失焦均发 window-focus 事件驱动前端纱态与交互。

pub mod bubble;
pub mod capture;
pub mod commands;
pub mod fullscreen;
#[cfg(target_os = "windows")]
pub mod glass_backdrop;
pub mod hotkey;
pub mod paths;
pub mod settings;
pub mod snap;
pub mod storage;
pub mod todo;
pub mod tray;
pub mod whiteboard;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::{Emitter, Manager, PhysicalPosition, WebviewWindow, WindowEvent};

use commands::AppContext;
use settings::WindowSettings;
use storage::Storage;

// —— 贴边吸附状态机（PL017.4，A 案松手吸附）——
// 松手判定（用户实测三轮修正）：**左键状态为准，Moved 静默为辅**——鼠标悬停/
// 慢速拖动都会造成 Moved 静默 >150ms，仅凭静默会在拖动中误吸（"还没松手就吸
// 走又回到手里"）；左键按住 = 必在拖动，绝不吸附。启动恢复的单次程序性 Moved
// 左键未按 → 不置位 → 默认落位 40px 不被吸
static LAST_MOVED_MS: AtomicU64 = AtomicU64::new(0);
static DRAG_ACTIVE: AtomicBool = AtomicBool::new(false);
static DRAG_SNAP_PENDING: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
mod win_input {
    /// 鼠标左键是否按下（GetAsyncKeyState 高位 = 物理/合成按下中）
    pub fn left_down() -> bool {
        const VK_LBUTTON: i32 = 0x01;
        const PRESSED: u16 = 0x8000;
        unsafe { (GetAsyncKeyState(VK_LBUTTON) as u16) & PRESSED != 0 }
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetAsyncKeyState(v_key: i32) -> i16;
    }
}

/// 当前系统毫秒时刻（吸附防抖时间源；回拨由 saturating_sub 兜底）
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 贴边吸附判定与落位（PL017.4）：读开关（锁失败跳过——吸附是体验增强层，
/// 容错白名单候选）→ 取所在显示器 → snap_position 算修正位 → 与当前位不同
/// 才 set_position（幂等防环：吸附位仍在阈值内，不判等会 Moved→snap 死循环）。
/// 坐标基准 = **视觉（client）矩形**：Windows 无边框窗口带不可见 resize 边
/// （左右 ~8px、顶 0——用户截图像素实测右吸附落位 13px = 5+8），outer 矩形
/// 直接判定会视觉偏移；用 inner_position/inner_size 取视觉矩形参与判定，
/// 落位换算回 outer 坐标（偏移 = inner − outer 自动吸收缩放与隐形边）
fn snap_if_needed(window: &tauri::Window) {
    let snap_on = window
        .app_handle()
        .state::<AppContext>()
        .lock_settings()
        .map(|s| s.snap_to_edge)
        .unwrap_or_else(|err| {
            eprintln!("贴边吸附：设置锁读取失败，跳过吸附：{err:?}");
            false
        });
    if !snap_on {
        return;
    }
    let Ok(monitor) = window.current_monitor() else {
        return;
    };
    let Some(monitor) = monitor else {
        return;
    };
    let Ok(outer) = window.outer_position() else {
        return;
    };
    let Ok(inner) = window.inner_position() else {
        return;
    };
    let Ok(vsize) = window.inner_size() else {
        return;
    };
    let mp = monitor.position();
    let ms = monitor.size();
    let (sx, sy) = snap::snap_position(
        snap::RectI32 {
            x: inner.x,
            y: inner.y,
            w: vsize.width as i32,
            h: vsize.height as i32,
        },
        snap::RectI32 {
            x: mp.x,
            y: mp.y,
            w: ms.width as i32,
            h: ms.height as i32,
        },
    );
    // 视觉目标 → outer 目标（隐形边偏移 = inner − outer）
    let (tx, ty) = (sx - (inner.x - outer.x), sy - (inner.y - outer.y));
    if (tx, ty) != (outer.x, outer.y) {
        if let Err(err) = window.set_position(PhysicalPosition::new(tx, ty)) {
            eprintln!("贴边吸附落位失败：{err}");
        }
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
/// 运行时设置整体透传保留（位置与 max_bubbles/热键共存一份 config.json）
fn save_window_position(window: &tauri::Window, runtime: &settings::WindowSettings) {
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
                    max_bubbles: runtime.max_bubbles,
                    bubble_hotkey: runtime.bubble_hotkey.clone(),
                    always_on_top: runtime.always_on_top,
                    snap_to_edge: runtime.snap_to_edge,
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
    // 载入规范化（上限钳制 + 热键回默认）收敛在 settings::load 单点（FIX004.17）
    let app_settings = settings::load(&paths::settings_path()?)?.unwrap_or_default();

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
            commands::settings::settings_get_bubble_hotkey,
            commands::settings::settings_set_bubble_hotkey,
            commands::settings::settings_get_always_on_top,
            commands::settings::settings_set_always_on_top,
            commands::settings::settings_get_snap_to_edge,
            commands::settings::settings_set_snap_to_edge,
            commands::tray_preview::tray_preview_resize,
        ])
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .expect("主窗口必须在 tauri.conf.json 中存在");
            // 位置记忆（PL003）：有存档且未越界 → 恢复；否则默认主屏右下距边 40px
            //（文件不存在回默认 = 白名单③；损坏 JSON 严格报错不在此列）。
            // 设置自 builder 前加载进 AppContext（PL014.2），此处直接读运行时副本
            let saved: settings::WindowSettings = match app.state::<AppContext>().lock_settings() {
                Ok(guard) => guard.clone(), // WindowSettings 含 String 字段（热键），clone 取副本
                Err(_) => return Err("运行时设置锁中毒".into()),
            };
            let position = match position_on_monitor(&window, saved.x, saved.y) {
                true => PhysicalPosition::new(saved.x, saved.y),
                false => default_position(&window)?,
            };
            window.set_position(position)?;
            // 置顶开关（PL017.1）：conf 静态 true 仅初值，按 config.json 校正生效
            window.set_always_on_top(saved.always_on_top)?;
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
            // 气泡热键（PL015）：启动注册全局热键，触发即捕获剪贴板入气泡；
            // 注册失败落日志不阻断启动（白名单：热键缺失不阻断主流程）
            #[cfg(target_os = "windows")]
            {
                let hotkey_text = app
                    .state::<AppContext>()
                    .lock_settings()
                    .map(|s| s.bubble_hotkey.clone())
                    .unwrap_or_else(|_| settings::DEFAULT_BUBBLE_HOTKEY.to_string());
                let combo = hotkey::parse(&hotkey_text).unwrap_or_else(|err| {
                    eprintln!(
                        "气泡热键解析失败（{err}），回默认 {}",
                        settings::DEFAULT_BUBBLE_HOTKEY
                    );
                    hotkey::parse(settings::DEFAULT_BUBBLE_HOTKEY).expect("默认热键必合法")
                });
                let handle = app.handle().clone();
                hotkey::set_on_hotkey(Box::new(move || {
                    commands::bubble::bubble_capture_from_clipboard_quiet(&handle);
                }));
                if let Err(err) = hotkey::reregister(combo) {
                    eprintln!("气泡热键注册失败（{err}）——快捷键不可用，其余功能不受影响");
                }
            }
            // 托盘（PL018.2）：主窗 skipTaskbar 后的常驻入口——左键显隐主窗，
            // 右键菜单（显示主窗 + 贴边/置顶勾选项；无退出项，用户定案）。
            // 勾选态唯一事实源 = AppContext；句柄存 TrayMenuItems state 供事件翻转
            #[cfg(target_os = "windows")]
            {
                use tauri::{
                    menu::{CheckMenuItem, Menu, MenuItem},
                    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
                    Listener,
                };
                let icon = app
                    .default_window_icon()
                    .cloned()
                    .expect("默认图标必须存在（打包期替换正式图标）");
                let (snap_on, top_on) = app
                    .state::<AppContext>()
                    .lock_settings()
                    .map(|s| (s.snap_to_edge, s.always_on_top))
                    .unwrap_or_else(|err| {
                        eprintln!("托盘勾选态读取失败（按默认开）：{err:?}");
                        (true, true)
                    });
                let show_item =
                    MenuItem::with_id(app, "tray-show", "显示主窗", true, None::<&str>)?;
                let snap_item = CheckMenuItem::with_id(
                    app,
                    "tray-snap",
                    "贴边吸附",
                    true,
                    snap_on,
                    None::<&str>,
                )?;
                let top_item = CheckMenuItem::with_id(
                    app,
                    "tray-top",
                    "窗口置顶",
                    true,
                    top_on,
                    None::<&str>,
                )?;
                // 退出项（PL018 用户定案变更：托盘模式需要退出入口）——走主窗
                // close() 复用完整关窗保存链（位置/设置落库 + T+R3 双腿），禁 app.exit
                let exit_item = MenuItem::with_id(app, "tray-exit", "退出", true, None::<&str>)?;
                app.manage(crate::tray::TrayMenuItems {
                    snap: snap_item.clone(),
                    top: top_item.clone(),
                });
                let menu = Menu::with_items(app, &[&show_item, &snap_item, &top_item, &exit_item])?;
                TrayIconBuilder::with_id("main-tray")
                    .icon(icon)
                    .menu(&menu)
                    .show_menu_on_left_click(false) // 左键给显隐，右键才出菜单
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "tray-show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        // 退出（用户定案变更：托盘模式需要退出入口）——走主窗 close()
                        // 复用完整关窗保存链（位置/设置落库 + T+R3 双腿），禁 app.exit
                        "tray-exit" => {
                            if let Some(w) = app.get_webview_window("main") {
                                if let Err(err) = w.close() {
                                    eprintln!("托盘退出失败：{err}");
                                }
                            }
                        }
                        "tray-snap" | "tray-top" => {
                            crate::tray::on_pref_menu(app, event.id.as_ref());
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| match event {
                        // 左键单击完成（Up 态）= 显隐切换；Down 忽略防按住中间态误触
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } => {
                            let app = tray.app_handle();
                            if let Some(w) = app.get_webview_window("main") {
                                match w.is_visible() {
                                    Ok(true) => {
                                        let _ = w.hide();
                                    }
                                    Ok(false) => {
                                        let _ = w.show();
                                        let _ = w.set_focus();
                                    }
                                    Err(err) => eprintln!("托盘显隐切换失败：{err}"),
                                }
                            }
                        }
                        // hover 预览：Enter 武装（延迟 0.5s 出现），显隐全在守候线程
                        //（Leave 不作显隐依据——慢速移出可能丢失，守候光标轮询全覆盖）
                        // 右键按下：锁死预览窗（菜单活跃期压制，收编进 tray 模块）
                        TrayIconEvent::Click {
                            button: MouseButton::Right,
                            button_state: MouseButtonState::Down,
                            ..
                        } => {
                            crate::tray::on_tray_right_button(tray.app_handle());
                        }
                        TrayIconEvent::Enter { rect, .. } => {
                            if let (tauri::Position::Physical(p), tauri::Size::Physical(s)) =
                                (rect.position, rect.size)
                            {
                                crate::tray::on_tray_enter(
                                    p.x,
                                    p.y,
                                    s.width as i32,
                                    s.height as i32,
                                );
                            }
                        }
                        _ => {}
                    })
                    .build(app)?;
                // 预览守候线程（单一常驻）：状态机驱动显隐，show/hide 唯一调用者
                crate::tray::spawn_preview_watcher(app.handle());
                // prefs-changed 监听（PL018.3）：设置板改开关 → 托盘勾选态同步
                let handle = app.handle().clone();
                app.listen("prefs-changed", move |event| {
                    let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload())
                    else {
                        return;
                    };
                    let items = handle.state::<crate::tray::TrayMenuItems>();
                    if let Some(on) = payload.get("always_on_top").and_then(|v| v.as_bool()) {
                        let _ = items.top.set_checked(on);
                    }
                    if let Some(on) = payload.get("snap_to_edge").and_then(|v| v.as_bool()) {
                        let _ = items.snap.set_checked(on);
                    }
                });
                // 预览窗（PL018.4）：托盘 hover 的 todo 前五列表——透明玻璃小窗。
                // alwaysOnTop = 悬浮层语义（诊断实测：非置顶 show 被活动窗盖住）。
                // 首绘方案（二轮修正）：屏外坐标创建 + visible(true) 让 webview 创建
                // 即完成首绘，build 后立即 hide——废弃"隐藏创建+show 预热"方案
                //（其屏幕外 show 被 Windows clamp 回可见区 = 左上残留窗 + "hover 出
                // 现两个"的根源；隐藏创建则首 hover 空窗）
                let preview = tauri::WebviewWindowBuilder::new(
                    app,
                    "tray-preview",
                    tauri::WebviewUrl::App("index.html".into()),
                )
                .title("CapsuleTODO 预览")
                .position(-2000.0, -2000.0)
                .inner_size(crate::tray::PREVIEW_WIDTH, 180.0)
                .transparent(true)
                .decorations(false)
                .skip_taskbar(true)
                .resizable(false)
                .visible(true)
                .focused(false)
                .always_on_top(true)
                .shadow(false) // 双下巴根因：无边框窗默认 DWM 投影，底部下沉似第二层
                .build()?;
                // DWM 系统圆角（PL018，用户问诊直角暗角）：DWM 切掉 CSS 圆角外的
                // 窗口直角区——预览窗视觉与内容圆角统一（Win11 原生浮窗同款）
                if let Ok(hwnd) = preview.hwnd() {
                    glass_backdrop::apply_round_corners(hwnd.0 as isize);
                }
                if let Err(err) = preview.hide() {
                    eprintln!("预览窗首绘后隐藏失败：{err}");
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // 位置记忆：关闭时保存（拖动中不写盘）。max_bubbles 从运行时设置透传，
            // 与位置共存一份 config.json（PL014.2）
            if let WindowEvent::CloseRequested { api, .. } = event {
                // 锁中毒跳过保存落日志（AGENTS 容错白名单登记项）：默认值写盘会静默
                // 覆盖用户已存设置——位置同弃（下次启动回默认位），磁盘现值不动
                match window.app_handle().state::<AppContext>().lock_settings() {
                    Ok(guard) => save_window_position(window, &guard),
                    Err(err) => eprintln!("设置锁中毒，窗口位置与气泡上限未保存：{err:?}"),
                }
                // R3 兜底腿（FIX004.19 T+R3 双腿）：拦默认关，600ms 后无条件销毁——
                // T 腿（前端 onCloseRequested flush 白板后自动销毁）健康时先到即秒关、
                // 进程随之退出本计时器不再触发；webview 卡死时 T 腿事件不可达，本腿
                // 保证任意状态 ≤600ms 必关（探针实测 .temp/close-probe/ 五场景）。
                // destroy 派发走 Rust 事件循环不依赖 webview 线程；先到腿销毁后的
                // 迟到 destroy 报错属预期竞态，落日志即止
                api.prevent_close();
                let win = window.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(600));
                    if let Err(err) = win.destroy() {
                        eprintln!("关窗兜底 destroy 失败（多为先到腿已销毁的预期竞态）：{err}");
                    }
                });
            }
            if let WindowEvent::Moved(_) = event {
                // 贴边吸附状态机（PL017.4）：Moved 更新时间戳；左键按住（真拖动，
                // 含悬停/慢速——静默判定会误吸）或间隔 ≤150ms 置 DRAG_ACTIVE；
                // pending 单线程守到"左键已松 + 静默 ≥150ms"→ 吸附一次
                let now = now_ms();
                let last = LAST_MOVED_MS.swap(now, Ordering::Relaxed);
                let dragging = now.saturating_sub(last) <= 150;
                #[cfg(target_os = "windows")]
                let dragging = dragging || win_input::left_down();
                if dragging {
                    DRAG_ACTIVE.store(true, Ordering::Relaxed);
                }
                if !DRAG_SNAP_PENDING.swap(true, Ordering::Relaxed) {
                    let win = window.clone();
                    std::thread::spawn(move || loop {
                        std::thread::sleep(Duration::from_millis(100));
                        #[cfg(target_os = "windows")]
                        if win_input::left_down() {
                            continue; // 左键仍按住 = 还在拖（悬停/慢速不误吸）
                        }
                        if now_ms().saturating_sub(LAST_MOVED_MS.load(Ordering::Relaxed)) < 150 {
                            continue; // 刚松手还在惯性/最后移动
                        }
                        DRAG_SNAP_PENDING.store(false, Ordering::Relaxed);
                        if DRAG_ACTIVE.swap(false, Ordering::Relaxed) {
                            snap_if_needed(&win);
                        }
                        break;
                    });
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
