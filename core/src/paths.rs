//! 运行时数据双落址解析（沿系列定案）：dev = 仓库根（CARGO_MANIFEST_DIR 在 core/ 内，
//! 项目根 = 其父目录，编译期保证），release = exe 同级；目录缺失自建。
//! 测试一律经 *_under 注入临时目录，禁触真实 data/。

use std::path::{Path, PathBuf};

/// 运行时根：dev 取仓库根，release 取当前 exe 所在目录（非法环境严格报错）
pub fn runtime_root() -> std::io::Result<PathBuf> {
    #[cfg(debug_assertions)]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| std::io::Error::other("Cargo 清单目录无父目录，无法定位项目根"))
    }
    #[cfg(not(debug_assertions))]
    {
        let exe = std::env::current_exe()?;
        let dir = exe
            .parent()
            .ok_or_else(|| std::io::Error::other("exe 路径无父目录，无法定位运行时根"))?;
        Ok(dir.to_path_buf())
    }
}

/// 基于指定根解析数据目录（缺失自建）；生产传 runtime_root()，测试注入临时目录
pub fn data_dir_under(root: &Path) -> std::io::Result<PathBuf> {
    let dir = root.join("data");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 数据目录：<运行时根>/data（双落址 dev=项目根 / release=exe 同级）
pub fn data_dir() -> std::io::Result<PathBuf> {
    data_dir_under(&runtime_root()?)
}

/// 清单库路径：<运行时根>/data/todo.db
pub fn db_path() -> std::io::Result<PathBuf> {
    Ok(data_dir()?.join("todo.db"))
}

/// 基于指定根解析配置目录（缺失自建）；生产传 runtime_root()，测试注入临时目录
pub fn configs_dir_under(root: &Path) -> std::io::Result<PathBuf> {
    let dir = root.join("configs");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 窗口设置路径：<运行时根>/configs/config.json（双落址沿系列）
pub fn settings_path() -> std::io::Result<PathBuf> {
    Ok(configs_dir_under(&runtime_root()?)?.join("config.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_under_creates_dir() {
        let root = std::env::temp_dir().join(format!("capsule-todo-paths-{}", std::process::id()));
        let dir = data_dir_under(&root).expect("目录自建必须成功");
        assert!(dir.is_dir());
        assert!(dir.ends_with("data"));
        std::fs::remove_dir_all(&root).expect("清理必须成功");
    }

    #[test]
    fn configs_dir_under_creates_dir() {
        let root =
            std::env::temp_dir().join(format!("capsule-todo-paths-c-{}", std::process::id()));
        let dir = configs_dir_under(&root).expect("目录自建必须成功");
        assert!(dir.is_dir());
        assert!(dir.ends_with("configs"));
        std::fs::remove_dir_all(&root).expect("清理必须成功");
    }

    #[test]
    fn runtime_root_exists() {
        assert!(runtime_root().expect("运行时根必须可解析").is_dir());
    }
}
