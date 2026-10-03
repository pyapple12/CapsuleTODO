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
        if let Err(err) = menu_win.hide() {
            eprintln!("菜单窗收起失败：{err}");
        }
    }
    match action.as_str() {
        // 显示主窗（原 tray-show 分发语义；PL022 定案更名"聚焦主窗"）
        "show" => {
            if let Some(w) = app.get_webview_window("main") {
                if let Err(err) = w.show() {
                    eprintln!("聚焦主窗 show 失败：{err}");
                }
                if let Err(err) = w.set_focus() {
                    eprintln!("聚焦主窗 set_focus 失败：{err}");
                }
            } else {
                eprintln!("聚焦主窗失败：主窗不存在（异常态）");
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
        // FIX005.4 hide 语义配套：先置 EXITING 标志，主窗 CloseRequested 见标志走
        // 真退销毁链（区别于 Alt+F4 的"保存+隐藏"）。主窗 close() 保留完整关窗
        // 保存链（位置/设置落库）；两个托盘隐藏窗（tray-preview/tray-menu）同步
        // destroy——只销毁主窗会被它们撑住事件循环致进程不退（V0.1.8.0 实测）。
        // 禁 app.exit(0) 绕过保存链（PL019 定案）
        "exit" => {
            crate::EXITING.store(true, std::sync::atomic::Ordering::Relaxed);
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
        other => Err(CommandError::Window(format!("未知菜单动作：{other}"))),
    }
}

/// 双击标题缩回托盘（用户需求 2026-10-03）：复用 Alt+F4 关窗链——main.close()
/// 触发 CloseRequested，EXITING 未置位即走既有"保存位置 + 隐藏到托盘"分支
/// （零逻辑重复；真退仍只走托盘菜单 exit）
#[tauri::command]
pub fn main_hide_to_tray(app: tauri::AppHandle) -> Result<(), CommandError> {
    if let Some(w) = app.get_webview_window("main") {
        if let Err(err) = w.close() {
            eprintln!("缩回托盘主窗关闭失败：{err}");
        }
    } else {
        eprintln!("缩回托盘失败：主窗不存在（异常态）");
    }
    Ok(())
}
