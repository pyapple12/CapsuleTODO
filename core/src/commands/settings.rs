//! 设置命令（PL014.2 气泡上限 + PL015 气泡热键）：读取与持久化（config.json 与
//! 窗口位置共存）。上限钳制 1~20 与 design stepper 同规；热键 parse 校验 + 注册
//! 失败回滚；核心抽自由函数直测。

use tauri::State;

use super::{AppContext, CommandError};
use crate::settings::clamp_max_bubbles;

/// 读取气泡提醒上限
#[tauri::command]
pub fn settings_get_max_bubbles(ctx: State<'_, AppContext>) -> Result<u32, CommandError> {
    settings_get_max_bubbles_core(&ctx)
}

/// settings_get_max_bubbles 核心实现：读运行时设置副本
pub fn settings_get_max_bubbles_core(ctx: &AppContext) -> Result<u32, CommandError> {
    let settings = ctx.lock_settings()?;
    Ok(settings.max_bubbles)
}

/// 写入气泡提醒上限（范围 1~20 钳制，越界值静默收敛到边界——与 stepper 前端钳制同规）
#[tauri::command]
pub fn settings_set_max_bubbles(
    value: u32,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    let path =
        crate::paths::settings_path().map_err(|err| CommandError::Settings(err.to_string()))?;
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
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<String, CommandError> {
    let path =
        crate::paths::settings_path().map_err(|err| CommandError::Settings(err.to_string()))?;
    let old = settings_get_bubble_hotkey_core(&ctx)?;
    let normalized = settings_set_bubble_hotkey_core(&combo, &path, &ctx)?;
    #[cfg(target_os = "windows")]
    if let Err(err) = crate::hotkey::reregister(app.clone(), normalized) {
        // 回滚：恢复旧热键落库 + 重注册（旧热键此前注册成功，二次失败仅落日志）
        let _ = settings_set_bubble_hotkey_core(&old, &path, &ctx);
        let rollback = crate::hotkey::parse(&old).expect("旧热键必合法");
        if let Err(re_err) = crate::hotkey::reregister(app, rollback) {
            eprintln!("旧热键重注册失败：{re_err}");
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
}
