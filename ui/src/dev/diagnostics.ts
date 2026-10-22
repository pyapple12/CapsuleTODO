import { invoke } from "@tauri-apps/api/core";

declare const __DIAG__: boolean;

/**
 * PATCH002 诊断开关：仅 `vite build --mode diag` 注入 true；
 * 普通 dev/release 构建注入 false，release 不保留诊断调用与字符串。
 */
export const DIAG = __DIAG__;

/**
 * 诊断上报：仅 diag 构建调用 dev-only 插件。
 * release 下该函数为 no-op，且不访问 `plugin:dev-diag|debug_log`。
 */
export function debugLog(line: string): void {
  if (!DIAG) return;
  void invoke("plugin:dev-diag|debug_log", { line }).catch((err) =>
    console.error("诊断上报失败", err),
  );
}

/** 详情撤销/保存诊断。 */
export function undoLog(line: string): void {
  if (DIAG) debugLog(line);
}

/** 详情拖拽/选区诊断。 */
export function dragDiag(line: string): void {
  if (DIAG) debugLog(line);
}
