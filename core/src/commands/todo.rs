//! Todo 命令：清单增删勾查改（操作即落库，单一事实源 = db；核心抽自由函数直测）。
//! PL010 扩容：todo_rename/todo_set_note 新命令 + todo_list 附龄期裁决（业务在 Rust）。

use tauri::{Emitter, State};

use super::{AppContext, CommandError};
use crate::todo::{
    age_level, validate_note_blocks, validate_text, AgeLevel, NoteBlock, NoteBlockKind, TodoError,
    TodoItem, ARCHIVE_RETENTION_MS, MAX_NOTE_BLOCKS,
};

/// 变更广播（PL018.6 问题一修复；FIX006.1 载荷 origin 方案）：清单数据任何变更后
/// 由命令层发 todo-changed，载荷 = 发起窗 label——**前端自判早退**（发起窗刷新走
/// 自身组件链时序，300ms 主拍不被截断；跨窗照常刷新）。
/// 为什么不用 emit_filter：tauri 2.x 的 match_any_or_filter 语义下，JS listen()
/// 缺省 target=Any **恒通过无视 filter**（listener.rs 实证）——FIX005.5 的过滤
/// 方案对 JS 监听者整个无效，故改载荷显式 origin（详见 AGENTS 环境陷阱节）
fn emit_todo_changed(window: &tauri::Window, app: &tauri::AppHandle) {
    if let Err(err) = app.emit("todo-changed", window.label().to_string()) {
        eprintln!("todo-changed 广播失败：{err}");
    }
}

/// 添加待办（文本校验 + 落库；拒绝经 CommandError 跨进程可见）→ 广播变更
#[tauri::command]
pub fn todo_add(
    window: tauri::Window,
    text: String,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<TodoItem, CommandError> {
    let item = todo_add_core(&text, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(item)
}

/// todo_add 核心实现：业务校验 + trim 落库（与 rename/bubble 同规——首尾空白不入库）
pub fn todo_add_core(text: &str, ctx: &AppContext) -> Result<TodoItem, CommandError> {
    validate_text(text)?;
    let storage = ctx.lock_storage()?;
    Ok(storage.add(text.trim())?)
}

/// 勾选翻转（不存在经 NotFound 严格报错）→ 广播变更
#[tauri::command]
pub fn todo_toggle(
    window: tauri::Window,
    id: i64,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<TodoItem, CommandError> {
    let item = todo_toggle_core(id, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(item)
}

/// todo_toggle 核心实现：翻转完成态（done_at 随语义落库：勾选置 now、退回清 NULL）
pub fn todo_toggle_core(id: i64, ctx: &AppContext) -> Result<TodoItem, CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.toggle(id)?)
}

/// 改标题（清单行内编辑/详情板标题共用；文本校验同 add）→ 广播变更（预览窗同步）
#[tauri::command]
pub fn todo_rename(
    window: tauri::Window,
    id: i64,
    text: String,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<TodoItem, CommandError> {
    let item = todo_rename_core(id, &text, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(item)
}

/// todo_rename 核心实现：文本校验 + 更新
pub fn todo_rename_core(id: i64, text: &str, ctx: &AppContext) -> Result<TodoItem, CommandError> {
    validate_text(text)?;
    let storage = ctx.lock_storage()?;
    Ok(storage.rename(id, text.trim())?)
}

/// 删除待办（不存在严格报错）→ 广播变更
#[tauri::command]
pub fn todo_remove(
    window: tauri::Window,
    id: i64,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    todo_remove_core(id, &images_dir, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(())
}

/// todo_remove 核心实现：删除条目 + PL027 连带释放其详情图文件（块行随删，清扫按
/// 引用归零删 `note_*` 夹；失败落日志不阻断——文件残留代价小于阻断删除）
pub fn todo_remove_core(
    id: i64,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    storage.remove(id)?;
    if let Err(err) = storage.sweep_orphan_note_images(images_dir) {
        eprintln!("删条目后详情图清扫失败（best-effort 不阻断）：{err}");
    }
    Ok(())
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

/// 清单排序视图（未完成在前按 sort_order 升序（拖拽序）、id 兜底，已完成在后；逐条附龄期档位）
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
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    todo_archive_list_core(&ctx, &images_dir)
}

/// todo_archive_list 核心实现：归档读路径惰性清扫（PATCH001）+ 出归档视图。
/// 清扫挂读路径 = 打卡软件模式在常驻板上的等价形态：本命令调用点天然覆盖 App 挂载
/// （App.vue 启动拉归档）与每次清单变更（onListChanged 三源齐拉），超期条目在出视图
/// 前已删、前端永无中间态，零新线程零定时器；删除数 > 0 落一行日志对账。
/// PL027：回收连带释放其详情图文件（块行随删，清扫按引用归零删 `note_*` 夹）
pub fn todo_archive_list_core(
    ctx: &AppContext,
    images_dir: &std::path::Path,
) -> Result<Vec<TodoItem>, CommandError> {
    let storage = ctx.lock_storage()?;
    let cutoff = storage.now_ms() - ARCHIVE_RETENTION_MS;
    let purged = storage.purge_expired_done(cutoff)?;
    if purged > 0 {
        eprintln!("归档回收：清理 {purged} 条超期条目");
        if let Err(err) = storage.sweep_orphan_note_images(images_dir) {
            eprintln!("归档回收后详情图清扫失败（best-effort 不阻断）：{err}");
        }
    }
    Ok(storage.list_done()?)
}

/// 清单重排（拖拽落点提交）：ids = 未完成条目的目标顺序（全量集，一致性校验在 storage 层）
/// → 广播变更（FIX005.5 补齐：拖拽排序原漏发致预览窗 stale）
#[tauri::command]
pub fn todo_reorder(
    window: tauri::Window,
    ids: Vec<i64>,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    todo_reorder_core(&ids, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(())
}

/// todo_reorder 核心实现：透传 storage.reorder_todos（校验+事务在存储层）
pub fn todo_reorder_core(ids: &[i64], ctx: &AppContext) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.reorder_todos(ids)?)
}

// —— PL027 详情块命令（一段一行一块 + 图块混排；业务在存储层，本层只做取数/广播/把关）——

/// 块视图（跨 IPC）：图块带**绝对路径**（前端 `convertFileSrc` 直用）；大图预览优先
#[derive(Debug, serde::Serialize)]
pub struct NoteBlockView {
    /// 块唯一标识（本地新块在保存前为 0）
    pub id: i64,
    /// 块类型
    pub kind: NoteBlockKind,
    /// 文本内容（图块恒空串）
    pub text: String,
    /// 图片绝对路径（图块专用；文本块 = None）——有 `preview.png` 时回预览（详情秒开）
    pub image_path: Option<String>,
}

/// 块保存入参（前端回传：id > 0 = 既有块、0 = 新块；图块文件信息在库内，无需回传）
#[derive(Debug, serde::Deserialize)]
pub struct NoteBlockInput {
    /// 块 id（0 = 新块）
    pub id: i64,
    /// 块类型
    pub kind: NoteBlockKind,
    /// 文本内容
    pub text: String,
}

/// 粘贴插图结果（跨 IPC）：命中图 → 新块列表（前端整体替换本地态）；未命中 → 回落文本粘贴
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NotePasteOutcome {
    /// 命中剪贴板图片并已插入
    Image { blocks: Vec<NoteBlockView> },
    /// 剪贴板无图片（前端回落文本粘贴）
    NoImage,
}

/// 块 → 视图（解析绝对路径 + 预览优先）
fn to_block_view(block: NoteBlock, images_dir: &std::path::Path) -> NoteBlockView {
    let image_path = block.image_file.as_ref().map(|rel| {
        let abs = images_dir.join(rel);
        match abs.parent().map(|dir| dir.join("preview.png")) {
            Some(preview) if preview.is_file() => preview.to_string_lossy().into_owned(),
            _ => abs.to_string_lossy().into_owned(),
        }
    });
    NoteBlockView {
        id: block.id,
        kind: block.kind,
        text: block.text,
        image_path,
    }
}

/// 读该条目的详情块（开板时按需拉，**不进清单载荷**）
#[tauri::command]
pub fn note_blocks_list(
    todo_id: i64,
    ctx: State<'_, AppContext>,
) -> Result<Vec<NoteBlockView>, CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    note_blocks_list_core(todo_id, &images_dir, &ctx)
}

/// note_blocks_list 核心实现：出块视图（预览优先）
pub fn note_blocks_list_core(
    todo_id: i64,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<Vec<NoteBlockView>, CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage
        .note_blocks(todo_id)?
        .into_iter()
        .map(|b| to_block_view(b, images_dir))
        .collect())
}

/// 全量覆盖保存详情块（前端防抖 300ms 后调用）→ 广播变更
#[tauri::command]
pub fn note_blocks_save(
    window: tauri::Window,
    todo_id: i64,
    blocks: Vec<NoteBlockInput>,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<Vec<NoteBlockView>, CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    let saved = note_blocks_save_core(todo_id, &blocks, &images_dir, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(saved)
}

/// note_blocks_save 核心实现：不变量校验（单点在 todo::validate_note_blocks）+ 差分落库
pub fn note_blocks_save_core(
    todo_id: i64,
    blocks: &[NoteBlockInput],
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<Vec<NoteBlockView>, CommandError> {
    let model: Vec<NoteBlock> = blocks
        .iter()
        .map(|b| NoteBlock {
            id: b.id,
            kind: b.kind,
            text: b.text.clone(),
            image_file: None,
        })
        .collect();
    validate_note_blocks(&model)?;
    let storage = ctx.lock_storage()?;
    let saved = storage.set_note_blocks(images_dir, todo_id, &model)?;
    Ok(saved
        .into_iter()
        .map(|b| to_block_view(b, images_dir))
        .collect())
}

/// 粘贴插图（**唯一插图入口**）：命中剪贴板图片 → 落盘/复用 + 光标处切分插入 → 广播
#[tauri::command]
pub fn note_paste_image(
    window: tauri::Window,
    todo_id: i64,
    block_id: i64,
    before: String,
    after: String,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<NotePasteOutcome, CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    let outcome = note_paste_image_core(todo_id, block_id, &before, &after, &images_dir, &ctx)?;
    if matches!(outcome, NotePasteOutcome::Image { .. }) {
        emit_todo_changed(&window, &app);
    }
    Ok(outcome)
}

/// 剪贴板读取薄壳（不可注入直测）：无图 → `NoImage`（前端回落文本粘贴）
pub fn note_paste_image_core(
    todo_id: i64,
    block_id: i64,
    before: &str,
    after: &str,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<NotePasteOutcome, CommandError> {
    match crate::clipboard_image::read_clipboard_image() {
        Some(captured) => {
            note_insert_image_core(todo_id, block_id, before, after, &captured, images_dir, ctx)
        }
        None => Ok(NotePasteOutcome::NoImage),
    }
}

/// 插图核心（**可注入字节直测**）：格式校验 → 块数上限把关 → 存储层落盘/复用 + 切分插入。
/// `before` / `after` 由前端按光标/选区本地切好（免 UTF-16 ↔ 字节换算）
pub fn note_insert_image_core(
    todo_id: i64,
    block_id: i64,
    before: &str,
    after: &str,
    captured: &crate::clipboard_image::CapturedImage,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<NotePasteOutcome, CommandError> {
    crate::clipboard_image::validate_image_bytes(&captured.bytes)?;
    let stamp = crate::util::local_stamp();
    let storage = ctx.lock_storage()?;
    // 插入后块数 = 现有 + 2（原块复用为 before 文本块）——超限直接拒（不写盘不插块）
    if storage.note_blocks(todo_id)?.len() + 2 > MAX_NOTE_BLOCKS {
        return Err(CommandError::Todo(TodoError::TooManyBlocks.to_string()));
    }
    let blocks = storage.add_note_image_block(
        images_dir,
        todo_id,
        block_id,
        before,
        after,
        &captured.bytes,
        captured.file_name.as_deref(),
        &stamp,
    )?;
    Ok(NotePasteOutcome::Image {
        blocks: blocks
            .into_iter()
            .map(|b| to_block_view(b, images_dir))
            .collect(),
    })
}

/// 详情块重排（图块拖拽落点提交）→ 广播变更
#[tauri::command]
pub fn note_reorder(
    window: tauri::Window,
    todo_id: i64,
    ids: Vec<i64>,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<(), CommandError> {
    note_reorder_core(todo_id, &ids, &ctx)?;
    emit_todo_changed(&window, &app);
    Ok(())
}

/// note_reorder 核心实现：透传 storage.reorder_note_blocks（集合一致性校验在存储层）
pub fn note_reorder_core(todo_id: i64, ids: &[i64], ctx: &AppContext) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    Ok(storage.reorder_note_blocks(todo_id, ids)?)
}

/// 双击图块 → 系统默认程序打开**原图**（跳过预览）
#[tauri::command]
pub fn note_open_image(block_id: i64, ctx: State<'_, AppContext>) -> Result<(), CommandError> {
    let images_dir = crate::paths::images_dir()
        .map_err(|err| CommandError::Storage(format!("图片目录解析失败：{err}")))?;
    note_open_image_core(block_id, &images_dir, &ctx)
}

/// note_open_image 核心实现：块 → 原图绝对路径（**跳过 preview.png**）→ 系统默认程序打开；
/// 文本块 / 文件缺失严格报错
pub fn note_open_image_core(
    block_id: i64,
    images_dir: &std::path::Path,
    ctx: &AppContext,
) -> Result<(), CommandError> {
    let storage = ctx.lock_storage()?;
    let block = storage.get_note_block(block_id)?;
    let rel = block
        .image_file
        .ok_or_else(|| CommandError::Todo("该块不是图片块".to_string()))?;
    let abs = images_dir.join(&rel);
    if !abs.is_file() {
        return Err(CommandError::Todo("图片文件已移动或删除".to_string()));
    }
    crate::util::open_path_with_default(&abs).map_err(CommandError::Todo)
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
        let dir = temp_images_dir();
        let item = todo_add_core("任务一", &ctx).expect("合法文本必须成功");
        todo_remove_core(item.id, &dir, &ctx).expect("刚添加的条目必须存在");
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

    #[test]
    fn archive_list_purges_expired_before_view() {
        // PATCH001：归档读路径惰性清扫全链——超期条目在出视图前已删，前端永无中间态
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
        let item = todo_add_core("老条目", &ctx).expect("合法文本必须成功");
        todo_toggle_core(item.id, &ctx).expect("勾选必须成功"); // done_at = t0
        clock.store(
            t0 + crate::todo::ARCHIVE_RETENTION_MS + 1,
            std::sync::atomic::Ordering::SeqCst,
        );
        let dir = temp_images_dir();
        let view = todo_archive_list_core(&ctx, &dir).expect("归档读必须成功");
        assert!(view.is_empty(), "超期条目在出视图前已回收");
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

    // —— PL027 详情块命令 ——

    /// 临时图片目录（禁触真实 data/images）
    fn temp_images_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "capsule-todo-note-images-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("临时图片目录必须可建");
        dir
    }

    /// 独立夹具：image crate 编码 1×1 PNG（合法格式，供格式嗅探过闸）
    fn fixture_png() -> Vec<u8> {
        let img = image::RgbaImage::new(1, 1);
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .expect("夹具编码必须成功");
        out
    }

    fn input(id: i64, kind: NoteBlockKind, text: &str) -> NoteBlockInput {
        NoteBlockInput {
            id,
            kind,
            text: text.to_string(),
        }
    }

    #[test]
    fn note_blocks_save_roundtrip_and_validation() {
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = todo_add_core("任务", &ctx).expect("合法文本必须成功");
        let saved = note_blocks_save_core(
            item.id,
            &[
                input(0, NoteBlockKind::Text, "第一行"),
                input(0, NoteBlockKind::Text, "第二行"),
            ],
            &dir,
            &ctx,
        )
        .expect("保存必须成功");
        assert_eq!(saved.len(), 2);
        assert!(saved.iter().all(|b| b.kind == NoteBlockKind::Text));
        // has_note 口径（PL027：任一块非空）
        let view = todo_list_core(&ctx).expect("读命令必须成功");
        assert!(view[0].item.has_note, "有文本块 → has_note 亮");
        // 不变量：文本块含换行 → 可见报错
        let err = note_blocks_save_core(
            item.id,
            &[input(0, NoteBlockKind::Text, "a\nb")],
            &dir,
            &ctx,
        )
        .expect_err("换行必须被拒");
        assert!(matches!(err, CommandError::Todo(_)));
        // 不变量：仅图块（无文本块）→ 可见报错
        let err = note_blocks_save_core(item.id, &[input(0, NoteBlockKind::Image, "")], &dir, &ctx)
            .expect_err("缺文本块必须被拒");
        assert!(matches!(err, CommandError::Todo(_)));
        // 读回（文本块无图路径）
        let listed = note_blocks_list_core(item.id, &dir, &ctx).expect("读块必须成功");
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|b| b.image_path.is_none()));
    }

    #[test]
    fn note_insert_image_splits_and_reuses() {
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = todo_add_core("任务", &ctx).expect("合法文本必须成功");
        let saved = note_blocks_save_core(
            item.id,
            &[input(0, NoteBlockKind::Text, "前半后半")],
            &dir,
            &ctx,
        )
        .expect("保存必须成功");
        let captured = crate::clipboard_image::CapturedImage {
            bytes: fixture_png(),
            file_name: None,
        };
        // 块内选区/光标处切分：块变 3 个（before / 图 / after）
        let out =
            note_insert_image_core(item.id, saved[0].id, "前半", "后半", &captured, &dir, &ctx)
                .expect("插图必须成功");
        let NotePasteOutcome::Image { blocks } = out else {
            panic!("必须命中图");
        };
        assert_eq!(blocks.len(), 3, "块变 3 个");
        assert_eq!(blocks[0].text, "前半");
        assert_eq!(blocks[2].text, "后半");
        assert!(
            blocks[1]
                .image_path
                .as_deref()
                .unwrap_or("")
                .ends_with(".png"),
            "图块带绝对路径"
        );
        // 同字节第二次插入 → 复用同一文件（域内引用计数）
        let again = note_insert_image_core(item.id, blocks[0].id, "", "", &captured, &dir, &ctx)
            .expect("再插必须成功");
        let NotePasteOutcome::Image { blocks: second } = again else {
            panic!("必须命中图");
        };
        let paths: Vec<String> = second.iter().filter_map(|b| b.image_path.clone()).collect();
        assert_eq!(paths.len(), 2, "两个图块");
        assert_eq!(paths[0], paths[1], "同内容复用同一文件");
    }

    #[test]
    fn note_reorder_core_roundtrip_and_mismatch() {
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = todo_add_core("任务", &ctx).expect("合法文本必须成功");
        let saved = note_blocks_save_core(
            item.id,
            &[
                input(0, NoteBlockKind::Text, "甲"),
                input(0, NoteBlockKind::Text, "乙"),
            ],
            &dir,
            &ctx,
        )
        .expect("保存必须成功");
        let ids: Vec<i64> = saved.iter().map(|b| b.id).collect();
        note_reorder_core(item.id, &[ids[1], ids[0]], &ctx).expect("重排必须成功");
        let listed = note_blocks_list_core(item.id, &dir, &ctx).expect("读块必须成功");
        assert_eq!(listed[0].text, "乙");
        assert!(
            note_reorder_core(item.id, &[ids[0]], &ctx).is_err(),
            "部分集拒绝"
        );
    }

    #[test]
    fn note_open_image_rejects_text_block_and_missing() {
        let ctx = test_context();
        let dir = temp_images_dir();
        let item = todo_add_core("任务", &ctx).expect("合法文本必须成功");
        let saved = note_blocks_save_core(
            item.id,
            &[input(0, NoteBlockKind::Text, "文字")],
            &dir,
            &ctx,
        )
        .expect("保存必须成功");
        let err = note_open_image_core(saved[0].id, &dir, &ctx).expect_err("文本块必须报错");
        assert!(matches!(err, CommandError::Todo(ref msg) if msg.contains("不是图片块")));
        assert!(
            note_open_image_core(999, &dir, &ctx).is_err(),
            "不存在块报错"
        );
    }
}
