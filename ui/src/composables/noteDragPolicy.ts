// ===== 详情块拖拽的几何策略（PATCH002.55）：机制 / 策略分离 =====
// 机制（useDragReorder）保持通用、继续 1:1 复刻 design；详情块的几何前提与等高列表不同
// （图块高度可远大于文本块），故"换位阈值"由详情自己提供——机制只在 target 配了
// swapRule 时调用它，缺省仍走原"中心对中心"公式（清单 / 气泡零影响）。
import type { DragCtx } from "./useDragReorder";

/**
 * 详情块换位判定（用户定案"按方向取缘"）：向上拖时，被拖块**上缘越过**目标行**上缘**即算越过；
 * 向下拖时，**下缘越过**目标行**下缘**才算。方向以起拖上缘 `selfTop` 判定。
 * 等高行时与机制缺省公式（中心对中心）数学等价——三缘同步，故等高列表不受影响。
 * @param ctx 拖拽上下文（含行快照 rows / 起拖槽位 fromIdx / 起拖上缘 selfTop / 行高 selfHeight）
 * @param eff 被拖块中心的当前等效位置（已补回自动滚动量）
 * @returns 目标槽位号（= 在其余行中的插入下标）
 */
export function noteSwapRule(ctx: DragCtx, eff: number): number {
  const effTop = eff - ctx.selfHeight / 2;
  const effBottom = eff + ctx.selfHeight / 2;
  const downward = effTop > ctx.selfTop; // 以起拖上缘判方向
  let to = 0;
  ctx.rows.forEach((r, i) => {
    if (i === ctx.fromIdx) return;
    if (downward ? r.top + r.height < effBottom : r.top < effTop) to += 1;
  });
  return to;
}
