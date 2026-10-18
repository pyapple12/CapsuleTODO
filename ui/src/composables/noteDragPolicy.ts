// ===== 详情块拖拽的几何策略（PATCH002.55）：机制 / 策略分离 =====
// 机制（useDragReorder）保持通用、继续 1:1 复刻 design；详情块的几何前提与等高列表不同
// （图块高度可远大于文本块），故"换位阈值"由详情自己提供——机制只在 target 配了
// swapRule 时调用它，缺省仍走原"中心对中心"公式（清单 / 气泡零影响）。
import type { DragCtx } from "./useDragReorder";

/**
 * 详情块换位判定（PATCH002.58 用户定案"丙"：**被拖块的上 / 下缘越过目标行中线即换位**）：
 * 向上拖看**上缘**、向下拖看**下缘**；方向以起拖上缘 `selfTop` 判定。
 * **为什么放宽**（`.55` 的"缘对缘"实测手感问题）：目标行是**长文本块**时，"上缘越上缘"要拖
 * 一整行那么远 → 手快/拖得短就"没换位、松手回原位"，像随机失效 ✗；改看**中线**后，越过程度
 * 与"半个行高"挂钩，长短行手感一致 ✓。清单 / 气泡**不受影响**（各自 target 未配 `swapRule` ✓）。
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
    if (downward ? r.mid < effBottom : r.mid < effTop) to += 1;
  });
  return to;
}
