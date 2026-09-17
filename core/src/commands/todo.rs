//! Todo 命令：清单增删勾查（操作即落库，单一事实源 = db；核心抽自由函数直测）。

use tauri::State;

use super::{AppContext, CommandError};
use crate::todo::{validate_text, TodoItem};

/// 添加待办（文本校验 + 落库；拒绝经 CommandError 跨进程可见）
#[tauri::command]
pub fn todo_add(text: String, ctx: State<'_, AppContext>) -> Result<TodoItem, CommandError> {
    todo_add_core(&text, &ctx)
}

/// todo_add 核心实现：业务校验 + 写库
pub fn todo_add_core(text: &str, ctx: &AppContext) -> Result<TodoItem, CommandError> {
    validate_text(text)?;
    let storage = ctx.lock_storage()?;
    Ok(storage.add(text)?)
}

/// 勾选翻转（不存在经 NotFound 严格报错）
#[tauri::command]
pub fn todo_toggle(id: i64, ctx: State<'_, AppContext>) -> Result<TodoItem, CommandError> {
    todo_toggle_core(id, &ctx)
}

/// todo_toggle 核心实现：翻转完成态
pub fn todo_toggle_core(id: i64, ctx: &AppContext) -> Result<TodoItem, CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.toggle(id)?)
}

/// 删除待办（不存在严格报错）
#[tauri::command]
pub fn todo_remove(id: i64, ctx: State<'_, AppContext>) -> Result<(), CommandError> {
    todo_remove_core(id, &ctx)
}

/// todo_remove 核心实现：删除条目
pub fn todo_remove_core(id: i64, ctx: &AppContext) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.remove(id)?)
}

/// 清单排序视图（未完成在前按 id 升序、已完成在后）
#[tauri::command]
pub fn todo_list(ctx: State<'_, AppContext>) -> Result<Vec<TodoItem>, CommandError> {
    todo_list_core(&ctx)
}

/// todo_list 核心实现：出排序视图
pub fn todo_list_core(ctx: &AppContext) -> Result<Vec<TodoItem>, CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.list()?)
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
    fn add_then_list_returns_sorted_view() {
        let ctx = test_context();
        let a = todo_add_core("任务一", &ctx).expect("合法文本必须成功");
        let b = todo_add_core("任务二", &ctx).expect("合法文本必须成功");
        todo_toggle_core(a.id, &ctx).expect("刚添加的条目必须存在");
        let view = todo_list_core(&ctx).expect("读命令必须成功");
        let ids: Vec<i64> = view.iter().map(|it| it.id).collect();
        assert_eq!(ids, vec![b.id, a.id]);
    }

    #[test]
    fn add_blank_text_is_visible_error() {
        let ctx = test_context();
        let err = todo_add_core("   ", &ctx).expect_err("空文本必须被拒");
        assert!(matches!(err, CommandError::Todo(_)));
    }

    #[test]
    fn toggle_missing_id_is_visible_error() {
        let ctx = test_context();
        let err = todo_toggle_core(99, &ctx).expect_err("不存在条目必须报错");
        assert!(matches!(err, CommandError::Storage(_)));
    }

    #[test]
    fn remove_then_list_shrinks() {
        let ctx = test_context();
        let item = todo_add_core("任务一", &ctx).expect("合法文本必须成功");
        todo_remove_core(item.id, &ctx).expect("刚添加的条目必须存在");
        assert!(todo_list_core(&ctx).expect("读命令必须成功").is_empty());
    }
}
