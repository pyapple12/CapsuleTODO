//! 托盘弹出物与偏好切换逻辑（PL018→PL021）：右键菜单窗的贴边/置顶勾选点击
//! 落库/生效/同步处理 + hover 预览窗显隐联动 + 自绘菜单窗显隐。勾选态唯一
//! 事实源 = AppContext + config.json；三入口（设置板 / 托盘菜单窗 / 事件回显）
//! 同源同步。菜单窗（PL021）= 自绘 HTML 窗替代原生 Win32 菜单——深色模式下
//! Win11 圆角边距被 muda 深色自绘涂实 = 上下空条，原生无尺寸口；点外收起 =
//! 专用守候（按键 + 光标判定），菜单存亡事实源 = 菜单窗可见性（守候锁死判定
//! 与 Enter 忽略共用）。
//! 预览窗显隐 = 单一常驻守候线程 + 状态机（Idle/Armed/Shown/Suppressed）：
//! Enter 武装（图标上持续停留 500ms 才显示，瞬间划过不打扰），离开两区域
//! 400ms 隐藏；菜单开 = 锁死（Suppressed），收起后需新 Enter 重新武装。
//! 预览 Shown 态的显隐只由守候线程执行（show/hide 唯一调用者），事件处理只写
//! 状态——杜绝"线程被换掉后没人管窗"的孤儿窗竞态；右键收窗例外（FIX009.9
//! 注记：on_tray_right_button 事件路径直调 hide，属事件侧的确定性收窗）。
//! **窗口调用（含查询）一律锁外**——Tauri 非主线程窗口调用同步等待主线程，
//! 锁内调用 = 互挂死锁（实测两轮）。

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use tauri::Manager;

use crate::commands::AppContext;

/// 守候线程预览窗连续缺席拍数（退出口计数：满 50 拍 = 5s 退出）
static ABSENT_TICKS: AtomicU32 = AtomicU32::new(0);

/// 预览窗宽度（逻辑像素，用户口头调参单一来源：创建与 resize 共用）
pub const PREVIEW_WIDTH: f64 = 200.0;

/// 预览窗创建高度（逻辑像素，FIX005.12 单源：高度随内容动态调整，此值仅创建
/// 初始尺寸——首绘后由 tray_preview_resize 按内容重设）
pub const PREVIEW_HEIGHT: f64 = 180.0;

/// 菜单窗宽度（逻辑像素，用户口头调参单一来源）：四项中文文本 + 勾选列 +
/// 内距的紧凑自定宽
pub const MENU_WIDTH: f64 = 100.0;

/// 菜单窗高度（逻辑像素）：四项 × 行高 34 + 上下内距 10
pub const MENU_HEIGHT: f64 = 146.0;

/// 弹出物与托盘图标的锚定间距（物理 px，FIX007.13 单源：菜单窗/预览窗两锚定共用）
const ANCHOR_MARGIN: i32 = 8;

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

/// 守候单步的窗口动作（预览 Shown 态显隐的唯一调用者 = 守候线程；右键收窗为
/// 事件侧确定性例外，见模块注记；Debug 供测试）
#[derive(Debug, Clone, Copy, PartialEq)]
enum WatchAction {
    None,
    Show,
    Hide,
}

/// 守候状态（唯一事实源）：相位 + 托盘图标矩形锚点（定位 / resize 后底缘锚定
/// / 光标判定共用；宽高为 Enter 事件真实值——FIX005.11 替代硬编码 50×60，DPI
/// 缩放下真实矩形大于旧硬编码值，命中区不再偏小）。事件处理（Enter/右键）与
/// 守候线程经 WATCH 互斥锁读写
struct WatchState {
    phase: Phase,
    tray_top: i32,
    tray_right: i32,
    tray_bottom: i32,
    tray_width: i32,
    tray_height: i32,
}

static WATCH: Mutex<WatchState> = Mutex::new(WatchState {
    phase: Phase::Idle,
    tray_top: 0,
    tray_right: 0,
    tray_bottom: 0,
    tray_width: 0,
    tray_height: 0,
});

/// WATCH 取锁（锁中毒恢复取值——持锁线程 panic 后内部数据仍完整，取值优于卡死）
fn lock_watch() -> std::sync::MutexGuard<'static, WatchState> {
    WATCH.lock().unwrap_or_else(|e| e.into_inner())
}

/// 当前时刻（Unix 毫秒）——Armed 停留计时的时钟源（FIX005.10 单点：lib.rs 吸附
/// 防抖共用此实现，删除其本地副本）
pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 托盘菜单勾选点击（原 PL018.2，PL021 起由 tray_menu_action 命令调用）：
/// 读当前值取反 → 复用命令核心落库 → 即时生效（置顶 = set_always_on_top；
/// 吸附 = 事件层自动）→ emit prefs-changed 全量广播（菜单窗监听回显勾选态）。
/// 任一步失败落日志，勾选态由下次事实源读取纠正（不做本地回滚——ctx 未变即
/// 事实未变）
pub fn on_pref_menu(app: &tauri::AppHandle, id: &str) {
    // FIX005.15 单 match：目标值计算与执行同臂（新增偏好开关只需加一个臂）。
    // FIX006.6 置顶臂对齐 FIX005.6 命令壳层顺序：先主窗 set_always_on_top 成功
    // 再落库——窗口失败/主窗缺失路径不动 ctx/磁盘（消除状态分叉）
    let ctx = app.state::<AppContext>();
    let path = match crate::paths::settings_path() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("托盘开关切换失败（定位设置文件）：{err}");
            return;
        }
    };
    let result = match id {
        "tray-snap" => {
            let on = !snap_current(app);
            crate::commands::settings::settings_set_snap_to_edge_core(on, &path, &ctx).map(|_| ())
        }
        "tray-top" => {
            let on = !top_current(app);
            let Some(w) = app.get_webview_window("main") else {
                eprintln!("托盘置顶切换失败：主窗不存在（异常态）");
                return;
            };
            if let Err(err) = w.set_always_on_top(on) {
                eprintln!("托盘置顶切换生效失败：{err}");
                return;
            }
            crate::commands::settings::settings_set_always_on_top_core(on, &path, &ctx)
        }
        _ => return,
    };
    if let Err(err) = result {
        eprintln!("托盘开关切换失败（落库未变）：{err}");
        return;
    }
    // 全量广播（FIX005.9 单源：settings.rs emit_prefs_changed 统一快照与载荷）
    crate::commands::settings::emit_prefs_changed(app, &ctx);
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
            let next = if menu_open { phase } else { Phase::Idle };
            if window_visible {
                // FIX008.3：Suppressed 期可见窗一律收走（复核通过到 show 落地间
                // 右键竞入 → show 后到，预览伴菜单滞留至下次 hover 的根因）——
                // "菜单开∧窗可见"与"解除瞬间残留"两形态同根，结算相位后顺手收
                return (next, WatchAction::Hide);
            }
            // 菜单开着保持锁死 / 菜单已收解除（需新 Enter 重新武装）
            (next, WatchAction::None)
        }
    }
}

/// 托盘 Enter（预览延迟出现）：菜单开着则整个事件忽略（实测：菜单开着时托盘
/// Enter 仍会触发——此时武装 = 菜单旁弹出预览窗，即旧 bug）。相位分派（FIX006.18
/// 连续性语义）：Shown（窗正显示，光标自预览窗移回图标）仅更新锚点保持显示——
/// 重武装会"Armed∧可见→Hide"闪没 0.5s 重现；Idle/Suppressed 才转 Armed 重新
/// 计时（持续停留 500ms 防划过误触）。
/// x/y/w/h 为托盘图标矩形物理坐标（真实值随 DPI 缩放，FIX005.11 起全量入状态）
pub fn on_tray_enter(app: &tauri::AppHandle, rx: i32, ry: i32, rw: i32, rh: i32) {
    if menu_is_open(app) {
        return; // 菜单开着：不解锁、不武装（解除只走守候线程的菜单存亡探测）
    }
    let mut st = lock_watch();
    st.tray_top = ry;
    st.tray_right = rx + rw;
    st.tray_bottom = ry + rh;
    st.tray_width = rw;
    st.tray_height = rh;
    match st.phase {
        // Shown 连续性：光标在两区域内移动，保持显示（锚点已更新）
        Phase::Shown { ref mut miss } => *miss = 0,
        _ => st.phase = Phase::Armed { since_ms: now_ms() },
    }
}

/// 托盘右键按下（PL021 自绘菜单版）：菜单已开 = 再按收起（原生菜单同款）；
/// 未开 = 记录图标矩形锚点 + 锁死预览窗（Suppressed）+ 收预览窗 + 锚定显示
/// 菜单窗 + 起点外收起守候。主线程窗口调用同步执行，无跨线程死锁面
pub fn on_tray_right_button(app: &tauri::AppHandle, rx: i32, ry: i32, rw: i32, rh: i32) {
    let Some(menu_win) = app.get_webview_window("tray-menu") else {
        return;
    };
    if menu_win.is_visible().unwrap_or(false) {
        if let Err(err) = menu_win.hide() {
            eprintln!("菜单窗收起失败：{err}");
        }
        return; // 菜单开着再右键 = 收起；锁死由守候探测解除
    }
    {
        let mut st = lock_watch();
        st.tray_top = ry;
        st.tray_right = rx + rw;
        st.tray_bottom = ry + rh;
        st.tray_width = rw;
        st.tray_height = rh;
        st.phase = Phase::Suppressed;
    }
    if let Some(pw) = app.get_webview_window("tray-preview") {
        if let Err(err) = pw.hide() {
            eprintln!("右键收预览窗失败：{err}");
        }
    }
    // 菜单窗锚定（用户定案）：左缘对齐图标水平中轴向右展开（区别于预览窗的
    // 右对齐）；纵向同预览窗规则——默认图标上方，屏上缘放不下翻下方。屏右缘
    // 放不下整体左移贴边（图标靠屏幕最右时防出屏）
    let Ok(size) = menu_win.outer_size() else {
        return;
    };
    let mut px = rx + rw / 2; // 左缘 = 图标中轴
    let mut py = ry - size.height as i32 - ANCHOR_MARGIN; // 默认图标上方
    if py < 0 {
        py = ry + rh + ANCHOR_MARGIN; // 任务栏在顶部 → 翻下方
    }
    // 先落位再取所在屏（窗口在屏外 -2000 时 monitor 判定不可靠），超右缘左移
    if let Err(err) = menu_win.set_position(tauri::PhysicalPosition::new(px, py)) {
        eprintln!("菜单窗定位失败：{err}");
    }
    if let Ok(Some(mon)) = menu_win.current_monitor() {
        let screen_right = mon.position().x + mon.size().width as i32;
        if px + size.width as i32 > screen_right - ANCHOR_MARGIN {
            px = screen_right - ANCHOR_MARGIN - size.width as i32;
            if let Err(err) = menu_win.set_position(tauri::PhysicalPosition::new(px, py)) {
                eprintln!("菜单窗贴边修正失败：{err}");
            }
        }
    }
    if let Err(err) = menu_win.show() {
        eprintln!("菜单窗显示失败：{err}");
        return;
    }
    spawn_menu_watch(app);
}

/// 按托盘锚点定位预览窗（显示前调用）：右对齐托盘图标、默认上方，屏幕上缘
/// 放不下翻托盘下方（任务栏在顶部场景）。定位用实际窗尺寸（高度随内容动态
/// 调整，常量会让少条目时窗体悬空）；锚点未记录（全 0）则跳过。
/// 锚点参数化（FIX009.9 纠偏）= 调用方锁外持副本调用——锁内只拷值出锁，窗
/// 调用在锁外（死锁铁律正确形态；原注释"守候线程持锁调用"失实且具诱导性）。
/// 菜单窗（PL021）弹出锚定同款复用（尺寸不同但规则一致）
fn anchor_preview(w: &tauri::WebviewWindow, tray_top: i32, tray_right: i32, tray_bottom: i32) {
    if tray_top == 0 && tray_right == 0 {
        return;
    }
    let Ok(size) = w.outer_size() else {
        return;
    };
    let px = tray_right - size.width as i32; // 右对齐托盘图标
    let mut py = tray_top - size.height as i32 - ANCHOR_MARGIN; // 默认托盘上方
    if py < 0 {
        py = tray_bottom + ANCHOR_MARGIN; // 翻到下方
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

/// 预览守候线程（单一常驻，setup 启动；预览窗连续 5s 取不到即自灭退出）：100ms
/// 步进驱动状态机，是预览 Shown 态 show/hide 的唯一调用者——事件处理只写状态
/// （右键收窗例外见模块注记），杜绝多线程换代的孤儿窗。窗口查询与动作全在锁外
/// 执行（见循环内注释），锁内只做纯 FFI 与状态结算
pub fn spawn_preview_watcher(app: &tauri::AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        // 退出口（托盘幽灵图标修复配套）：预览窗连续 50 拍（5s）取不到 = app 在
        // 退出收尾（或创建失败），线程自灭不再空转——但不得卡启动时序（创建在
        // 同一 setup 段内先后执行，毫秒级完成，远小于 5s）
        let Some(w) = handle.get_webview_window("tray-preview") else {
            ABSENT_TICKS.fetch_add(1, Ordering::Relaxed);
            if ABSENT_TICKS.load(Ordering::Relaxed) >= 50 {
                return;
            }
            continue;
        };
        ABSENT_TICKS.store(0, Ordering::Relaxed);
        // 窗口查询与动作全部在锁外：Tauri 2 非主线程的窗口调用（含查询 is_visible/
        // outer_position/outer_size，走事件循环消息）都是同步等待主线程应答——持锁
        // 等待遇主线程等锁 = 互等死锁（实测两轮：set_position/show 持锁挂死一次、
        // is_visible 持锁首触即挂）。锁内只留纯 FFI（GetCursorPos 不派发消息）
        // 与状态结算
        let visible = w.is_visible().unwrap_or(false);
        let in_window = visible && cursor_in_window(&w);
        let menu_open = menu_is_open(&handle);
        let (action, anchors) = {
            let mut st = lock_watch();
            let in_tray = cursor_in_tray(st.tray_top, st.tray_right, st.tray_width, st.tray_height);
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
                // FIX007.17 竞态复核：状态提交（锁内）与窗口执行（锁外）之间有毫秒级
                // 空窗——主线程此间处理右键会置 Suppressed 并收窗，无复核则预览窗
                // 伴菜单弹出且无人收走（滞留至下次 hover）。复核"仍 Shown 且菜单
                // 未开"再显示，任一变了放弃（后续拍按当前相位处理）。
                // FIX008.1：拆两步——锁内只拷相位 bool，menu_is_open（is_visible
                // 同步等主线程）必须锁外调（复核块原写法持锁调窗 = 死锁铁律违规，
                // 触发窗口恰是本复核的目标竞态本身）；phase 已变时 || 短路免一次
                // 主线程往返。残余空窗由 step_watch Suppressed 臂窗可见 Hide 兜底
                let still_shown = {
                    let st = lock_watch();
                    matches!(st.phase, Phase::Shown { .. })
                };
                if !still_shown || menu_is_open(&handle) {
                    continue;
                }
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

/// 托盘菜单窗（PL021 自绘 HTML 窗）此刻是否开着——菜单存亡的唯一硬事实源，
/// 锁死判定（Suppressed 解除）与 Enter 忽略共用。原 #32768 原生菜单窗口探测
/// 随原生菜单弃用而移除。
/// **只允许锁外调用**：is_visible 走事件循环同步等待主线程应答，锁内调用 =
/// 互挂死锁（死锁铁律，见 spawn_preview_watcher 内注释）
fn menu_is_open(app: &tauri::AppHandle) -> bool {
    app.get_webview_window("tray-menu")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false)
}

/// 托盘菜单窗点外收起守候（PL021）：右键弹出时启动，100ms 步进——起步先等
/// 弹出那次右键松开（立即判定会"弹出即收"）；此后任一左/右键按下且光标不在
/// 菜单窗内 = 点外 → 收起退出。点菜单项光标在窗内不触发，菜单窗由
/// tray_menu_action 命令收起；窗口已不可见（动作路径已收）即自灭退出
///（FIX005.18 cfg 化：win_input 为 Windows 专属模块，非 Windows 无托盘语义
/// 给空实现，沿 cursor_in_tray 先例）
#[cfg(target_os = "windows")]
fn spawn_menu_watch(app: &tauri::AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || {
        // 宽限：等弹出用的右键松开再开始点外判定（轮询松手，不抢任何焦点）
        while crate::win_input::right_down() {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let Some(w) = handle.get_webview_window("tray-menu") else {
                return;
            };
            if !w.is_visible().unwrap_or(false) {
                return; // 菜单项动作已收起（命令路径）
            }
            let clicked_outside = (crate::win_input::left_down() || crate::win_input::right_down())
                && !cursor_in_window(&w);
            if clicked_outside {
                if let Err(err) = w.hide() {
                    eprintln!("菜单窗点外收起失败：{err}");
                }
                return;
            }
        }
    });
}

/// 非 Windows：无托盘语义，点外收起无实现面（编译期分支互不影响）
#[cfg(not(target_os = "windows"))]
fn spawn_menu_watch(_app: &tauri::AppHandle) {}

/// 光标是否落在托盘图标矩形内（守候循环判定：矩形四缘由 Enter 真实记录，
/// FIX005.11 参数化替代硬编码 50×60——DPI 缩放下硬编码命中区偏小）
fn cursor_in_tray(tray_top: i32, tray_right: i32, tray_width: i32, tray_height: i32) -> bool {
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
        // 图标矩形：右缘/顶缘已知，宽高为 Enter 真实值，左/下缘反推
        pt.x >= tray_right - tray_width
            && pt.x <= tray_right
            && pt.y >= tray_top
            && pt.y <= tray_top + tray_height
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (tray_top, tray_right, tray_width, tray_height);
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

    #[test]
    fn 锁死期_窗可见一律收走() {
        // FIX008.3：show 后到等线程交错致锁死期残留可见窗——菜单开着也收（预览
        // 伴菜单滞留的兜底）
        assert_eq!(
            step_watch(Phase::Suppressed, 5000, true, false, true, true),
            (Phase::Suppressed, WatchAction::Hide)
        );
        // 解除瞬间残留同根：结算 Idle 顺手收
        assert_eq!(
            step_watch(Phase::Suppressed, 5000, true, false, false, true),
            (Phase::Idle, WatchAction::Hide)
        );
    }
}
