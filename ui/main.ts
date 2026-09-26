import { createApp } from "vue";
import App from "./App.vue";
import "./style.css";
import "./src/styles/board-read.css";
import { installMockInvoke } from "./src/dev/mock-invoke";

// PL008.1 DEV 冒烟基座：纯浏览器（无 Tauri runtime）时把 invoke 路由到内存模拟——
// vite dev + IAB 是 APP 回归期的自动化验证通道；DEV 死分支，生产构建静态消除
if (import.meta.env.DEV) {
  installMockInvoke();
}

// 应用入口：挂载 Vue 根组件（展示层骨架，业务逻辑零含量）
createApp(App).mount("#app");
