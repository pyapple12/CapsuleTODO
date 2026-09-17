//! Tauri 命令层：共享上下文与错误封装（命令按职责分文件）。

pub mod bubble;
pub mod todo;
pub mod whiteboard;

use std::sync::{LockResult, Mutex, MutexGuard, PoisonError};

use crate::bubble::BubbleError;
use crate::storage::{Storage, StorageError};
use crate::todo::TodoError;
use crate::whiteboard::WhiteboardError;

/// 应用共享上下文：清单存储（单一事实源 = db；锁序 todo → storage 单向禁反向）
pub struct AppContext {
    pub storage: Mutex<Storage>,
}

impl AppContext {
    /// 取存储锁；中毒严格报错（禁静默恢复）
    pub fn lock_storage(&self) -> Result<MutexGuard<'_, Storage>, CommandError> {
        poison(self.storage.lock())
    }
}

/// 锁中毒收敛助手：LockResult → Result&lt;T, CommandError&gt;（命令层锁取用的统一出口）
pub(crate) fn poison<T>(result: LockResult<T>) -> Result<T, CommandError> {
    result.map_err(|_: PoisonError<T>| CommandError::Poisoned)
}

/// 命令层错误（跨 IPC 序列化为可读纯字符串，前端 String(err) 直显）：业务/存储/剪贴板错误透传可读原因 + 锁中毒
#[derive(Debug)]
pub enum CommandError {
    /// 业务规则拒绝（文本非法），承载错误说明
    Todo(String),
    /// 存储层失败（SQLite/IO/条目不存在），承载错误说明
    Storage(String),
    /// 剪贴板读写失败或剪贴板内容不可用，承载错误说明
    Clipboard(String),
    /// 白板内容非法，承载错误说明
    Whiteboard(String),
    /// 共享锁中毒（持锁线程 panic 后遗症，不可恢复）
    Poisoned,
}

// 序列化契约（A001-P3-1 修复）：按变体输出可读字符串而非对象形态（derive Serialize
// 产出 {"Todo":"…"}，前端 String(err) 只能得到 [object Object]）
impl serde::Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            CommandError::Todo(msg)
            | CommandError::Storage(msg)
            | CommandError::Clipboard(msg)
            | CommandError::Whiteboard(msg) => serializer.serialize_str(msg),
            CommandError::Poisoned => serializer.serialize_str("共享锁中毒"),
        }
    }
}

impl From<TodoError> for CommandError {
    fn from(err: TodoError) -> Self {
        CommandError::Todo(err.to_string())
    }
}

impl From<StorageError> for CommandError {
    fn from(err: StorageError) -> Self {
        CommandError::Storage(err.to_string())
    }
}

impl From<BubbleError> for CommandError {
    fn from(err: BubbleError) -> Self {
        CommandError::Clipboard(err.to_string())
    }
}

impl From<WhiteboardError> for CommandError {
    fn from(err: WhiteboardError) -> Self {
        CommandError::Whiteboard(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::CommandError;

    #[test]
    fn command_error_serializes_to_readable_string() {
        // 跨 IPC 契约：序列化结果必须是可读纯字符串（前端 String(err) 直显）。
        // 修复前 derive Serialize 产出对象形态（如 {"Todo":"x"}），AddBar 错误行显示
        // "添加失败：[object Object]"（A001-P3-1 缺陷本体），此断言用于锁死契约
        assert_eq!(
            serde_json::to_string(&CommandError::Todo("待办文本为空".into()))
                .expect("序列化必须成功"),
            "\"待办文本为空\""
        );
        assert_eq!(
            serde_json::to_string(&CommandError::Storage("待办条目不存在：9".into()))
                .expect("序列化必须成功"),
            "\"待办条目不存在：9\""
        );
        assert_eq!(
            serde_json::to_string(&CommandError::Poisoned).expect("序列化必须成功"),
            "\"共享锁中毒\""
        );
        assert_eq!(
            serde_json::to_string(&CommandError::Clipboard("剪贴板无文本内容".into()))
                .expect("序列化必须成功"),
            "\"剪贴板无文本内容\""
        );
        assert_eq!(
            serde_json::to_string(&CommandError::Whiteboard("白板内容过长".into()))
                .expect("序列化必须成功"),
            "\"白板内容过长\""
        );
    }
}
