// FIX014.38：`npm run tauri dev` 旧习惯保留，但必须自动带 dev-diag feature 与
// dev capability 配置；否则前端是 build:diag、Rust 却没注册诊断插件。
// build / other 子命令原样透传，release 构建不注入 dev-only 参数。
import { spawn } from "node:child_process";
import { createRequire } from "node:module";

const args = process.argv.slice(2);

/** 识别长/短 config 选项，避免重复注入 dev 配置。 */
function hasConfigArg(list) {
  return list.some(
    (arg) =>
      arg === "--config" || arg.startsWith("--config=") || arg === "-c" || arg.startsWith("-c="),
  );
}

/** 把 dev-diag 合并进用户自定义 features，而不是直接跳过注入。 */
function ensureDevFeature(list) {
  const inline = list.findIndex((arg) => arg.startsWith("--features="));
  if (inline >= 0) {
    const value = list[inline].slice("--features=".length);
    if (!value.split(",").includes("dev-diag")) {
      list[inline] = `--features=${value ? `${value},` : ""}dev-diag`;
    }
    return;
  }
  const index = list.findIndex((arg) => arg === "--features" || arg === "-f");
  if (index >= 0) {
    const value = list[index + 1];
    if (value && !value.startsWith("-")) {
      if (!value.split(",").includes("dev-diag")) {
        list[index + 1] = `${value},dev-diag`;
      }
    } else {
      list.splice(index + 1, 0, "dev-diag");
    }
    return;
  }
  list.unshift("--features", "dev-diag");
}

let commandArgs = [...args];
if (args[0] === "dev") {
  const rest = args.slice(1);
  if (rest.includes("--release")) {
    console.error("tauri:dev 不支持 --release：诊断构建必须在 debug profile 下运行");
    process.exit(1);
  }
  if (!hasConfigArg(rest)) {
    rest.unshift("--config", "core/tauri.dev.conf.json");
  }
  ensureDevFeature(rest);
  commandArgs = ["dev", ...rest];
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
