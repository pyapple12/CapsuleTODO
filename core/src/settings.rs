//! 运行时设置持久化（PL003 窗口位置 + PL014.2 气泡提醒上限 + PL015 气泡热键）：
//! JSON 原子写（同目录 .tmp 写入 + rename 替换，失败清理临时文件）。尺寸固定
//! 300×400 不入配置，仅记位置；文件不存在 = 首启正常态（白名单③回默认位）。

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 气泡提醒上限缺省值（serde default：旧 config.json 缺字段兼容）
pub const DEFAULT_MAX_BUBBLES: u32 = 5;

/// 气泡捕获全局热键缺省值（PL015：serde default 旧 config.json 缺字段兼容）
pub const DEFAULT_BUBBLE_HOTKEY: &str = "Ctrl+Alt+C";

/// 气泡提醒上限合法区间钳制（1~20，与设置板步进同规）：读路径（config.json 加载点）
/// 与写路径（settings_set 命令）共用单一来源——手改文件越界静默收敛到边界（白名单⑤）
pub fn clamp_max_bubbles(value: u32) -> u32 {
    value.clamp(1, 20)
}

fn default_max_bubbles() -> u32 {
    DEFAULT_MAX_BUBBLES
}

fn default_bubble_hotkey() -> String {
    DEFAULT_BUBBLE_HOTKEY.to_string()
}

/// 运行时设置（窗口位置 + 气泡提醒上限 + 气泡热键；serde default 容忍手改缺字段）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WindowSettings {
    /// 窗口左上角 x（屏幕物理坐标）
    #[serde(default)]
    pub x: i32,
    /// 窗口左上角 y（屏幕物理坐标）
    #[serde(default)]
    pub y: i32,
    /// 气泡提醒上限（PL014.2；旧 config.json 缺字段回填 5）
    #[serde(default = "default_max_bubbles")]
    pub max_bubbles: u32,
    /// 气泡捕获全局热键（PL015；旧 config.json 缺字段回填 Ctrl+Alt+C）
    #[serde(default = "default_bubble_hotkey")]
    pub bubble_hotkey: String,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            max_bubbles: DEFAULT_MAX_BUBBLES,
            bubble_hotkey: DEFAULT_BUBBLE_HOTKEY.to_string(),
        }
    }
}

/// 设置持久化错误
#[derive(Debug, Error)]
pub enum SettingsError {
    /// JSON 解析失败（严格报错，不静默回默认）
    #[error("JSON 解析失败：{0}")]
    Json(#[from] serde_json::Error),
    /// IO 失败（读写/替换）
    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),
}

/// 读取窗口设置；文件不存在返回 Ok(None)（首启正常态，白名单③），其余错误严格上抛
pub fn load(path: &Path) -> Result<Option<WindowSettings>, SettingsError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(SettingsError::Io(err)),
    };
    let mut settings: WindowSettings = serde_json::from_str(&text)?;
    // 热键规范化（PL015 白名单⑥）：用户手改 config.json 非法热键静默回默认，
    // 不崩常驻应用（与 max_bubbles 越界钳制同款纪律）
    if crate::hotkey::parse(&settings.bubble_hotkey).is_err() {
        eprintln!(
            "config.json 气泡热键非法（{}），回默认 {DEFAULT_BUBBLE_HOTKEY}",
            settings.bubble_hotkey
        );
        settings.bubble_hotkey = default_bubble_hotkey();
    }
    Ok(Some(settings))
}

/// 保存窗口设置（原子写：同目录 .tmp 写入 + rename 替换；rename 失败清理临时文件后上抛）
pub fn save(path: &Path, settings: &WindowSettings) -> Result<(), SettingsError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(settings)?)?;
    if let Err(err) = std::fs::rename(&tmp, path) {
        if let Err(cleanup) = std::fs::remove_file(&tmp) {
            eprintln!("设置临时文件清理失败：{cleanup}");
        }
        return Err(SettingsError::Io(err));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "capsule-todo-settings-{}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn load_missing_file_returns_none() {
        let path = temp_path("missing.json");
        let _ = std::fs::remove_file(&path);
        assert!(matches!(load(&path), Ok(None)));
    }

    #[test]
    fn save_load_roundtrip() {
        let path = temp_path("roundtrip.json");
        save(
            &path,
            &WindowSettings {
                x: 120,
                y: -40,
                max_bubbles: 5,
                bubble_hotkey: DEFAULT_BUBBLE_HOTKEY.to_string(),
            },
        )
        .expect("保存必须成功");
        let loaded = load(&path).expect("读取必须成功").expect("文件必须存在");
        assert_eq!(loaded.x, 120);
        assert_eq!(loaded.y, -40);
        assert_eq!(loaded.max_bubbles, 5);
        std::fs::remove_file(&path).expect("清理必须成功");
    }

    #[test]
    fn corrupted_json_is_strict_error() {
        let path = temp_path("corrupt.json");
        std::fs::write(&path, "{not json").expect("写入必须成功");
        assert!(matches!(load(&path), Err(SettingsError::Json(_))));
        std::fs::remove_file(&path).expect("清理必须成功");
    }

    #[test]
    fn save_creates_parent_dirs() {
        let root =
            std::env::temp_dir().join(format!("capsule-todo-settings-dir-{}", std::process::id()));
        let path = root.join("configs").join("config.json");
        save(
            &path,
            &WindowSettings {
                x: 1,
                y: 2,
                max_bubbles: 5,
                bubble_hotkey: DEFAULT_BUBBLE_HOTKEY.to_string(),
            },
        )
        .expect("保存必须成功（父目录自建）");
        assert!(path.exists());
        std::fs::remove_dir_all(&root).expect("清理必须成功");
    }

    #[test]
    fn missing_max_bubbles_field_backfills_default() {
        // PL014.2：旧 config.json 缺 max_bubbles 字段 → serde default 回填 5，不崩
        let path = temp_path("legacy-no-max.json");
        std::fs::write(&path, r#"{"x": 10, "y": 20}"#).expect("写入必须成功");
        let loaded = load(&path).expect("读取必须成功").expect("文件必须存在");
        assert_eq!(loaded.max_bubbles, DEFAULT_MAX_BUBBLES);
        assert_eq!(loaded.x, 10);
        std::fs::remove_file(&path).expect("清理必须成功");
    }

    #[test]
    fn default_impl_uses_five() {
        assert_eq!(WindowSettings::default().max_bubbles, DEFAULT_MAX_BUBBLES);
    }

    #[test]
    fn clamp_max_bubbles_bounds() {
        // FIX003.8：共享钳制单一来源（1~20），读写两路径同规
        assert_eq!(clamp_max_bubbles(0), 1);
        assert_eq!(clamp_max_bubbles(1), 1);
        assert_eq!(clamp_max_bubbles(20), 20);
        assert_eq!(clamp_max_bubbles(99), 20);
    }

    #[test]
    fn out_of_range_config_field_clamps_on_load() {
        // FIX003.8 读路径闭环：手改 config.json max_bubbles:0 → 加载点钳 1（模拟
        // lib.rs 装配调用方式：load 后经共享钳制函数再入运行时副本）
        let path = temp_path("oob-max.json");
        std::fs::write(&path, r#"{"x": 3, "y": 4, "max_bubbles": 0}"#).expect("写入必须成功");
        let loaded = load(&path).expect("读取必须成功").expect("文件必须存在");
        assert_eq!(clamp_max_bubbles(loaded.max_bubbles), 1);
        std::fs::remove_file(&path).expect("清理必须成功");
    }

    #[test]
    fn bubble_hotkey_missing_field_backfills_default() {
        // PL015.2：旧 config.json 缺 bubble_hotkey 字段 → serde default 回填 Ctrl+Alt+C
        let path = temp_path("hotkey-missing.json");
        std::fs::write(&path, r#"{"x": 1, "y": 2, "max_bubbles": 5}"#).expect("写入必须成功");
        let loaded = load(&path).expect("读取必须成功").expect("文件必须存在");
        assert_eq!(loaded.bubble_hotkey, DEFAULT_BUBBLE_HOTKEY);
        std::fs::remove_file(&path).expect("清理必须成功");
    }

    #[test]
    fn bubble_hotkey_valid_value_preserved() {
        // PL015.2：合法自定义热键原样保留
        let path = temp_path("hotkey-valid.json");
        std::fs::write(&path, r#"{"bubble_hotkey": "Ctrl+Shift+9"}"#).expect("写入必须成功");
        let loaded = load(&path).expect("读取必须成功").expect("文件必须存在");
        assert_eq!(loaded.bubble_hotkey, "Ctrl+Shift+9");
        std::fs::remove_file(&path).expect("清理必须成功");
    }

    #[test]
    fn bubble_hotkey_invalid_falls_back_to_default() {
        // PL015.2 白名单⑥：手改非法热键 → 加载点静默回默认，不崩不报错
        let path = temp_path("hotkey-invalid.json");
        std::fs::write(&path, r#"{"bubble_hotkey": "Space+鼠标中键"}"#).expect("写入必须成功");
        let loaded = load(&path).expect("读取必须成功").expect("文件必须存在");
        assert_eq!(loaded.bubble_hotkey, DEFAULT_BUBBLE_HOTKEY);
        std::fs::remove_file(&path).expect("清理必须成功");
    }
}
