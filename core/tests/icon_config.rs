//! 图标配置断言（FIX017.26）：`bundle.icon` 必须与 `core/icons/` 的生成物一一对应。
//! 防止生成脚本已产出 64px、配置却漏列（Tauri 只能拿 32/128 缩放）的漂移。

use std::path::Path;

/// bundle.icon 必须列全五个 PNG + 多尺寸 ICO，且文件真实存在。
#[test]
fn bundle_icon_lists_all_generated_files() {
    let raw = include_str!("../tauri.conf.json");
    let cfg: serde_json::Value =
        serde_json::from_str(raw).expect("tauri.conf.json 必须是合法 JSON");
    let icons: Vec<&str> = cfg["bundle"]["icon"]
        .as_array()
        .expect("bundle.icon 必须是数组")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();

    for name in [
        "32x32.png",
        "64x64.png",
        "128x128.png",
        "128x128@2x.png",
        "icon.ico",
        "icon.png",
    ] {
        let expected = format!("icons/{name}");
        assert!(
            icons.contains(&expected.as_str()),
            "bundle.icon 缺少 {expected}"
        );
        assert!(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("icons")
                .join(name)
                .is_file(),
            "图标文件不存在：{name}"
        );
    }
}
