//! 气泡命令：捕获剪贴板/列表/复制回/删除/清空（操作即落库，单一事实源 = db）。
//! clipboard 读写留命令薄壳（不可注入直测），文本链路核心抽自由函数直测内存库。

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use super::{AppContext, CommandError};
use crate::bubble::{should_remind, validate_bubble_text, BubbleItem, BubbleSnapshot};
use crate::storage::BubbleAddOutcome;

/// 热键触发路径（PL015.5）：主线程执行——读剪贴板 → 校验 → 去重 → 入库，
/// 全程失败落日志静默（反馈静默定案：桌面常驻面板即所见）
pub fn bubble_capture_from_clipboard_quiet(app: &AppHandle) {
    let text = match app.clipboard().read_text() {
        Ok(text) => text,
        Err(err) => {
            eprintln!("热键捕获：剪贴板读取失败（{err}）");
            return;
        }
    };
    let ctx = app.state::<AppContext>();
    match bubble_capture_core(&text, &ctx) {
        // added：新气泡已入排头，桌面常驻面板自会呈现；duplicate：静默
        Ok(_) => {}
        Err(err) => eprintln!("热键捕获失败：{err}"),
    }
}

/// 捕获结果（跨 IPC，PL015.5 去重）：added 携带新条目；duplicate 供前端捕获钮切换
/// "重复捕获，无效！"占字态（热键路径静默）
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BubbleCaptureOutcome {
    Added { item: BubbleItem },
    Duplicate,
}

impl BubbleCaptureOutcome {
    /// 测试辅助：取新增条目（Duplicate panic——调用方应先分支）
    pub fn added_item(self) -> BubbleItem {
        match self {
            BubbleCaptureOutcome::Added { item } => item,
            BubbleCaptureOutcome::Duplicate => panic!("气泡捕获意外重复"),
        }
    }
}

/// 捕获剪贴板文本为新气泡（剪贴板无文本/空文本严格报错；重复内容返回 Duplicate
/// 不入库——PL015.5 去重，与热键路径同规）
#[tauri::command]
pub fn bubble_capture(
    app: AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<BubbleCaptureOutcome, CommandError> {
    let text = app
        .clipboard()
        .read_text()
        .map_err(|err| CommandError::Clipboard(format!("剪贴板读取失败：{err}")))?;
    bubble_capture_core(&text, &ctx)
}

/// bubble_capture 核心实现：校验 + 去重裁决 + 入库
pub fn bubble_capture_core(
    text: &str,
    ctx: &AppContext,
) -> Result<BubbleCaptureOutcome, CommandError> {
    validate_bubble_text(text)?;
    let storage = ctx.lock_storage()?;
    match storage.add_bubble(text.trim())? {
        BubbleAddOutcome::Added(item) => Ok(BubbleCaptureOutcome::Added { item }),
        BubbleAddOutcome::Duplicate => Ok(BubbleCaptureOutcome::Duplicate),
    }
}

/// 气泡列表（sort_order 升序拖拽序，新捕获排头插入）
#[tauri::command]
pub fn bubble_list(ctx: State<'_, AppContext>) -> Result<BubbleSnapshot, CommandError> {
    bubble_list_core(&ctx)
}

/// bubble_list 核心实现：出快照（升序拖拽序 + 满额提醒标记——阈值裁决在 Rust 侧，
/// 上限来自设置 PL014.2，前端零业务）
pub fn bubble_list_core(ctx: &AppContext) -> Result<BubbleSnapshot, CommandError> {
    let storage = ctx.lock_storage()?;
    let max = ctx.lock_settings()?.max_bubbles as usize;
    let items = storage.list_bubbles()?;
    let remind = should_remind(items.len(), max);
    Ok(BubbleSnapshot { items, remind })
}

/// 复制气泡内容回剪贴板（捕获→粘贴走→清理闭环的回程）
#[tauri::command]
pub fn bubble_copy(
    id: i64,
    app: AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    let text = bubble_text_core(id, &ctx)?;
    app.clipboard()
        .write_text(&text)
        .map_err(|err| CommandError::Clipboard(format!("写入剪贴板失败：{err}")))?;
    Ok(())
}

/// 复制回的前半段：回读气泡文本（直测）
pub fn bubble_text_core(id: i64, ctx: &AppContext) -> Result<String, CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.get_bubble(id)?.text)
}

/// 删除单条气泡（不存在严格报错）
#[tauri::command]
pub fn bubble_remove(id: i64, ctx: State<'_, AppContext>) -> Result<(), CommandError> {
    bubble_remove_core(id, &ctx)
}

/// bubble_remove 核心实现：删除条目
pub fn bubble_remove_core(id: i64, ctx: &AppContext) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.remove_bubble(id)?)
}

/// 一键清空气泡，返回清除条数（满 5 提醒后的清理出口）
#[tauri::command]
pub fn bubble_clear(ctx: State<'_, AppContext>) -> Result<usize, CommandError> {
    bubble_clear_core(&ctx)
}

/// bubble_clear 核心实现：清空
pub fn bubble_clear_core(ctx: &AppContext) -> Result<usize, CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.clear_bubbles()?)
}

/// 气泡重排（拖拽落点提交）：ids = 全量气泡的目标顺序（一致性校验在 storage 层）
#[tauri::command]
pub fn bubble_reorder(ids: Vec<i64>, ctx: State<'_, AppContext>) -> Result<(), CommandError> {
    bubble_reorder_core(&ids, &ctx)
}

/// bubble_reorder 核心实现：透传 storage.reorder_bubbles（校验+事务在存储层）
pub fn bubble_reorder_core(ids: &[i64], ctx: &AppContext) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.reorder_bubbles(ids)?)
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
            settings: Mutex::new(crate::settings::WindowSettings::default()),
        }
    }

    #[test]
    fn capture_trims_and_persists() {
        let ctx = test_context();
        let item = bubble_capture_core("  复制的内容  ", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        let snapshot = bubble_list_core(&ctx).expect("读命令必须成功");
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].id, item.id);
        assert_eq!(snapshot.items[0].text, "复制的内容");
        assert!(!snapshot.remind);
    }

    #[test]
    fn capture_duplicate_outcome_and_list_unchanged() {
        // PL015.5 去重：重复文本返回 Duplicate（非错误），列表不变；trim 后命中查重
        let ctx = test_context();
        bubble_capture_core("重复片段", &ctx)
            .expect("首次必须成功")
            .added_item();
        let second = bubble_capture_core("  重复片段  ", &ctx).expect("重复不是错误");
        assert!(matches!(second, BubbleCaptureOutcome::Duplicate));
        let snapshot = bubble_list_core(&ctx).expect("读命令必须成功");
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].text, "重复片段");
    }

    #[test]
    fn capture_blank_is_visible_error() {
        let ctx = test_context();
        let err = bubble_capture_core("   ", &ctx).expect_err("空文本必须被拒");
        assert!(matches!(err, CommandError::Clipboard(_)));
    }

    #[test]
    fn text_core_reads_back_and_missing_errors() {
        let ctx = test_context();
        let item = bubble_capture_core("片段", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        assert_eq!(
            bubble_text_core(item.id, &ctx).expect("回读必须成功"),
            "片段"
        );
        let err = bubble_text_core(99, &ctx).expect_err("不存在必须报错");
        assert!(matches!(err, CommandError::Storage(_)));
    }

    #[test]
    fn remove_and_clear_shrink_list() {
        let ctx = test_context();
        let a = bubble_capture_core("一", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        bubble_capture_core("二", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        bubble_remove_core(a.id, &ctx).expect("刚添加的条目必须存在");
        assert_eq!(
            bubble_list_core(&ctx).expect("读命令必须成功").items.len(),
            1
        );
        let cleared = bubble_clear_core(&ctx).expect("清空必须成功");
        assert_eq!(cleared, 1);
        let snapshot = bubble_list_core(&ctx).expect("读命令必须成功");
        assert!(snapshot.items.is_empty());
        assert!(!snapshot.remind);
    }

    // —— PL013.2 重排命令 ——

    #[test]
    fn reorder_core_roundtrip_and_mismatch() {
        let ctx = test_context();
        let a = bubble_capture_core("一", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        let b = bubble_capture_core("二", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        let c = bubble_capture_core("三", &ctx)
            .expect("合法文本必须成功")
            .added_item();
        bubble_reorder_core(&[c.id, a.id, b.id], &ctx).expect("全量重排必须成功");
        let ids: Vec<i64> = bubble_list_core(&ctx)
            .expect("读命令必须成功")
            .items
            .into_iter()
            .map(|it| it.id)
            .collect();
        assert_eq!(ids, vec![c.id, a.id, b.id]);
        // 部分集 / 幽灵 id：错误跨命令层可见
        assert!(bubble_reorder_core(&[a.id], &ctx).is_err());
        assert!(bubble_reorder_core(&[a.id, 99], &ctx).is_err());
    }
}
