//! 诊断日志（PATCH002.62）：把"失败必须明确记录"的承诺在 **release 也兑现**——
//! release 走 GUI 子系统（`main.rs` 的 `windows_subsystem = "windows"`）**没有控制台**，
//! `eprintln!` 全部落空；本模块把关键失败路径落到 `<运行时根>/data/todo.log`。
//!
//! 三个出口（分派口径，调用点按语义选一个）：
//! - [`log`]：**只落档**（不需要控制台噪声的场合）
//! - [`warn_diag`]：**控制台 + 落档**（失败路径默认选择——debug 期可见、release 有档）
//! - [`console`]：**只控制台**（纯开发诊断；release 无控制台即丢，语义明确）
//!
//! 设计取舍（沿系列 CapsulePulse `diag.rs`）：**不引日志框架**（YAGNI）；写失败
//! **静默忽略**——日志自身不得引发二次故障（容错白名单"诊断日志写失败静默忽略"同条）；
//! **测试不落盘**（`cfg(test)` 隔离）——单测在 debug 下运行时根 = 项目根，写盘会污染
//! 真实 `data/todo.log`（系列项目曾因此被误判"实机故障"，故必须隔离）。
//!
//! 本模块为**纯逻辑模块**（不 import tauri，可 `cargo test` 直测）；时间戳复用
//! [`crate::util::local_stamp`]（零依赖）。

use std::io::Write;

/// 追加一行诊断（本地时间戳前缀）：路径经 [`crate::paths::default_log_path`] 解析，
/// 任何失败静默忽略（容错白名单"诊断日志写失败静默忽略"）。测试进程不写真实日志。
pub fn log(line: &str) {
    #[cfg(test)]
    {
        // 测试隔离：不触碰真实 data/todo.log（防污染误判）
        let _ = line;
    }
    #[cfg(not(test))]
    {
        let Ok(path) = crate::paths::default_log_path() else {
            return;
        };
        append(&path, &format!("[{}] {line}", crate::util::local_stamp()));
    }
}

/// 控制台 + 落档（失败路径的默认出口）：debug 期控制台可见，release 落 `data/todo.log`。
pub fn warn_diag(line: &str) {
    eprintln!("{line}");
    log(line);
}

/// 只落控制台（纯开发诊断）：release 无控制台 ⇒ 该行丢弃，**语义明确**（非失败路径）。
pub fn console(line: &str) {
    eprintln!("{line}");
}

/// 向指定文件追加一行（路径注入可测）；打开 / 写入失败静默忽略。
fn append(path: &std::path::Path, line: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 追加：多行按序落盘（诊断日志的核心语义）。
    #[test]
    fn append_writes_lines_in_order() {
        let dir = std::env::temp_dir().join(format!("capsule-todo-diag-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("probe.log");
        let _ = std::fs::remove_file(&path);

        append(&path, "第一行");
        append(&path, "第二行");

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "第一行\n第二行\n");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    /// 不可写路径静默忽略（容错白名单：日志失败不引发二次故障）。
    #[test]
    fn append_failure_is_silent() {
        append(
            std::path::Path::new("Z:/capsule-todo-不可能存在的目录/probe.log"),
            "不应 panic",
        );
    }

    /// 回归守护：测试进程调用 `log()` 不得改动真实运行时日志（`cfg(test)` 隔离）。
    #[test]
    fn log_does_not_touch_real_runtime_log() {
        let path = crate::paths::default_log_path().unwrap();
        let before = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        log("测试不得写入真实日志");
        let after = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        assert_eq!(before, after, "log() 在测试进程不得写真实 data/todo.log");
    }
}
