//! 托盘预览窗命令（PL018.5）：预览窗高度随内容调整（Rust set_size 直调 +
//! 底缘锚定重算）。显隐已收归 tray.rs 单一守候线程（前端不再上报悬停状态
//! ——守候线程的光标轮询全条件覆盖 mouseenter/mouseleave 丢失场景）

use tauri::Manager;

use super::CommandError;

/// 预览窗高度随内容（PL018，用户定案 <5 条自动减高）：Rust set_size 直调
///（JS setSize 需 ACL allow-set-size，core:default 不含会静默拒绝）。可见态
/// resize 后 1 帧切换（DWM 圆角已切掉闪帧直角，观感代价可接受——用户实测后
/// 拍板恢复实时缩高）；隐藏态仅 set_size（无残影），Enter 定位按实际尺寸
#[tauri::command]
pub fn tray_preview_resize(height: f64, app: tauri::AppHandle) -> Result<(), CommandError> {
    let Some(w) = app.get_webview_window("tray-preview") else {
        return Ok(());
    };
    w.set_size(tauri::LogicalSize::new(crate::tray::PREVIEW_WIDTH, height))
        .map_err(|err| CommandError::Settings(format!("预览窗高度调整失败：{err}")))?;
    // resize 后底缘锚定托盘重算 y（上缘缩向托盘方向——用户定案的收缩方向）
    crate::tray::reanchor_to_tray(&w);
    Ok(())
}
