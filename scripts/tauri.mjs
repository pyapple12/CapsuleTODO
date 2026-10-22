// FIX014.38：`npm run tauri dev` 旧习惯保留，但必须自动带 dev-diag feature 与
// dev capability 配置；否则前端是 build:diag、Rust 却没注册诊断插件。
// build / other 子命令原样透传，release 构建不注入 dev-only 参数。
import { spawn } from "node:child_process";
import { createRequire } from "node:module";

const args = process.argv.slice(2);
const commandArgs = [...args];

if (args[0] === "dev") {
  const rest = args.slice(1);
  const hasFeatures = rest.some((arg) => arg === "--features" || arg.startsWith("--features="));
  const hasConfig = rest.some((arg) => arg === "--config" || arg.startsWith("--config="));
  const injected = [];
  if (!hasFeatures) injected.push("--features", "dev-diag");
  if (!hasConfig) injected.push("--config", "core/tauri.dev.conf.json");
  commandArgs.splice(1, 0, ...injected);
}

// 直接跑本地 CLI 的 JS 入口：绕开 .cmd 的 shell 拼接，Windows 下不触发
// Node 的 DEP0190 警告，也不依赖 PATH 里是否有 tauri.cmd。
const require = createRequire(import.meta.url);
const cliEntry = require.resolve("@tauri-apps/cli/tauri.js");
const child = spawn(process.execPath, [cliEntry, ...commandArgs], { stdio: "inherit" });

child.on("error", (err) => {
  console.error("tauri 启动失败", err);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});
