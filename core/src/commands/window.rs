//! 窗口级命令（PATCH002.60）：板开合驱动的系统背板（按需毛玻璃）唯一入口。
//!
//! 语义：`open = true` 挂系统亚克力（`DWMSBT_TRANSIENTWINDOW`）、`false` 摘回纯透明
//! （`DWMSBT_NONE`）；由前端在"**板开合 + 窗口聚焦**"变化时调用（**失焦必须传 `false`**
//! ——DWM 材质焦点绑定，不摘会退回不透明灰板）。

use tauri::{AppHandle, Manager};

use crate::glass_backdrop;

/// 板开合驱动的系统背板（PATCH002.60）：按需挂摘系统亚克力。
///
/// **返回 `()` 不抛错**：材质是纯装饰层，失败仅落诊断日志、维持前态（容错白名单
/// "焦点联动材质切换失败维持前态"同条纪律）——前端 `catch` 只落 console，UI 静默。
///
/// # 参数
/// - `open`：`true` = 挂亚克力；`false` = 摘回纯透明。
#[tauri::command]
pub fn set_board_backdrop(app: AppHandle, open: bool) {
    let Some(window) = app.get_webview_window("main") else {
        crate::diag::warn_diag("主窗口缺失（系统背板未切换）");
        return;
    };
    glass_backdrop::set_backdrop(&window, open);
}
