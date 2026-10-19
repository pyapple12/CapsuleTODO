import { describe, expect, it } from "vitest";

import type { DragCtx } from "../src/composables/useDragReorder";
import { noteSwapRule } from "../src/composables/noteDragPolicy";

/**
 * 造一个只含 `noteSwapRule` 所需字段的上下文（其余字段对该策略无意义 ⇒ 断言隔离，
 * 不把机制内部结构绑进用例）。
 * @param rows 各行中线 y（高度由 `height` 统一给定）
 */
function makeCtx(opts: {
  rows: { mid: number; height: number }[];
  fromIdx: number;
  selfTop: number;
  selfHeight: number;
}): DragCtx {
  return {
    rows: opts.rows.map((r) => ({
      el: {} as HTMLElement,
      top: r.mid - r.height / 2,
      mid: r.mid,
      height: r.height,
    })),
    fromIdx: opts.fromIdx,
    selfTop: opts.selfTop,
    selfHeight: opts.selfHeight,
  } as unknown as DragCtx;
}

describe("noteSwapRule（PATCH002.58：越目标行中线即换位）", () => {
  /** 等高行向下拖：被拖块**下缘**越过下一行中线 ⇒ 换位（越过一行）。 */
  it("向下拖：下缘越中线即越过一行", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 20, height: 40 },
        { mid: 60, height: 40 },
        { mid: 100, height: 40 },
      ],
      fromIdx: 0,
      selfTop: 0,
      selfHeight: 40,
    });
    // 被拖块中心 50 ⇒ 下缘 70 > 目标行中线 60 ⇒ 已越过
    expect(noteSwapRule(ctx, 50)).toBe(1);
  });

  /** 未越中线：停在原槽位（**不提交**，机制侧据此判"没换位"）。 */
  it("向下拖：未越中线 ⇒ 原槽位", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 20, height: 40 },
        { mid: 60, height: 40 },
        { mid: 100, height: 40 },
      ],
      fromIdx: 0,
      selfTop: 0,
      selfHeight: 40,
    });
    // 被拖块中心 35 ⇒ 下缘 55 < 60 ⇒ 未越
    expect(noteSwapRule(ctx, 35)).toBe(0);
  });

  /** 向上拖：被拖块**上缘**越过上方行中线 ⇒ 越过一行。 */
  it("向上拖：上缘越中线即越过一行", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 20, height: 40 },
        { mid: 60, height: 40 },
        { mid: 100, height: 40 },
      ],
      fromIdx: 2,
      selfTop: 80,
      selfHeight: 40,
    });
    // 被拖块中心 50 ⇒ 上缘 30 > 行 0 中线 20 ⇒ 越过行 0
    expect(noteSwapRule(ctx, 50)).toBe(1);
  });

  /**
   * **放宽的核心用例（长目标行）**：目标行高 400（中线 220、下缘 420）时，
   * 被拖块下缘只需到 300 即换位；`.55` 的"缘对缘"规则要 420 才换 ⇒ 本规则更宽、
   * 长文本块不再需要拖一整行。
   */
  it("长目标行：下缘仅越中线（未及目标行下缘）也换位", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 20, height: 40 }, // 被拖块（fromIdx = 0）
        { mid: 220, height: 400 }, // 长目标行：中线 220、下缘 420
      ],
      fromIdx: 0,
      selfTop: 0,
      selfHeight: 40,
    });
    // 被拖块中心 280 ⇒ 下缘 300：> 中线 220（本规则 ✓）、< 下缘 420（旧规则 ✗）
    expect(noteSwapRule(ctx, 280)).toBe(1);
  });

  /** 起拖块自身不计入（避免"自己越过自己"）。 */
  it("起拖块自身不参与计数", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 20, height: 40 },
        { mid: 60, height: 40 },
      ],
      fromIdx: 0,
      selfTop: 0,
      selfHeight: 40,
    });
    expect(noteSwapRule(ctx, 0)).toBe(0);
  });
});
