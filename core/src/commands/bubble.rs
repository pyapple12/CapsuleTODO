//! 气泡命令：捕获剪贴板/列表/复制回/删除/清空（操作即落库，单一事实源 = db）。
//! clipboard 读写留命令薄壳（不可注入直测），文本链路核心抽自由函数直测内存库。

use tauri::{AppHandle, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use super::{AppContext, CommandError};
use crate::bubble::{should_remind, validate_bubble_text, BubbleItem, BubbleSnapshot};

/// 捕获剪贴板文本为新气泡（剪贴板无文本/空文本严格报错）
#[tauri::command]
pub fn bubble_capture(
    app: AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<BubbleItem, CommandError> {
    let text = app
        .clipboard()
        .read_text()
        .map_err(|err| CommandError::Clipboard(format!("剪贴板读取失败：{err}")))?;
    bubble_capture_core(&text, &ctx)
}

/// bubble_capture 核心实现：校验 + 入库
pub fn bubble_capture_core(text: &str, ctx: &AppContext) -> Result<BubbleItem, CommandError> {
    validate_bubble_text(text)?;
    let storage = ctx.lock_storage()?;
    Ok(storage.add_bubble(text.trim())?)
}

/// 气泡列表（新在前）
#[tauri::command]
pub fn bubble_list(ctx: State<'_, AppContext>) -> Result<BubbleSnapshot, CommandError> {
    bubble_list_core(&ctx)
}

/// bubble_list 核心实现：出快照（倒序列表 + 满 5 提醒标记——阈值裁决在 Rust 侧，前端零业务）
pub fn bubble_list_core(ctx: &AppContext) -> Result<BubbleSnapshot, CommandError> {
    let storage = ctx.lock_storage()?;
    let items = storage.list_bubbles()?;
    let remind = should_remind(items.len());
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
    fn capture_trims_and_persists() {
        let ctx = test_context();
        let item = bubble_capture_core("  复制的内容  ", &ctx).expect("合法文本必须成功");
        let snapshot = bubble_list_core(&ctx).expect("读命令必须成功");
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].id, item.id);
        assert_eq!(snapshot.items[0].text, "复制的内容");
        assert!(!snapshot.remind);
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
        let item = bubble_capture_core("片段", &ctx).expect("合法文本必须成功");
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
        let a = bubble_capture_core("一", &ctx).expect("合法文本必须成功");
        bubble_capture_core("二", &ctx).expect("合法文本必须成功");
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
}
