//! 开发期诊断插件（FIX014.38，仅 `dev-diag` feature + debug 构建）。
//!
//! PATCH002 的前端诊断打点通过本插件写入 Rust stdout；release 不编译本模块、
//! 不注册插件，也不保留 `debug_log` 命令。这样诊断能力只在 `tauri dev`
//! 构建可用，生产错误诊断仍走 `diag::warn_diag` / `data/todo.log`。

use tauri::plugin::TauriPlugin;
use tauri::Runtime;

/// 前端诊断行进入 Rust stdout（仅 dev-diag 构建注册）。
#[tauri::command]
fn debug_log(line: String) {
    println!("[DIAG] {line}");
}

/// 构建 dev-only 诊断插件：命令名 `dev-diag|debug_log`。
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("dev-diag")
        .invoke_handler(tauri::generate_handler![debug_log])
        .build()
}
