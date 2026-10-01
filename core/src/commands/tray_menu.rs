//! 托盘菜单窗命令（PL021）：自绘 HTML 菜单窗的菜单项动作分发——收起菜单窗
//! 后按 action 执行（显示主窗 / 贴边与置顶勾选复用 on_pref_menu 核心 / 退出
//! 走主窗 close() 复用关窗保存链）。原生菜单事件分发（PL018）随原生菜单弃用
//! 而移除，分发语义原样迁移至此。

use tauri::Manager;

use super::CommandError;

/// 菜单项动作统一分发（TrayMenu.vue 点击调用）：先收菜单窗（点项必收，原生
/// 菜单同款），再执行动作。action 全集 = show / snap / top / exit
#[tauri::command]
pub fn tray_menu_action(action: String, app: tauri::AppHandle) -> Result<(), CommandError> {
    if let Some(menu_win) = app.get_webview_window("tray-menu") {
        let _ = menu_win.hide();
    }
    match action.as_str() {
        // 显示主窗（原 tray-show 分发语义）
        "show" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
            Ok(())
        }
        // 贴边/置顶勾选（原 tray-snap/tray-top 分发语义）：落库与广播在
        // on_pref_menu 内完成，失败已落日志（事实源读取纠正）
        "snap" => {
            crate::tray::on_pref_menu(&app, "tray-snap");
            Ok(())
        }
        "top" => {
            crate::tray::on_pref_menu(&app, "tray-top");
            Ok(())
        }
        // 退出（用户定案变更：托盘模式需要退出入口）——**必须销毁全部窗口**：
        // 主窗 close() 复用完整关窗保存链（位置/设置落库 + T+R3 双腿）；两个托盘
        // 隐藏窗（tray-preview/tray-menu）若只销毁主窗会撑住事件循环致进程不退
        //（实测：主窗消失但进程存活，托盘图标健在——曾被误判为"幽灵图标"）。
        // 全窗销毁后事件循环自然退出，App drop 删托盘 + ExitRequested 显式删除
        // 双保险。禁 app.exit(0) 绕过保存链（PL019 定案）
        "exit" => {
            if let Some(w) = app.get_webview_window("main") {
                if let Err(err) = w.close() {
                    eprintln!("托盘退出主窗关闭失败：{err}");
                }
            }
            for label in ["tray-preview", "tray-menu"] {
                if let Some(w) = app.get_webview_window(label) {
                    if let Err(err) = w.destroy() {
                        eprintln!("托盘退出清理 {label} 窗失败：{err}");
                    }
                }
            }
            Ok(())
        }
        other => Err(CommandError::Settings(format!("未知菜单动作：{other}"))),
    }
}
