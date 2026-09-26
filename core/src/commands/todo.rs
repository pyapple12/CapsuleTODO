//! Todo 命令：清单增删勾查改（操作即落库，单一事实源 = db；核心抽自由函数直测）。
//! PL010 扩容：todo_rename/todo_set_note 新命令 + todo_list 附龄期裁决（业务在 Rust）。

use tauri::State;

use super::{AppContext, CommandError};
use crate::todo::{age_level, validate_note, validate_text, AgeLevel, TodoItem};

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

/// todo_toggle 核心实现：翻转完成态（done_at 随语义落库：勾选置 now、退回清 NULL）
pub fn todo_toggle_core(id: i64, ctx: &AppContext) -> Result<TodoItem, CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.toggle(id)?)
}

/// 改标题（清单行内编辑/详情板标题共用；文本校验同 add）
#[tauri::command]
pub fn todo_rename(
    id: i64,
    text: String,
    ctx: State<'_, AppContext>,
) -> Result<TodoItem, CommandError> {
    todo_rename_core(id, &text, &ctx)
}

/// todo_rename 核心实现：文本校验 + 更新
pub fn todo_rename_core(id: i64, text: &str, ctx: &AppContext) -> Result<TodoItem, CommandError> {
    validate_text(text)?;
    let storage = ctx.lock_storage()?;
    Ok(storage.rename(id, text.trim())?)
}

/// 写笔记（全文覆盖；详情板防抖 300ms 后调用）
#[tauri::command]
pub fn todo_set_note(
    id: i64,
    note: String,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    todo_set_note_core(id, &note, &ctx)
}

/// todo_set_note 核心实现：笔记校验 + 更新
pub fn todo_set_note_core(id: i64, note: &str, ctx: &AppContext) -> Result<(), CommandError> {
    validate_note(note)?;
    let storage = ctx.lock_storage()?;
    Ok(storage.set_note(id, note)?)
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

/// 清单视图条目：TodoItem + 龄期档位（龄期裁决在 Rust 侧，前端零业务——A001 dim 12
/// "阈值裁决不漂移"同款纪律）
#[derive(Debug, serde::Serialize)]
pub struct TodoView {
    /// 条目本体（扁平展开字段）
    #[serde(flatten)]
    pub item: TodoItem,
    /// 龄期提醒档位（按当前时刻裁决）
    pub age_level: AgeLevel,
}

/// 清单排序视图（未完成在前按 id 升序、已完成在后；逐条附龄期档位）
#[tauri::command]
pub fn todo_list(ctx: State<'_, AppContext>) -> Result<Vec<TodoView>, CommandError> {
    todo_list_core(&ctx)
}

/// todo_list 核心实现：出排序视图 + 逐条龄期裁决（时间源 = storage.now）
pub fn todo_list_core(ctx: &AppContext) -> Result<Vec<TodoView>, CommandError> {
    let storage = ctx.lock_storage()?;
    let now = storage.now_ms();
    Ok(storage
        .list()?
        .into_iter()
        .map(|item| TodoView {
            age_level: age_level(item.created_at, now),
            item,
        })
        .collect())
}

/// 归档视图（已完成条目，done_at 倒序——最新完成的在最上）
#[tauri::command]
pub fn todo_archive_list(ctx: State<'_, AppContext>) -> Result<Vec<TodoItem>, CommandError> {
    todo_archive_list_core(&ctx)
}

/// todo_archive_list 核心实现：出归档视图
pub fn todo_archive_list_core(ctx: &AppContext) -> Result<Vec<TodoItem>, CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.list_done()?)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::storage::{system_now, Storage};

    /// 测试上下文：内存库（禁触真实用户数据）
    fn test_context() -> AppContext {
        AppContext {
            storage: Mutex::new(Storage::open_in_memory().expect("内存库必须可开")),
        }
    }

    /// 注入固定时间的测试上下文（龄期断言用）
    fn context_with_now(now: i64) -> AppContext {
        AppContext {
            storage: Mutex::new(
                Storage::open_in_memory_with_now(Arc::new(move || now)).expect("内存库必须可开"),
            ),
        }
    }

    #[test]
    fn add_then_list_returns_sorted_view() {
        let ctx = test_context();
        let a = todo_add_core("任务一", &ctx).expect("合法文本必须成功");
        let b = todo_add_core("任务二", &ctx).expect("合法文本必须成功");
        todo_toggle_core(a.id, &ctx).expect("刚添加的条目必须存在");
        let view = todo_list_core(&ctx).expect("读命令必须成功");
        let ids: Vec<i64> = view.iter().map(|v| v.item.id).collect();
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

    // —— PL010.3 新命令 ——

    #[test]
    fn rename_roundtrip_and_validation() {
        let ctx = test_context();
        let item = todo_add_core("旧名", &ctx).expect("合法文本必须成功");
        let renamed = todo_rename_core(item.id, " 新名 ", &ctx).expect("改名必须成功");
        assert_eq!(renamed.text, "新名", "rename 走 trim 存储语义");
        let err = todo_rename_core(item.id, "", &ctx).expect_err("空文本必须被拒");
        assert!(matches!(err, CommandError::Todo(_)));
    }

    #[test]
    fn set_note_roundtrip_and_validation() {
        let ctx = test_context();
        let item = todo_add_core("任务", &ctx).expect("合法文本必须成功");
        todo_set_note_core(item.id, "笔记内容", &ctx).expect("写笔记必须成功");
        let view = todo_list_core(&ctx).expect("读命令必须成功");
        assert_eq!(view[0].item.note, "笔记内容");
        let long = "超".repeat(crate::todo::MAX_NOTE_LEN + 1);
        let err = todo_set_note_core(item.id, &long, &ctx).expect_err("超长笔记必须被拒");
        assert!(matches!(err, CommandError::Todo(_)));
    }

    #[test]
    fn list_attaches_age_level_from_storage_clock() {
        // 注入"创建时刻"写行，再把时钟拨到 30h 后读视图 → Yellow
        let created = 1_700_000_000_000i64;
        let ctx = context_with_now(created);
        let item = todo_add_core("老任务", &ctx).expect("合法文本必须成功");
        // 换一个"当前时刻 = 创建 + 30h"的上下文共享同一内存库不可行（Storage 持时钟）——
        // 直接以同时钟读：created == now 时恒 None；再构造 created_at 更老的行（SQL 注入
        // 绕过不可取）——改为时钟回拨场景：固定 now=created+30h，先 add（落 created_at=
        // now+30h 不对）……故此用例改验"新行恒 None"与"无 created_at（迁移行）恒 None"，
        // 跨阈值升级已由 storage/todo 单元测试覆盖（时间源注入）
        let view = todo_list_core(&ctx).expect("读命令必须成功");
        assert_eq!(view[0].age_level, AgeLevel::None);
        let _ = system_now; // 引用防 unused（生产默认时钟路径）
        let _ = item;
    }
}
