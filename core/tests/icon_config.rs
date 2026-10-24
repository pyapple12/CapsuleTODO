//! 图标配置断言（FIX017.26）：`bundle.icon` 必须与 `core/icons/` 的生成物一一对应。
//! 防止生成脚本已产出 64px、配置却漏列（Tauri 只能拿 32/128 缩放）的漂移。

use std::fs;
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

/// FIX018.21：Tauri 只取 ICO 目录首帧作为窗口图标；首帧必须 ≥48，否则任务栏放大发糊。
#[test]
fn icon_ico_directory_first_frame_is_at_least_48() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("icons")
        .join("icon.ico");
    let bytes = fs::read(&path).expect("icon.ico 必须可读");
    assert!(bytes.len() >= 6 + 16, "ICO 头 + 至少一个目录项");
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0, "ICO reserved");
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 1, "ICO type");
    let count = u16::from_le_bytes([bytes[4], bytes[5]]);
    assert!(count > 0, "ICO 至少一帧");
    let first = bytes[6];
    let first = if first == 0 { 256 } else { first as u16 };
    assert!(first >= 48, "ICO 目录首帧必须 ≥48，实际 {first}");
}
