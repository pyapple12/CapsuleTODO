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

/// todo_add 核心实现：业务校验 + trim 落库（与 rename/bubble 同规——首尾空白不入库）
pub fn todo_add_core(text: &str, ctx: &AppContext) -> Result<TodoItem, CommandError> {
    validate_text(text)?;
    let storage = ctx.lock_storage()?;
    Ok(storage.add(text.trim())?)
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

/// 清单重排（拖拽落点提交）：ids = 未完成条目的目标顺序（全量集，一致性校验在 storage 层）
#[tauri::command]
pub fn todo_reorder(ids: Vec<i64>, ctx: State<'_, AppContext>) -> Result<(), CommandError> {
    todo_reorder_core(&ids, &ctx)
}

/// todo_reorder 核心实现：透传 storage.reorder_todos（校验+事务在存储层）
pub fn todo_reorder_core(ids: &[i64], ctx: &AppContext) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.reorder_todos(ids)?)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

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
    fn add_trims_surrounding_whitespace() {
        // FIX003.10：add 与 rename 同规 trim 落库
        let ctx = test_context();
        let item = todo_add_core("  任务一  ", &ctx).expect("合法文本必须成功");
        assert_eq!(item.text, "任务一", "add 走 trim 存储语义");
    }

    #[test]
    fn list_attaches_age_level_from_storage_clock() {
        // 可变时钟真断言（FIX003.10）：建行于 T0 → 拨至 +25h 断言 Yellow → 拨至
        // +49h 断言 Red（命令层龄期裁决全链，替代旧恒 None 弱断言）
        let t0 = 1_700_000_000_000i64;
        let clock = Arc::new(std::sync::atomic::AtomicI64::new(t0));
        let ctx = AppContext {
            storage: Mutex::new(
                Storage::open_in_memory_with_now(Arc::new({
                    let clock = Arc::clone(&clock);
                    move || clock.load(std::sync::atomic::Ordering::SeqCst)
                }))
                .expect("内存库必须可开"),
            ),
            settings: Mutex::new(crate::settings::WindowSettings::default()),
        };
        let item = todo_add_core("老任务", &ctx).expect("合法文本必须成功");
        let hour = 3_600_000i64;
        clock.store(t0 + 25 * hour, std::sync::atomic::Ordering::SeqCst);
        let view = todo_list_core(&ctx).expect("读命令必须成功");
        assert_eq!(view[0].age_level, AgeLevel::Yellow, "超 24h 升 Yellow");
        clock.store(t0 + 49 * hour, std::sync::atomic::Ordering::SeqCst);
        let view = todo_list_core(&ctx).expect("读命令必须成功");
        assert_eq!(view[0].age_level, AgeLevel::Red, "超 48h 升 Red");
        let _ = item;
    }

    // —— PL013.2 重排命令 ——

    #[test]
    fn reorder_core_roundtrip_and_mismatch() {
        let ctx = test_context();
        let a = todo_add_core("甲", &ctx).expect("合法文本必须成功");
        let b = todo_add_core("乙", &ctx).expect("合法文本必须成功");
        let c = todo_add_core("丙", &ctx).expect("合法文本必须成功");
        todo_reorder_core(&[c.id, b.id, a.id], &ctx).expect("全量重排必须成功");
        let ids: Vec<i64> = todo_list_core(&ctx)
            .expect("读命令必须成功")
            .into_iter()
            .map(|v| v.item.id)
            .collect();
        assert_eq!(ids, vec![c.id, b.id, a.id]);
        // 部分集 / 幽灵 id：错误跨命令层可见
        assert!(todo_reorder_core(&[a.id], &ctx).is_err());
        assert!(todo_reorder_core(&[a.id, b.id, 99], &ctx).is_err());
    }
}
