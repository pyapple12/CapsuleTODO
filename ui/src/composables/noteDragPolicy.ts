// ===== 详情块拖拽的几何策略（PATCH002.55）：机制 / 策略分离 =====
// 机制（useDragReorder）保持通用、继续 1:1 复刻 design；详情块的几何前提与等高列表不同
// （图块高度可远大于文本块），故"换位阈值"由详情自己提供——机制只在 target 配了
// swapRule 时调用它，缺省仍走原"中心对中心"公式（清单 / 气泡零影响）。
import type { DragCtx } from "./useDragReorder";

/** 高度差的比例阈值（PATCH002.80 最终定案 k=0.25）：k=0 回 `.58` 缘对中线、k=0.5 为中心。
 * PATCH002.87：清单/气泡默认换位也复用同一阈值（等高 d=0，逐帧退回中心对中心）。 */
export const HEIGHT_SWAP_K = 0.25;

/**
 * 详情块换位判定（PATCH002.58 定案"丙" + PATCH002.80 等比阈值）：
 * **向下看被拖块下缘、向上看被拖块上缘**，越目标行中线即换位；方向以起拖上缘 `selfTop` 判。
 *
 * `.58` 的缘对中线让长文本行不必拖一整行；但对**显著更高的图块**，触发距离恒为
 * `行距 + 目标行高/2`（≈16px），而让位位移 = 被拖块高 + 行距（≈206px）⇒ 轻拖即整块跳（1:13）。
 * `.80` 最终改为**连续等比**：`d = k × max(0, 被拖块高 - 目标行高)`，把判定缘往里收 d
 * （k = 0.25）——等高行 d=0 完全退回 `.58`；200px 图对 20px 文字触发距离 ≈61px。
 *
 * **上拖符号**：向上是"行中线 < 上缘 + d"（`+d` 才是"多拖 d 才越"；`-d` 会让高图静止时
 * 上方行就不计数 ⇒ 一拖即跳）。
 * **顶行可达**：到达顶部钳制位时（`eff ≤ centerMin`，由引擎算入 ctx）直接返回 0——
 * 拖到顶即"到首行之前"；该覆盖对所有行成立（d=0 时本就不计数，空操作）。
 * **末行可达钳制**：引擎把被拖块中心钳在卡片内（`centerMax`），高图的最大下缘到不了
 * 末尾行中线；阈值须钳到 `centerMax + selfHeight/2`，拖到卡片底即可换到最后一行。
 * @param ctx 拖拽上下文（含行快照 rows / 起拖槽位 fromIdx / 起拖上缘 selfTop / 行高 selfHeight）
 * @param eff 被拖块中心的当前等效位置（已补回自动滚动量）
 * @returns 目标槽位号（= 在其余行中的插入下标）
 */
export function noteSwapRule(ctx: DragCtx, eff: number): number {
  const effTop = eff - ctx.selfHeight / 2;
  const effBottom = eff + ctx.selfHeight / 2;
  const downward = effTop > ctx.selfTop; // 以起拖上缘判方向
  // 顶行可达：到顶部钳制位即"到首"（对所有行成立；d=0 时本就是 0）
  const atTop = eff <= ctx.centerMin + 0.5;
  if (!downward && atTop) return 0;
  // 末行可达钳制（仅向下用）：被拖块下缘最多到卡片底 - 引擎边距
  const maxEdge = downward ? ctx.centerMax + ctx.selfHeight / 2 : 0;
  let to = 0;
  ctx.rows.forEach((r, i) => {
    if (i === ctx.fromIdx) return;
    // d = 等比阈值：等高行 0（退回 `.58`）；高图对矮行把触发缘往里收，避免 16px→206px 的跳变
    const d = HEIGHT_SWAP_K * Math.max(0, ctx.selfHeight - r.height);
    if (downward) {
      const edge = effBottom - d;
      if (edge >= Math.min(r.mid, maxEdge - d)) to += 1;
    } else if (r.mid < effTop + d) {
      to += 1;
    }
  });
  return to;
}
