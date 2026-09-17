//! 白板纯逻辑：DTO（无——白板为单块内容）与内容长度校验（禁 import tauri，业务纯逻辑约束）。
//! 单一事实源 = db：白板读写由 storage.rs 单行表承载，本模块只剩契约与校验。

use thiserror::Error;

/// 白板内容长度上限（按字符数；防呆软上限，可调）
pub const MAX_CONTENT_LEN: usize = 10_000;

/// 白板业务错误：内容非法
#[derive(Debug, Error)]
pub enum WhiteboardError {
    /// 内容超出 MAX_CONTENT_LEN 上限
    #[error("白板内容过长（上限 {MAX_CONTENT_LEN} 字符）")]
    TooLong,
}

/// 白板内容校验：不超过 MAX_CONTENT_LEN（按字符计；空内容合法 = 清空白板）
pub fn validate_content(content: &str) -> Result<(), WhiteboardError> {
    if content.chars().count() > MAX_CONTENT_LEN {
        return Err(WhiteboardError::TooLong);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_within_limit_accepted() {
        assert!(validate_content("随手记点什么").is_ok());
        assert!(validate_content("").is_ok());
        assert!(validate_content(&"字".repeat(MAX_CONTENT_LEN)).is_ok());
    }

    #[test]
    fn overlong_content_rejected() {
        assert!(matches!(
            validate_content(&"长".repeat(MAX_CONTENT_LEN + 1)),
            Err(WhiteboardError::TooLong)
        ));
    }
}
