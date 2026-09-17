//! 窗口设置持久化（PL003）：JSON 原子写（同目录 .tmp 写入 + rename 替换，失败清理临时文件）。
//! 尺寸固定 300×400 不入配置，仅记位置；文件不存在 = 首启正常态（白名单③回默认位）。

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 窗口设置（serde default 容忍手改缺字段，配合启动越界兜底）
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct WindowSettings {
    /// 窗口左上角 x（屏幕物理坐标）
    #[serde(default)]
    pub x: i32,
    /// 窗口左上角 y（屏幕物理坐标）
    #[serde(default)]
    pub y: i32,
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
    Ok(serde_json::from_str(&text)?)
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
        save(&path, &WindowSettings { x: 120, y: -40 }).expect("保存必须成功");
        let loaded = load(&path).expect("读取必须成功").expect("文件必须存在");
        assert_eq!(loaded.x, 120);
        assert_eq!(loaded.y, -40);
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
        save(&path, &WindowSettings { x: 1, y: 2 }).expect("保存必须成功（父目录自建）");
        assert!(path.exists());
        std::fs::remove_dir_all(&root).expect("清理必须成功");
    }
}
