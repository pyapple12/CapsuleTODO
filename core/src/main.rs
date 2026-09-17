//! CapsuleTODO 薄入口：仅调用应用装配层，启动失败如实退出非零。

fn main() {
    if let Err(err) = capsule_todo::run() {
        eprintln!("应用启动失败：{err}");
        std::process::exit(1);
    }
}
