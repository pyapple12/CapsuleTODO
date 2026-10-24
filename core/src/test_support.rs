//! 测试辅助：临时目录 RAII 收尾（FIX017.14）。
//!
//! 为什么放在独立模块：图片测试在 storage / commands 三处都要建临时目录，
//! 若各自实现 Drop 容易漂移；这里用 `Arc` 内层保证 `dir.clone()` 进线程时
//! 不会提前删除目录，最后一个 guard 释放才收尾。

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 测试用临时图片目录（Clone 共享同一目录；最后一个 guard 释放时删除）。
#[derive(Clone)]
pub(crate) struct TempImagesDir {
    inner: Arc<TempImagesDirInner>,
}

struct TempImagesDirInner {
    path: PathBuf,
}

impl TempImagesDir {
    /// 创建唯一临时目录；`prefix` 只用于可读命名（如 `capsule-todo-images`）。
    pub(crate) fn new(prefix: &str) -> Self {
        static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "{prefix}-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
            TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("临时图片目录必须可建");
        Self {
            inner: Arc::new(TempImagesDirInner { path }),
        }
    }
}

impl Deref for TempImagesDir {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.inner.path
    }
}

impl AsRef<Path> for TempImagesDir {
    fn as_ref(&self) -> &Path {
        &self.inner.path
    }
}

impl Drop for TempImagesDirInner {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_dir_all(&self.path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "测试临时目录清理失败（best-effort）：{}（{err}）",
                    self.path.display()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TempImagesDir;

    #[test]
    fn clone_keeps_directory_until_last_guard_drops() {
        let dir = TempImagesDir::new("capsule-todo-test-support");
        let clone = dir.clone();
        let path = dir.to_path_buf();
        drop(dir);
        assert!(path.is_dir(), "还有 clone 持有目录时不得删除");
        drop(clone);
        assert!(!path.exists(), "最后一个 guard 释放后必须删除");
    }

    #[test]
    fn drop_ignores_already_removed_directory() {
        let dir = TempImagesDir::new("capsule-todo-test-support");
        let path = dir.to_path_buf();
        std::fs::remove_dir_all(&path).expect("先手动删除必须成功");
        drop(dir); // Drop 内 NotFound 必须静默忽略，不 panic
    }
}
