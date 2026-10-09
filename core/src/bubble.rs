//! 气泡纯逻辑：DTO 与捕获文本校验（禁 import tauri，业务纯逻辑约束）。
//! 单一事实源 = db：气泡存取由 storage.rs 承载，本模块只剩契约与校验；
//! 满额提醒显隐由前端本地阈值裁决（FIX004.23：Rust 侧 should_remind/remind 死值已删）。

use serde::Serialize;
use thiserror::Error;

/// 气泡文本长度上限（按字符数；A002-P2-2 定案——气泡 = 短片段语义，防超长剪贴板无界入库，可调）
pub const MAX_BUBBLE_TEXT_LEN: usize = 2_000;

/// 单图字节上限（PNG 编码后落库字节；4MB 足够全屏截图，可调，PL024）
pub const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;

/// 气泡业务错误：捕获文本非法
#[derive(Debug, Error)]
pub enum BubbleError {
    /// 文本 trim 后为空（空剪贴板已在上游拦截，此为兜底校验）
    #[error("气泡内容为空")]
    EmptyText,
    /// 文本超出 MAX_BUBBLE_TEXT_LEN 上限
    #[error("气泡内容过长（上限 {MAX_BUBBLE_TEXT_LEN} 字符）")]
    TooLong,
    /// 图片超出 MAX_IMAGE_BYTES 上限（PL024）
    #[error("图片过大（上限 {MAX_IMAGE_BYTES} 字节）")]
    ImageTooLong,
    /// 图片格式暂不支持（PNG 魔数不符，或 DIB 罕见变体——PL024 严格抛错主线）
    #[error("图片格式暂不支持")]
    UnsupportedFormat,
}

/// 气泡类型（serde plain string 沿 AgeLevel 先例："Text" | "Image"，前端 types.ts 镜像；
/// 值域由自有写入收敛，行映射宽容回落 Text——PL024）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BubbleKind {
    /// 文本气泡（一期全部存量行）
    Text,
    /// 图片气泡（剪贴板图片捕获，PL024）
    Image,
}

/// 单条气泡（跨进程 DTO：serde 为前端 types.ts 的契约单一来源）
#[derive(Debug, Clone, Serialize)]
pub struct BubbleItem {
    /// 条目唯一标识（库内自增主键，创建序；展示 = sort_order 拖拽序，新捕获排头插入）
    pub id: i64,
    /// 气泡文本（捕获的剪贴板内容，入库前已 trim；图片气泡 = 自动占位文案）
    pub text: String,
    /// 气泡类型（列表载荷只带标记不带图——图片字节走 bubble_get_image 按需拉取，PL024）
    pub kind: BubbleKind,
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

/// PNG 文件签名（RFC 2083 固定 8 字节魔数；clipboard_image 文件分支复用，PL024.8b）
pub(crate) const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// 图片校验：PNG 魔数 + 字节上限（入库前唯一闸口，PL024；魔数在先——垃圾字节
/// 直接报格式，不浪费上限比较）
pub fn validate_image_png(bytes: &[u8]) -> Result<(), BubbleError> {
    if !bytes.starts_with(&PNG_MAGIC) {
        return Err(BubbleError::UnsupportedFormat);
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(BubbleError::ImageTooLong);
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

    // —— PL024.1 图片校验与类型契约（TDD 红灯）——

    #[test]
    fn image_validation_magic_and_limit() {
        // 合法头 + 少量载荷 = 接受；魔数不符 = UnsupportedFormat；超上限 = ImageTooLong
        let mut png = PNG_MAGIC.to_vec();
        png.extend_from_slice(&[0u8; 32]);
        assert!(validate_image_png(&png).is_ok());
        assert!(matches!(
            validate_image_png(b"not a png at all"),
            Err(BubbleError::UnsupportedFormat)
        ));
        let mut big = PNG_MAGIC.to_vec();
        big.resize(MAX_IMAGE_BYTES + 1, 0);
        assert!(matches!(
            validate_image_png(&big),
            Err(BubbleError::ImageTooLong)
        ));
        assert!(matches!(
            validate_image_png(&[]),
            Err(BubbleError::UnsupportedFormat)
        ));
    }

    #[test]
    fn bubble_kind_serializes_as_plain_string() {
        // serde 契约 = 前端 types.ts 镜像（AgeLevel 同族：variant 名原样）
        assert_eq!(
            serde_json::to_string(&BubbleKind::Text).unwrap(),
            "\"Text\""
        );
        assert_eq!(
            serde_json::to_string(&BubbleKind::Image).unwrap(),
            "\"Image\""
        );
    }

    #[test]
    fn bubble_item_serializes_kind_field() {
        let item = BubbleItem {
            id: 1,
            text: "占位".into(),
            kind: BubbleKind::Text,
        };
        let json = serde_json::to_string(&item).expect("序列化必须成功");
        assert!(json.contains("\"kind\":\"Text\""), "kind 键名漂移防线");
    }
}
