//! CapsuleTODO 薄入口：仅调用应用装配层，启动失败如实退出非零。
//!
//! windows_subsystem 仅 release 生效（FIX008.4）：GUI 子系统免直启黑控制台窗；
//! debug/test 保留 console 子系统——stderr 日志通道（容错白名单"落日志"承诺）
//! 依赖它可见，打包发布若补文件日志再回改。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(err) = capsule_todo::run() {
        eprintln!("应用启动失败：{err}");
        std::process::exit(1);
    }
}
