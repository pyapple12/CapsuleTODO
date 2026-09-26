//! Todo 清单业务纯逻辑：DTO、文本/笔记校验、龄期裁决（禁 import tauri，业务纯逻辑约束）。
//! PL002 起"单一事实源 = db"：存取与排序由 storage.rs（SQL）承载，本模块只剩契约与校验。
//! PL010 扩容：created_at/done_at/note 三字段 + AgeLevel 龄期提醒裁决（时间源注入可测）。

use serde::{Deserialize, Serialize};
use thiserror::Error;
/// 单条待办文本长度上限（按字符数；300px 小板可读性考量，可调）
pub const MAX_TEXT_LEN: usize = 100;

/// 笔记长度上限（详情板自由文本；10K 足够防失控，可调）
pub const MAX_NOTE_LEN: usize = 10_000;

/// 龄期提醒黄字阈值：24 小时（毫秒；与实验场 V0.010 定案同源）
pub const AGE_YELLOW_MS: i64 = 24 * 3600 * 1000;
/// 龄期提醒红字阈值：48 小时（毫秒）
pub const AGE_RED_MS: i64 = 48 * 3600 * 1000;

/// Todo 业务错误：文本/笔记非法
#[derive(Debug, Error)]
pub enum TodoError {
    /// 文本 trim 后为空
    #[error("待办文本为空")]
    EmptyText,
    /// 文本超出 MAX_TEXT_LEN 上限
    #[error("待办文本超长（上限 {MAX_TEXT_LEN} 字符）")]
    TooLong,
    /// 笔记超出 MAX_NOTE_LEN 上限
    #[error("笔记超长（上限 {MAX_NOTE_LEN} 字符）")]
    NoteTooLong,
}

/// 龄期提醒档位（前端据此渲染黄/红提醒行；无提醒 = None）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgeLevel {
    /// 无提醒（创建未满 24h 或无 created_at）
    None,
    /// 黄字：已超过 24 小时
    Yellow,
    /// 红字：已超过 48 小时
    Red,
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
    /// 创建时刻（ epoch 毫秒；存量迁移行 = NULL 不模拟时间）
    pub created_at: Option<i64>,
    /// 完成时刻（勾选置 now、退回清 NULL；归档板排序依据）
    pub done_at: Option<i64>,
    /// 详情板笔记（自由文本，允许空白）
    pub note: String,
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

/// 笔记校验：仅长度上限（允许空白——笔记语义不同于标题）
pub fn validate_note(note: &str) -> Result<(), TodoError> {
    if note.chars().count() > MAX_NOTE_LEN {
        return Err(TodoError::NoteTooLong);
    }
    Ok(())
}

/// 龄期裁决：created_at 缺席 = 无提醒；now−created 跨阈值升级黄/红（恰好等于阈值不升级）
pub fn age_level(created_at: Option<i64>, now: i64) -> AgeLevel {
    let Some(created) = created_at else {
        return AgeLevel::None;
    };
    let age = now - created;
    if age > AGE_RED_MS {
        AgeLevel::Red
    } else if age > AGE_YELLOW_MS {
        AgeLevel::Yellow
    } else {
        AgeLevel::None
    }
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

    // —— PL010.2 龄期裁决（TDD：时间全注入，零真实等待） ——

    const NOW: i64 = 1_700_000_000_000;

    #[test]
    fn no_created_at_means_no_reminder() {
        assert_eq!(age_level(None, NOW), AgeLevel::None);
    }

    #[test]
    fn fresh_item_has_no_reminder() {
        assert_eq!(age_level(Some(NOW - 1000), NOW), AgeLevel::None);
        // 恰好 24h = 不升级（严格大于才触发）
        assert_eq!(age_level(Some(NOW - AGE_YELLOW_MS), NOW), AgeLevel::None);
    }

    #[test]
    fn over_24h_is_yellow() {
        assert_eq!(
            age_level(Some(NOW - AGE_YELLOW_MS - 1), NOW),
            AgeLevel::Yellow
        );
        assert_eq!(
            age_level(Some(NOW - 30 * 3600 * 1000), NOW),
            AgeLevel::Yellow
        );
    }

    #[test]
    fn over_48h_is_red() {
        // 恰好 48h = 仍黄（红需严格大于）
        assert_eq!(age_level(Some(NOW - AGE_RED_MS), NOW), AgeLevel::Yellow);
        assert_eq!(age_level(Some(NOW - AGE_RED_MS - 1), NOW), AgeLevel::Red);
        assert_eq!(age_level(Some(NOW - 100 * 3600 * 1000), NOW), AgeLevel::Red);
    }

    // —— PL010.2 笔记校验 ——

    #[test]
    fn note_allows_blank_and_enforces_limit() {
        assert!(validate_note("").is_ok());
        assert!(validate_note("   \n 草稿 ").is_ok());
        assert!(validate_note(&"记".repeat(MAX_NOTE_LEN)).is_ok());
        assert!(matches!(
            validate_note(&"记".repeat(MAX_NOTE_LEN + 1)),
            Err(TodoError::NoteTooLong)
        ));
    }

    /// AgeLevel 序列化形态断言（serde 契约 = 前端 types.ts 镜像）
    #[test]
    fn age_level_serializes_as_plain_string() {
        assert_eq!(serde_json::to_string(&AgeLevel::None).unwrap(), "\"None\"");
        assert_eq!(
            serde_json::to_string(&AgeLevel::Yellow).unwrap(),
            "\"Yellow\""
        );
        assert_eq!(serde_json::to_string(&AgeLevel::Red).unwrap(), "\"Red\"");
    }

    /// Deserialize 需求占位（AgeLevel 前端只读不回传，derive 保持对称防未来需要）
    #[test]
    fn age_level_deserialize_roundtrip() {
        let v: AgeLevel = serde_json::from_str("\"Red\"").unwrap();
        assert_eq!(v, AgeLevel::Red);
    }
}
