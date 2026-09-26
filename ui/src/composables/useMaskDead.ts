// ===== 罩死判定组合式（PL009.3 自 design/assets/js/drag-reorder.js 罩死段 1:1）：
// 行侵入溶解带 ≥38% 整卡禁交互（点/双击/拖拽/删除/复制全失效 + 灰化）；迟滞 36%
// 防阈值附近反复穿越闪烁；at-bottom 例外不计底带（V0.027 修复：短行贴底 14/35≈40%
// 误判灰死而视觉并无溶解带）。数值零改动 =====
import { registerMaskDeadHook } from "./useGlassBar";

/** 罩死阈值（用户定案 2026-09-26 微调）：侵入超卡高 38% 即死 */
export const MASK_FADE_RATIO = 0.38;
/** 出死区迟滞（用户定案）：回到 36% 才摘罩死类 */
export const MASK_FADE_HYST = 0.36;
/** 顶部溶解带高（glass-bar.css 遮罩顶段同源） */
export const MASK_BAND_TOP = 6;
/** 底部溶解带高（--fade-btm calc(100%-14px) 同源） */
export const MASK_BAND_BTM = 14;

/** 罩死核算目标（清单 + 气泡；归档板无罩死语义——用户定案） */
interface MaskDeadTarget {
  listId: string;
  rowSel: string;
  itemSel: string;
}

const MASK_DEAD_TARGETS: MaskDeadTarget[] = [
  { listId: "todo-active", rowSel: ".todo-row", itemSel: ".todo-item" },
  { listId: "bubble-list", rowSel: ".bubble-row", itemSel: ".bubble-row" },
];

/**
 * 行侵入溶解带比例（0 起）：行矩形与容器上下溶解带的侵入深度 ÷ 行高。
 * 列表取行所在 .board-read；at-bottom 时底带已抬出可视区不计（V0.027 修复）
 * @param row 行元素（.todo-row / .bubble-row）
 * @returns 侵入比 0~1+
 */
export function rowInvadeRatio(row: HTMLElement): number {
  const list =
    (row.closest(".board-read") as HTMLElement | null) ??
    (document.getElementById("todo-active") as HTMLElement | null);
  if (!list) return 0;
  const rr = row.getBoundingClientRect();
  const lr = list.getBoundingClientRect();
  const topDead = lr.top + MASK_BAND_TOP; // 顶带下缘：侵入此线以上的部分被罩
  // 到底抬带：底缘溶解带整体抬出可视区，视觉上已无底带，侵入比不再计底带
  const btmDead = list.classList.contains("at-bottom")
    ? Number.POSITIVE_INFINITY
    : lr.bottom - MASK_BAND_BTM;
  const h = rr.height;
  if (!h) return 0;
  return (Math.max(0, topDead - rr.top) + Math.max(0, rr.bottom - btmDead)) / h;
}

/**
 * 行是否被罩死：mousedown/click/dblclick 交互入口统一前置此判定
 * @param row 行元素
 */
export function rowMaskDead(row: HTMLElement): boolean {
  return rowInvadeRatio(row) >= MASK_FADE_RATIO;
}

/** 罩死类同步：双列表逐行翻转 .mask-dead（灰化+光标由 CSS 按类过渡）；整页隐藏跳过 */
export function syncMaskDead(): void {
  for (const t of MASK_DEAD_TARGETS) {
    const list = document.getElementById(t.listId);
    if (!list || list.hidden || list.closest("[hidden]")) continue;
    for (const row of list.querySelectorAll<HTMLElement>(t.rowSel)) {
      const ratio = rowInvadeRatio(row);
      const li = row.closest(t.itemSel) as HTMLElement | null;
      if (!li) continue;
      const isDead = li.classList.contains("mask-dead");
      if (!isDead && ratio >= MASK_FADE_RATIO) li.classList.add("mask-dead");
      else if (isDead && ratio < MASK_FADE_HYST) li.classList.remove("mask-dead");
    }
  }
}

// 注册进 useGlassBar 的滚动/内容重建挂点（与滑杆同频——design 松耦合同款）
registerMaskDeadHook(syncMaskDead);
