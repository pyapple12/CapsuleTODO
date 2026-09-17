//! 白板命令：装载与保存（单块内容，操作即落库；核心抽自由函数直测内存库）。

use tauri::State;

use super::{AppContext, CommandError};
use crate::whiteboard::validate_content;

/// 装载白板内容（首启无行返回空串——正常态）
#[tauri::command]
pub fn whiteboard_load(ctx: State<'_, AppContext>) -> Result<String, CommandError> {
    whiteboard_load_core(&ctx)
}

/// whiteboard_load 核心实现
pub fn whiteboard_load_core(ctx: &AppContext) -> Result<String, CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.load_whiteboard()?)
}

/// 保存白板内容（超长严格拒绝）
#[tauri::command]
pub fn whiteboard_save(content: String, ctx: State<'_, AppContext>) -> Result<(), CommandError> {
    whiteboard_save_core(&content, &ctx)
}

/// whiteboard_save 核心实现：校验 + 落库
pub fn whiteboard_save_core(content: &str, ctx: &AppContext) -> Result<(), CommandError> {
    validate_content(content).map_err(|err| CommandError::Whiteboard(err.to_string()))?;
    let storage = ctx.lock_storage()?;
    Ok(storage.save_whiteboard(content)?)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::storage::Storage;

    /// 测试上下文：内存库（禁触真实用户数据）
    fn test_context() -> AppContext {
        AppContext {
            storage: Mutex::new(Storage::open_in_memory().expect("内存库必须可开")),
        }
    }

    #[test]
    fn load_default_empty_and_save_roundtrip() {
        let ctx = test_context();
        assert_eq!(whiteboard_load_core(&ctx).expect("读命令必须成功"), "");
        whiteboard_save_core("草稿", &ctx).expect("保存必须成功");
        assert_eq!(whiteboard_load_core(&ctx).expect("读命令必须成功"), "草稿");
    }

    #[test]
    fn save_overlong_is_visible_error() {
        let ctx = test_context();
        let overlong = "长".repeat(10_001);
        let err = whiteboard_save_core(&overlong, &ctx).expect_err("超长必须被拒");
        assert!(matches!(err, CommandError::Whiteboard(_)));
    }
}
