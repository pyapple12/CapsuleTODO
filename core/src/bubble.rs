//! 气泡纯逻辑：DTO 与捕获文本校验（禁 import tauri，业务纯逻辑约束）。
//! 单一事实源 = db：气泡存取由 storage.rs 承载，本模块只剩契约与校验；
//! 满额提醒显隐由前端本地阈值裁决（FIX004.23：Rust 侧 should_remind/remind 死值已删）。

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

/// 气泡页快照 DTO：列表（满额提醒显隐由前端本地阈值裁决，FIX004.23 删 Rust 侧死值）
#[derive(Debug, Clone, Serialize)]
pub struct BubbleSnapshot {
    /// 气泡列表（sort_order 升序 = 拖拽序，PL013 起）
    pub items: Vec<BubbleItem>,
}

/// 捕获文本校验：trim 后非空且长度（trim 后按字符计，FIX008.8 对齐 todo
/// validate_text 口径——入库形态即 trim 后文本，按原文计数会把"原文贴上限
/// 且带首尾空白"的合规内容误拒）不超过 MAX_BUBBLE_TEXT_LEN
pub fn validate_bubble_text(text: &str) -> Result<(), BubbleError> {
    if text.trim().is_empty() {
        return Err(BubbleError::EmptyText);
    }
    if text.trim().chars().count() > MAX_BUBBLE_TEXT_LEN {
        return Err(BubbleError::TooLong);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn length_counted_after_trim() {
        // FIX008.8 口径锁定：计数对 trim 后文本——原文贴上限带空白不误拒，
        // trim 后真超限仍拒（与入库形态一致，文档"trim 后"措辞即此语义）
        assert!(validate_bubble_text(&format!(" {} ", "字".repeat(MAX_BUBBLE_TEXT_LEN))).is_ok());
        assert!(matches!(
            validate_bubble_text(&format!(" {} ", "长".repeat(MAX_BUBBLE_TEXT_LEN + 1))),
            Err(BubbleError::TooLong)
        ));
    }
}
