import { createApp } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App.vue";
import TrayPreview from "./src/components/TrayPreview.vue";
import "./style.css";
import "./src/styles/board-read.css";
import "./src/styles/archive.css";
import "./src/styles/settings.css";
import "./src/styles/todos.css";
import "./src/styles/bubbles.css";
import "./src/styles/whiteboard.css";
import { installMockInvoke } from "./src/dev/mock-invoke";

// ===== WebView 网页行为抑制（2026-09-28 目验定案）：F5/Ctrl+R 刷新与右键菜单是
// 系统 WebView 的网页默认行为，非桌面 app 语义——前端拦截关闭（跨平台零依赖）。
// 拦截面收敛：keydown 只吞刷新组合键（app 内无刷新语义），contextmenu 只吞默认
// 菜单；拖拽重排、行内编辑、开发者工具均不受影响
function installWebViewGuards(): void {
  window.addEventListener(
    "keydown",
    (e) => {
      if (e.key === "F5" || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "r")) {
        e.preventDefault();
      }
    },
    { capture: true },
  );
  window.addEventListener("contextmenu", (e) => e.preventDefault());
}

// PL008.1 DEV 冒烟基座：纯浏览器（无 Tauri runtime）时把 invoke 路由到内存模拟——
// vite dev + IAB 是 APP 回归期的自动化验证通道；DEV 死分支，生产构建静态消除
if (import.meta.env.DEV) {
  installMockInvoke();
}
installWebViewGuards();

// 应用入口：按窗口 label 分流（PL018.4）——tray-preview 窗挂悬浮预览组件，
// 主窗挂三页签骨架；纯浏览器（IAB/无 Tauri runtime）走主窗分支（mock 基座）
function currentWindowLabel(): string {
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
}

if (currentWindowLabel() === "tray-preview") {
  createApp(TrayPreview).mount("#app");
} else {
  createApp(App).mount("#app");
}
