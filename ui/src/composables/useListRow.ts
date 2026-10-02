// ===== 列表行通用小工具（FIX007.9 收敛 FIX005.24 遗漏面）：rowEl 定位 /
// pinLeaveHeight 退场钉高。TodoList/BubblesView/ArchiveOverlay 三列表组件共用
// （180ms 单双击消歧维持两组件本地实现——FIX007.9 裁量在案：迁移牵动
// installDragHooks cancelPendingClick 签名收益低风险高；曾建的共享件零引用
// 已随 FIX008.6 删除）

/**
 * 按条目 id 定位行元素（罩死判定/视觉直改的取元素入口）
 * @param pageSelector 页级行选择器前缀（如 "#page-todos"/"#page-bubbles"）
 * @param id 条目 id
 * @returns 行元素；行不在 DOM（塌缩离场中）返回脱管 div（罩死判定视为不罩死，
 * 视觉直改 no-op——调用方语义天然安全）
 */
export function rowElById(pageSelector: string, id: number): HTMLElement {
  return (
    (document.querySelector(`${pageSelector} [data-row-id="${id}"]`) as HTMLElement | null) ??
    document.createElement("div")
  );
}

/**
 * 退场钉高（design collapseRow 第一步 1:1）：height auto→0 不可过渡，leave 前
 * 钉实测高度作过渡起点，缺失即"行直接消失"。boxSizing 一并钉 border-box
 *（气泡行有 padding，不钉则塌缩终值偏大）
 * @param el TransitionGroup leave 钩子收到的行元素
 */
export function pinLeaveHeight(el: Element): void {
  const h = el as HTMLElement;
  h.style.boxSizing = "border-box";
  h.style.height = `${h.offsetHeight}px`;
}
