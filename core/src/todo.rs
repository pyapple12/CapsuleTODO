//! Todo 清单业务纯逻辑：DTO、文本/笔记校验、龄期裁决（禁 import tauri，业务纯逻辑约束）。
//! PL002 起"单一事实源 = db"：存取与排序由 storage.rs（SQL）承载，本模块只剩契约与校验。
//! PL010 扩容：created_at/done_at/note 三字段 + AgeLevel 龄期提醒裁决（时间源注入可测）。
//! PL027 扩容：详情块模型（NoteBlockKind / NoteBlock / validate_note_blocks）——`todos.note`
//! 单列纯文本退役，改"一段一行一块 + 图块混排"的有序块序列（不变量校验单点在此）。

use serde::{Deserialize, Serialize};
use thiserror::Error;
/// 单条待办文本长度上限（按字符数；前端 UI 约束 12 字的 2 倍兼容余量——
/// 用户定案放宽 1 倍，后端兜底防线，可调）
pub const MAX_TEXT_LEN: usize = 24;

/// 笔记长度上限（详情板自由文本；10K 足够防失控，可调）
pub const MAX_NOTE_LEN: usize = 10_000;

/// 详情块数上限（PL027：防失控，可调）
pub const MAX_NOTE_BLOCKS: usize = 100;

/// 龄期提醒黄字阈值：24 小时（毫秒；与实验场 V0.010 定案同源）
pub const AGE_YELLOW_MS: i64 = 24 * 3600 * 1000;
/// 龄期提醒红字阈值：48 小时（毫秒）
pub const AGE_RED_MS: i64 = 48 * 3600 * 1000;

/// 归档保留期（PATCH001，用户定案固定 7 天、不做设置项）：已完成条目 done_at
/// 距 now 超过该时长即由归档读路径回收；恰好等于阈值不回收（沿 age_level
/// "等于阈值不升级"先例）
pub const ARCHIVE_RETENTION_MS: i64 = 7 * 24 * 60 * 60 * 1000;

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
    /// 详情块集为空（不变量：至少保留一个文本块）
    #[error("详情块为空（至少保留一个文本块）")]
    EmptyBlocks,
    /// 文本块内含换行（一段一行：换行 = 新块）
    #[error("文本块不允许含换行")]
    NewlineInTextBlock,
    /// 详情块数超出 MAX_NOTE_BLOCKS 上限
    #[error("详情块数超限（上限 {MAX_NOTE_BLOCKS} 块）")]
    TooManyBlocks,
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
    /// 详情有无裁决（PL027 改口径：**任一块非空**——存在图块 或 存在 trim 非空文本块；
    /// 语义单源在存储层 SQL 的 EXISTS 子查询，红点指示依据，前端禁自行判空）
    pub has_note: bool,
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

/// 详情块类型（PL027 跨进程 DTO 契约 = 前端 `types.ts` 镜像）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteBlockKind {
    /// 文本块（一段一行：内容不含换行）
    Text,
    /// 图片块（`image_file` 指向落盘相对路径）
    Image,
}

/// 详情块（note = 有序块序列：一段一行一块，图块可插在文字之间）
#[derive(Debug, Clone, Serialize)]
pub struct NoteBlock {
    /// 块唯一标识（库内自增主键）
    pub id: i64,
    /// 块类型
    pub kind: NoteBlockKind,
    /// 文本内容（图块恒空串）
    pub text: String,
    /// 图片落盘相对路径（含子目录；图块专用，文本块恒 None）
    pub image_file: Option<String>,
}

/// 详情块集校验（PL027 不变量四条）：① 块数 ≤ MAX_NOTE_BLOCKS ② 至少一个文本块
/// ③ 文本块不含换行（`\n` / `\r`——一段一行，换行即新块）④ 文本块字符总和 ≤ MAX_NOTE_LEN。
/// 违者严格报错（保存路径唯一把关点）。
/// **PATCH002.8 修订**：空块（前导 / 连续）**不再是校验项**——空块规则是"显示 / 保存点"
/// 规则（前端在保存点归一化），DB 允许编辑期的中间态（否则防抖自动保存会被拒）
pub fn validate_note_blocks(blocks: &[NoteBlock]) -> Result<(), TodoError> {
    if blocks.len() > MAX_NOTE_BLOCKS {
        return Err(TodoError::TooManyBlocks);
    }
    if !blocks.iter().any(|b| b.kind == NoteBlockKind::Text) {
        return Err(TodoError::EmptyBlocks);
    }
    let mut total = 0usize;
    for block in blocks {
        if block.kind == NoteBlockKind::Text {
            if block.text.contains('\n') || block.text.contains('\r') {
                return Err(TodoError::NewlineInTextBlock);
            }
            total += block.text.chars().count();
        }
    }
    if total > MAX_NOTE_LEN {
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

    /// has_note 序列化契约断言（PL023）：新字段随 TodoItem serde 下发，键名漂移
    /// 防线（A004/A006 契约断言先例——前端 types.ts 镜像的单一事实源）
    #[test]
    fn todo_item_serializes_has_note() {
        let item = TodoItem {
            id: 1,
            text: "任务".into(),
            done: false,
            created_at: None,
            done_at: None,
            has_note: false,
        };
        let json = serde_json::to_string(&item).expect("序列化必须成功");
        assert!(json.contains("\"has_note\":false"));
    }

    // —— PL027.4 详情块校验（TDD：先红后绿） ——

    /// 造块辅助（只做形态，id 无意义）
    fn block(kind: NoteBlockKind, text: &str) -> NoteBlock {
        NoteBlock {
            id: 0,
            kind,
            text: text.to_string(),
            image_file: None,
        }
    }

    #[test]
    fn note_blocks_require_at_least_one_text_block() {
        assert!(matches!(
            validate_note_blocks(&[]),
            Err(TodoError::EmptyBlocks)
        ));
        assert!(matches!(
            validate_note_blocks(&[block(NoteBlockKind::Image, "")]),
            Err(TodoError::EmptyBlocks)
        ));
        assert!(validate_note_blocks(&[block(NoteBlockKind::Text, "")]).is_ok());
    }

    #[test]
    fn text_block_rejects_newline() {
        assert!(matches!(
            validate_note_blocks(&[block(NoteBlockKind::Text, "a\nb")]),
            Err(TodoError::NewlineInTextBlock)
        ));
        assert!(matches!(
            validate_note_blocks(&[block(NoteBlockKind::Text, "a\rb")]),
            Err(TodoError::NewlineInTextBlock)
        ));
    }

    #[test]
    fn note_blocks_enforce_total_length() {
        let ok = "字".repeat(MAX_NOTE_LEN);
        assert!(validate_note_blocks(&[block(NoteBlockKind::Text, &ok)]).is_ok());
        let too = "字".repeat(MAX_NOTE_LEN + 1);
        assert!(matches!(
            validate_note_blocks(&[block(NoteBlockKind::Text, &too)]),
            Err(TodoError::NoteTooLong)
        ));
    }

    #[test]
    fn note_blocks_enforce_block_count() {
        // 恰好 100 块过；101 块拒（边界两侧都断言；首块为有内容文本块——空块规则见 PATCH002）
        let mut blocks = vec![block(NoteBlockKind::Text, "内容")];
        for _ in 1..MAX_NOTE_BLOCKS {
            blocks.push(block(NoteBlockKind::Image, ""));
        }
        assert_eq!(blocks.len(), MAX_NOTE_BLOCKS);
        assert!(validate_note_blocks(&blocks).is_ok(), "恰好上限必须过");
        blocks.push(block(NoteBlockKind::Image, ""));
        assert!(matches!(
            validate_note_blocks(&blocks),
            Err(TodoError::TooManyBlocks)
        ));
    }

    /// NoteBlockKind 序列化形态断言（serde 契约 = 前端 types.ts 镜像）
    #[test]
    fn note_block_kind_serializes_as_plain_string() {
        assert_eq!(
            serde_json::to_string(&NoteBlockKind::Text).expect("序列化必须成功"),
            "\"Text\""
        );
        assert_eq!(
            serde_json::to_string(&NoteBlockKind::Image).expect("序列化必须成功"),
            "\"Image\""
        );
    }

    // —— PATCH002.2/.8 空块规则：DB 侧不校验（显示/保存点规则在前端） ——

    #[test]
    fn note_blocks_accept_blank_arrangements() {
        // PATCH002.8：空块不再是校验项——前导/连续空块**可入库**（编辑期中间态），
        // 归一化只发生在前端"保存点"（Ctrl+Enter / 焦点离开 / 关板）
        let blank = || block(NoteBlockKind::Text, "");
        let content = || block(NoteBlockKind::Text, "内容");
        let image = || NoteBlock {
            id: 0,
            kind: NoteBlockKind::Image,
            text: String::new(),
            image_file: Some("note_x/1.png".to_string()),
        };
        assert!(
            validate_note_blocks(&[blank(), content()]).is_ok(),
            "前导空块可入库"
        );
        assert!(
            validate_note_blocks(&[content(), blank(), blank()]).is_ok(),
            "连续空块可入库"
        );
        assert!(
            validate_note_blocks(&[content(), blank(), content()]).is_ok(),
            "内容｜空｜内容 过"
        );
        assert!(
            validate_note_blocks(&[content(), blank()]).is_ok(),
            "内容｜空 过"
        );
        assert!(
            validate_note_blocks(&[blank()]).is_ok(),
            "整条全空保留一个空块过"
        );
        assert!(
            validate_note_blocks(&[image(), blank()]).is_ok(),
            "图后空块过"
        );
        // 仍必须拒的：无文本块 / 换行 / 超长 / 块数超限
        assert!(matches!(
            validate_note_blocks(&[image()]),
            Err(TodoError::EmptyBlocks)
        ));
        assert!(matches!(
            validate_note_blocks(&[block(NoteBlockKind::Text, "a\nb")]),
            Err(TodoError::NewlineInTextBlock)
        ));
    }
}
