//! 设置命令（PL014.2 气泡上限 + PL015 气泡热键 + PL017 窗口偏好）：读取与持久化
//!（config.json 与窗口位置共存）。上限钳制 1~MAX_BUBBLES_LIMIT 与 design stepper
//! 同规；热键 parse 校验 + 注册失败回滚；核心抽自由函数直测。

use tauri::{Emitter, Manager, State};

use super::{AppContext, CommandError};
use crate::settings::{clamp_max_bubbles, MAX_BUBBLES_LIMIT};

/// 读取气泡提醒上限
#[tauri::command]
pub fn settings_get_max_bubbles(ctx: State<'_, AppContext>) -> Result<u32, CommandError> {
    settings_get_max_bubbles_core(&ctx)
}

/// 配置路径解析收敛（FIX007.11）：四写命令共用同一两行习语，单点化后策略变更
///（如错误消息加语境）只改此处
fn settings_path_or_err() -> Result<std::path::PathBuf, CommandError> {
    crate::paths::settings_path().map_err(|err| CommandError::Settings(err.to_string()))
}

/// 读取气泡提醒上限的最大合法值（FIX005.25 单一来源出口：设置板步进禁用态与
/// mock 钳制经此拉取，替代三处硬编码——上限调整只改 settings::MAX_BUBBLES_LIMIT。
/// FIX006.12 返回值 Result 化对齐同文件约定；当前无失败路径，Ok 直返）
#[tauri::command]
pub fn settings_get_bubble_max_limit() -> Result<u32, CommandError> {
    Ok(MAX_BUBBLES_LIMIT)
}

/// settings_get_max_bubbles 核心实现：读运行时设置副本
pub fn settings_get_max_bubbles_core(ctx: &AppContext) -> Result<u32, CommandError> {
    let settings = ctx.lock_settings()?;
    Ok(settings.max_bubbles)
}

/// 写入气泡提醒上限（范围 1~MAX_BUBBLES_LIMIT 钳制，越界值静默收敛到边界——与
/// stepper 前端钳制同规）
#[tauri::command]
pub fn settings_set_max_bubbles(
    value: u32,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    let path = settings_path_or_err()?;
    settings_set_max_bubbles_core(value, &path, &ctx)
}

/// settings_set_max_bubbles 核心实现：钳制 + 更新运行时设置 + 原子落盘 config.json。
/// 落盘持 settings 锁（低频小写入）；锁序 storage → settings 单向，此处无反向。
/// path 由调用方注入（生产 = config.json 真实路径，测试 = 临时路径，禁触真实用户数据）
pub fn settings_set_max_bubbles_core(
    value: u32,
    path: &std::path::Path,
    ctx: &AppContext,
) -> Result<(), CommandError> {
    let clamped = clamp_max_bubbles(value);
    let mut settings = ctx.lock_settings()?;
    settings.max_bubbles = clamped;
    crate::settings::save(path, &settings).map_err(CommandError::from)?;
    Ok(())
}

/// 读取气泡捕获热键（PL015）
#[tauri::command]
pub fn settings_get_bubble_hotkey(ctx: State<'_, AppContext>) -> Result<String, CommandError> {
    settings_get_bubble_hotkey_core(&ctx)
}

/// settings_get_bubble_hotkey 核心实现：读运行时设置副本
pub fn settings_get_bubble_hotkey_core(ctx: &AppContext) -> Result<String, CommandError> {
    let settings = ctx.lock_settings()?;
    Ok(settings.bubble_hotkey.clone())
}

/// 写入气泡捕获热键（PL015）：parse 校验非法拒 → 落库 → 重注册全局热键；
/// 重注册失败（热键被占等）回滚落库旧热键并上抛——前端红字提示旧值已恢复
#[tauri::command]
pub fn settings_set_bubble_hotkey(
    combo: String,
    ctx: State<'_, AppContext>,
) -> Result<String, CommandError> {
    let path = settings_path_or_err()?;
    let old = settings_get_bubble_hotkey_core(&ctx)?;
    let normalized = settings_set_bubble_hotkey_core(&combo, &path, &ctx)?;
    let normalized_display = crate::hotkey::to_display(&normalized);
    #[cfg(target_os = "windows")]
    if let Err(err) = crate::hotkey::reregister(normalized) {
        // FIX010.6：回滚写前复核当前落库值仍为本轮——reregister 失败若因并发后
        // 发轮接管（代际失配致本线线程自灭），无条件回滚会覆盖后发轮已落库的新值
        // （UI 显新值磁盘为旧值的分叉）；接管则放弃回滚，锁再失败同样保守放弃
        let taken_over = ctx
            .lock_settings()
            .map(|s| s.bubble_hotkey != normalized_display)
            .unwrap_or(true);
        if taken_over {
            eprintln!("热键设置已被并发请求接管，放弃回滚落库（err：{err}）");
            return Err(CommandError::Hotkey(format!("热键注册失败：{err}")));
        }
        // 回滚：恢复旧热键落库 + 重注册（FIX005.16 去 expect：parse 失败属异常态，
        // 落日志跳过重注册——热键是"失败不阻断"容错域，禁业务 panic）
        if let Err(rollback_err) = settings_set_bubble_hotkey_core(&old, &path, &ctx) {
            eprintln!("热键回滚落库失败：{rollback_err}");
        }
        match crate::hotkey::parse(&old) {
            Ok(rollback) => {
                if let Err(re_err) = crate::hotkey::reregister(rollback) {
                    eprintln!("旧热键重注册失败：{re_err}");
                }
            }
            Err(parse_err) => {
                eprintln!("旧热键解析失败（跳过重注册，异常态）：{parse_err}");
            }
        }
        return Err(CommandError::Hotkey(format!(
            "热键注册失败（已回退 {old}）：{err}"
        )));
    }
    Ok(crate::hotkey::to_display(&normalized))
}

/// settings_set_bubble_hotkey 核心实现：parse 校验 + 规范化落库（不含重注册，
/// 命令壳负责；直测 = 临时路径内存库，禁触真实用户数据）
pub fn settings_set_bubble_hotkey_core(
    combo: &str,
    path: &std::path::Path,
    ctx: &AppContext,
) -> Result<crate::hotkey::HotkeyCombo, CommandError> {
    let normalized = crate::hotkey::parse(combo)?;
    let mut settings = ctx.lock_settings()?;
    settings.bubble_hotkey = crate::hotkey::to_display(&normalized);
    crate::settings::save(path, &settings).map_err(CommandError::from)?;
    Ok(normalized)
}

/// 读取窗口置顶开关（PL017）
#[tauri::command]
pub fn settings_get_always_on_top(ctx: State<'_, AppContext>) -> Result<bool, CommandError> {
    settings_get_always_on_top_core(&ctx)
}

/// settings_get_always_on_top 核心实现：读运行时设置副本
pub fn settings_get_always_on_top_core(ctx: &AppContext) -> Result<bool, CommandError> {
    Ok(ctx.lock_settings()?.always_on_top)
}

/// 写入窗口置顶开关（PL017；FIX005.6 调序）：主窗 set_always_on_top **成功后**才
/// 落库与广播——窗口操作失败路径不动 ctx/磁盘（消除"配置已开、窗口实际未置顶"
/// 的状态分叉，全屏让位线程读 ctx 不再基于错值行动）
#[tauri::command]
pub fn settings_set_always_on_top(
    on: bool,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| CommandError::Window("主窗口不存在".to_string()))?;
    window
        .set_always_on_top(on)
        .map_err(|err| CommandError::Window(format!("置顶切换失败：{err}")))?;
    let path = settings_path_or_err()?;
    settings_set_always_on_top_core(on, &path, &ctx)?;
    emit_prefs_changed(&app, &ctx);
    Ok(())
}

/// settings_set_always_on_top 核心实现：更新运行时设置 + 原子落盘（窗口操作在
/// 壳层；直测 = 临时路径内存库，禁触真实用户数据）
pub fn settings_set_always_on_top_core(
    on: bool,
    path: &std::path::Path,
    ctx: &AppContext,
) -> Result<(), CommandError> {
    let mut settings = ctx.lock_settings()?;
    settings.always_on_top = on;
    crate::settings::save(path, &settings).map_err(CommandError::from)?;
    Ok(())
}

/// 读取贴边吸附开关（PL017）
#[tauri::command]
pub fn settings_get_snap_to_edge(ctx: State<'_, AppContext>) -> Result<bool, CommandError> {
    settings_get_snap_to_edge_core(&ctx)
}

/// settings_get_snap_to_edge 核心实现：读运行时设置副本
pub fn settings_get_snap_to_edge_core(ctx: &AppContext) -> Result<bool, CommandError> {
    Ok(ctx.lock_settings()?.snap_to_edge)
}

/// 写入贴边吸附开关（PL017）：落库即生效（吸附在 Moved 事件层每轮读运行时设置，
/// 无窗口操作）→ 广播 prefs-changed（托盘菜单勾选态同步，PL018.3）
#[tauri::command]
pub fn settings_set_snap_to_edge(
    on: bool,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    let path = settings_path_or_err()?;
    settings_set_snap_to_edge_core(on, &path, &ctx)?;
    emit_prefs_changed(&app, &ctx);
    Ok(())
}

/// 窗口偏好广播载荷（FIX005.9 单一来源）：prefs-changed 的 serde 结构，替代
/// 裸 json! 拼键——键名漂移从此编译期可查，两端共享此定义
#[derive(Clone, serde::Serialize)]
pub(crate) struct PrefsSnapshot {
    /// 窗口置顶开关
    pub always_on_top: bool,
    /// 贴边吸附开关
    pub snap_to_edge: bool,
}

/// 全量广播窗口偏好（PL018.3；FIX005.9 提 pub(crate) 供 tray.rs 单源复用）：
/// 设置板/托盘菜单切换后各入口同步勾选态与回显。单锁取快照（消除 tray.rs 旧
/// 实现两次取锁的半新半旧窗口）
pub(crate) fn emit_prefs_changed(app: &tauri::AppHandle, ctx: &AppContext) {
    let snapshot = match ctx.lock_settings() {
        Ok(s) => PrefsSnapshot {
            always_on_top: s.always_on_top,
            snap_to_edge: s.snap_to_edge,
        },
        Err(err) => {
            eprintln!("prefs-changed 读取失败（跳过广播）：{err:?}");
            return;
        }
    };
    if let Err(err) = app.emit("prefs-changed", snapshot) {
        eprintln!("prefs-changed 广播失败：{err}");
    }
}

/// settings_set_snap_to_edge 核心实现：更新运行时设置 + 原子落盘
pub fn settings_set_snap_to_edge_core(
    on: bool,
    path: &std::path::Path,
    ctx: &AppContext,
) -> Result<(), CommandError> {
    let mut settings = ctx.lock_settings()?;
    settings.snap_to_edge = on;
    crate::settings::save(path, &settings).map_err(CommandError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::settings::DEFAULT_MAX_BUBBLES;
    use crate::storage::Storage;

    /// 测试上下文：内存库 + 默认设置
    fn test_context() -> AppContext {
        AppContext {
            storage: Mutex::new(Storage::open_in_memory().expect("内存库必须可开")),
            settings: Mutex::new(crate::settings::WindowSettings::default()),
        }
    }

    #[test]
    fn get_returns_default_and_set_persists_in_context() {
        let tmp =
            std::env::temp_dir().join(format!("capsule-settings-cmd-{}.json", std::process::id()));
        let ctx = test_context();
        assert_eq!(
            settings_get_max_bubbles_core(&ctx).expect("读必须成功"),
            DEFAULT_MAX_BUBBLES
        );
        settings_set_max_bubbles_core(8, &tmp, &ctx).expect("写必须成功");
        assert_eq!(settings_get_max_bubbles_core(&ctx).expect("读必须成功"), 8);
        std::fs::remove_file(&tmp).expect("清理必须成功");
    }

    #[test]
    fn set_clamps_out_of_range() {
        let ctx = test_context();
        let tmp = std::env::temp_dir().join(format!(
            "capsule-settings-clamp-{}.json",
            std::process::id()
        ));
        settings_set_max_bubbles_core(0, &tmp, &ctx).expect("越下界钳到 1");
        assert_eq!(ctx.lock_settings().expect("锁").max_bubbles, 1);
        settings_set_max_bubbles_core(99, &tmp, &ctx).expect("越上界钳到 20");
        assert_eq!(ctx.lock_settings().expect("锁").max_bubbles, 20);
        std::fs::remove_file(&tmp).expect("清理必须成功");
    }

    #[test]
    fn set_persists_to_disk() {
        // 落盘路径注入临时目录：写后重读文件确认持久化（禁触真实用户数据）
        let tmp =
            std::env::temp_dir().join(format!("capsule-settings-disk-{}.json", std::process::id()));
        let ctx = test_context();
        settings_set_max_bubbles_core(12, &tmp, &ctx).expect("写必须成功");
        let loaded = crate::settings::load(&tmp)
            .expect("读取必须成功")
            .expect("文件必须存在");
        assert_eq!(loaded.max_bubbles, 12);
        std::fs::remove_file(&tmp).expect("清理必须成功");
    }

    #[test]
    fn hotkey_set_normalizes_and_persists() {
        // PL015.3：热键 set 落库规范化显示串（不含重注册——壳层职责）
        let tmp =
            std::env::temp_dir().join(format!("capsule-hotkey-set-{}.json", std::process::id()));
        let ctx = test_context();
        let normalized =
            settings_set_bubble_hotkey_core("alt+ctrl+c", &tmp, &ctx).expect("合法组合必须成功");
        let settings = ctx.lock_settings().expect("锁必须成功");
        assert_eq!(settings.bubble_hotkey, "Ctrl+Alt+C", "落库规范化串");
        assert_eq!(normalized.vk, b'C' as u32);
        let loaded = crate::settings::load(&tmp)
            .expect("读取必须成功")
            .expect("文件必须存在");
        assert_eq!(loaded.bubble_hotkey, "Ctrl+Alt+C");
        std::fs::remove_file(&tmp).expect("清理必须成功");
    }

    #[test]
    fn hotkey_set_rejects_illegal_combo() {
        // PL015.3：非法组合 parse 拒绝（HotkeyError 上抛），落库不动
        let tmp =
            std::env::temp_dir().join(format!("capsule-hotkey-bad-{}.json", std::process::id()));
        let ctx = test_context();
        let err = settings_set_bubble_hotkey_core("Space", &tmp, &ctx).expect_err("未知键必须被拒");
        assert!(matches!(err, CommandError::Hotkey(_)));
        assert_eq!(
            ctx.lock_settings().expect("锁").bubble_hotkey,
            crate::settings::DEFAULT_BUBBLE_HOTKEY.to_string(),
            "落库不动"
        );
    }

    #[test]
    fn window_prefs_set_and_get_roundtrip() {
        // PL017.3：置顶/吸附开关 core 直测——set 落库 + get 回读一致（临时路径）
        let tmp =
            std::env::temp_dir().join(format!("capsule-prefs-cmd-{}.json", std::process::id()));
        let ctx = test_context();
        assert!(
            settings_get_always_on_top_core(&ctx).expect("读必须成功"),
            "默认置顶开"
        );
        assert!(
            settings_get_snap_to_edge_core(&ctx).expect("读必须成功"),
            "默认吸附开"
        );
        settings_set_always_on_top_core(false, &tmp, &ctx).expect("写必须成功");
        settings_set_snap_to_edge_core(false, &tmp, &ctx).expect("写必须成功");
        assert!(!settings_get_always_on_top_core(&ctx).expect("读必须成功"));
        assert!(!settings_get_snap_to_edge_core(&ctx).expect("读必须成功"));
        // 落盘重读确认持久化
        let loaded = crate::settings::load(&tmp)
            .expect("读取必须成功")
            .expect("文件必须存在");
        assert!(!loaded.always_on_top && !loaded.snap_to_edge);
        std::fs::remove_file(&tmp).expect("清理必须成功");
    }
}
