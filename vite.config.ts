import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// Vite 配置：Vue 插件 + 固定端口；不配 devUrl，构建产物内嵌进 Tauri exe（自包含，规避 dev 端口依赖）
export default defineConfig(({ mode }) => ({
  plugins: [vue()],
  clearScreen: false,
  define: {
    // FIX014.38：诊断构建才注入 true；普通 dev/release 构建为 false，
    // 配合 `if (DIAG)` 可在打包阶段剔除诊断调用与字符串。
    __DIAG__: JSON.stringify(mode === "diag"),
  },
  server: {
    port: 5173,
    strictPort: true,
    // core/target 与 .temp 在 watch 排除段：cargo 编译期间产物文件句柄被锁，chokidar
    // watch 会 EBUSY 直接崩掉 dev 服务（Windows 实测，2026-09-27 PL013 期间两次中招）
    watch: {
      ignored: ["**/core/target/**", "**/dist/**", "**/data/**", "**/.temp/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
}));
