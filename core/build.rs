//! Tauri 构建脚本：生成应用上下文与 ACL 资源。

fn main() {
    // FIX014.38：dev-diag 是应用内联插件；ACL 需要显式声明其命令权限，
    // 否则 `plugin:dev-diag|debug_log` 会被 IPC ACL 拒绝。
    // 仅在 dev-diag feature + 非 release profile 下生成插件 ACL，与 lib.rs 的
    // `feature + debug_assertions` 双门控保持一致；release 不引入该权限/命令名。
    let dev_profile = std::env::var("PROFILE")
        .map(|profile| profile != "release")
        .unwrap_or(false);
    let mut attrs = tauri_build::Attributes::new();
    if std::env::var_os("CARGO_FEATURE_DEV_DIAG").is_some() && dev_profile {
        attrs = attrs.plugin(
            "dev-diag",
            tauri_build::InlinedPlugin::new()
                .commands(&["debug_log"])
                .default_permission(tauri_build::DefaultPermissionRule::AllowAllCommands),
        );
    }
    tauri_build::try_build(attrs).expect("Tauri build 脚本失败");
}
