//! 气泡纯逻辑：DTO、满额提醒判定与捕获文本校验（禁 import tauri，业务纯逻辑约束）。
//! 单一事实源 = db：气泡存取由 storage.rs 承载，本模块只剩契约、阈值与校验。

use serde::Serialize;
use thiserror::Error;

/// 气泡文本长度上限（按字符数；A002-P2-2 定案——气泡 = 短片段语义，防超长剪贴板无界入库，可调）
pub const MAX_BUBBLE_TEXT_LEN: usize = 2_000;

/// 气泡业务错误：捕获文本非法
#[derive(Debug, Error)]
pub enum BubbleError {
    /// 文本 trim 后为空（空剪贴板已在上游拦截，此为兜底校验）
    #[error("气泡内容为空")]
    EmptyText,
    /// 文本超出 MAX_BUBBLE_TEXT_LEN 上限
    #[error("气泡内容过长（上限 {MAX_BUBBLE_TEXT_LEN} 字符）")]
    TooLong,
}

/// 单条气泡（跨进程 DTO：serde 为前端 types.ts 的契约单一来源）
#[derive(Debug, Clone, Serialize)]
pub struct BubbleItem {
    /// 条目唯一标识（库内自增主键，创建序；展示 = sort_order 拖拽序，新捕获排头插入）
    pub id: i64,
    /// 气泡文本（捕获的剪贴板内容，入库前已 trim）
    pub text: String,
}

/// 气泡页快照 DTO：列表 + 满额提醒标记（提醒阈值裁决在 Rust 侧，前端零业务）
#[derive(Debug, Clone, Serialize)]
pub struct BubbleSnapshot {
    /// 气泡列表（sort_order 升序 = 拖拽序，PL013 起）
    pub items: Vec<BubbleItem>,
    /// 是否达到提醒阈值（count >= max_bubbles，PL014.2 配置化）
    pub remind: bool,
}

/// 满额提醒判定：条数达到上限即触发横幅（软提醒，不自动删——用户定案）。
/// max 来自设置（PL014.2 config.json 持久化，默认 5）
pub fn should_remind(count: usize, max: usize) -> bool {
    count >= max
}

/// 捕获文本校验：trim 后非空且不超过 MAX_BUBBLE_TEXT_LEN（按字符计）
pub fn validate_bubble_text(text: &str) -> Result<(), BubbleError> {
    if text.trim().is_empty() {
        return Err(BubbleError::EmptyText);
    }
    if text.chars().count() > MAX_BUBBLE_TEXT_LEN {
        return Err(BubbleError::TooLong);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remind_at_and_above_threshold() {
        assert!(!should_remind(4, 5));
        assert!(should_remind(5, 5));
        assert!(should_remind(6, 5));
        // PL014.2 阈值参数化：随设置变化
        assert!(!should_remind(6, 10));
        assert!(should_remind(2, 2));
    }

    #[test]
    fn blank_bubble_text_rejected() {
        assert!(matches!(
            validate_bubble_text(""),
            Err(BubbleError::EmptyText)
        ));
        assert!(matches!(
            validate_bubble_text("   "),
            Err(BubbleError::EmptyText)
        ));
    }

    #[test]
    fn normal_bubble_text_accepted() {
        assert!(validate_bubble_text("  复制的一段内容  ").is_ok());
    }

    #[test]
    fn overlong_bubble_text_rejected_and_limit_accepted() {
        // A002-P2-2：气泡无长度上限缺陷的锁定断言（超长剪贴板全量入库 → db 无界膨胀）
        assert!(matches!(
            validate_bubble_text(&"长".repeat(MAX_BUBBLE_TEXT_LEN + 1)),
            Err(BubbleError::TooLong)
        ));
        assert!(validate_bubble_text(&"字".repeat(MAX_BUBBLE_TEXT_LEN)).is_ok());
    }
}
