//! Todo 清单业务纯逻辑：DTO 与文本校验规则（禁 import tauri，业务纯逻辑约束）。
//! PL002 起"单一事实源 = db"：存取与排序由 storage.rs（SQL）承载，本模块只剩契约与校验。

use serde::Serialize;
use thiserror::Error;

/// 单条待办文本长度上限（按字符数；300px 小板可读性考量，可调）
pub const MAX_TEXT_LEN: usize = 100;

/// Todo 业务错误：文本非法
#[derive(Debug, Error)]
pub enum TodoError {
    /// 文本 trim 后为空
    #[error("待办文本为空")]
    EmptyText,
    /// 文本超出 MAX_TEXT_LEN 上限
    #[error("待办文本超长（上限 {MAX_TEXT_LEN} 字符）")]
    TooLong,
}

/// 单条待办（跨进程 DTO：serde 为前端 types.ts 的契约单一来源）
#[derive(Debug, Clone, Serialize)]
pub struct TodoItem {
    /// 条目唯一标识（库内自增主键，创建序）
    pub id: i64,
    /// 待办文本
    pub text: String,
    /// 完成态
    pub done: bool,
}

/// 文本校验：trim 后非空且不超过 MAX_TEXT_LEN（按字符计）
pub fn validate_text(text: &str) -> Result<(), TodoError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(TodoError::EmptyText);
    }
    if trimmed.chars().count() > MAX_TEXT_LEN {
        return Err(TodoError::TooLong);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_text_rejected() {
        assert!(matches!(validate_text(""), Err(TodoError::EmptyText)));
        assert!(matches!(validate_text("   "), Err(TodoError::EmptyText)));
    }

    #[test]
    fn overlong_text_rejected_and_limit_accepted() {
        assert!(matches!(
            validate_text(&"长".repeat(MAX_TEXT_LEN + 1)),
            Err(TodoError::TooLong)
        ));
        assert!(validate_text(&"字".repeat(MAX_TEXT_LEN)).is_ok());
    }

    #[test]
    fn normal_text_accepted() {
        assert!(validate_text("  买牛奶  ").is_ok());
    }
}
