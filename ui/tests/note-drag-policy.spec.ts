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
  /** 卡片下边界（钳制可达范围用；缺省给足够大，不影响普通用例） */
  cardBottom?: number;
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
    cardRect: { top: 0, bottom: opts.cardBottom ?? 10000, left: 0, right: 1000 } as DOMRect,
    // 引擎钳制边界（eff 空间；cardTop=0 + DRAG_EDGE_MARGIN=12）
    centerMin: 12 + opts.selfHeight / 2,
    centerMax: (opts.cardBottom ?? 10000) - 12 - opts.selfHeight / 2,
    scrollDelta: 0,
  } as unknown as DragCtx;
}

describe("noteSwapRule（`.58` 缘对中线 + `.80` 等比阈值/可达钳制）", () => {
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

  /**
   * PATCH002.80 比例阈值（k=0.25）：200px 图对 20px 文字，d = 0.25×(200-20) = 45px；
   * 触发缘 = 图片下缘 - 45 ⇒ 中心 161 时换（`.58` 是 116、中心规则是 216+）。
   */
  it("高块对矮行：等比阈值 k=0.25 触发", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 100, height: 200 }, // 被拖图片（fromIdx = 0）
        { mid: 216, height: 20 }, // 目标文字：中线 216
      ],
      fromIdx: 0,
      selfTop: 0,
      selfHeight: 200,
    });
    // 中心 160（下缘 260 - 45 = 215 < 216）⇒ 不换；中心 161（216）⇒ 换
    expect(noteSwapRule(ctx, 160)).toBe(0);
    expect(noteSwapRule(ctx, 161)).toBe(1);
  });

  /**
   * PATCH002.80 + 上拖修复：高块上拖用 `+d`（多拖 d 才越）。
   * 文字 20（mid 80）、图 200（fromIdx=1，top 100）：d=45。
   * 上越文字：80 < effTop + 45 ⇒ effTop > 35 ⇒ 中心 136 才越；135 不越（钳制 112 之内，可达）。
   */
  it("高块上拖：+d 触发（钳制范围内）", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 80, height: 20 }, // 上方文字
        { mid: 200, height: 200 }, // 被拖图片（fromIdx = 1，top 100）
      ],
      fromIdx: 1,
      selfTop: 100,
      selfHeight: 200,
    });
    expect(noteSwapRule(ctx, 136)).toBe(1); // effTop=36；80 < 81
    expect(noteSwapRule(ctx, 135)).toBe(0); // effTop=35；80 < 80 为假
  });

  /**
   * 高块上拖：`+d` 阈值落到顶部钳制之外时，由"到顶即到首"兜住。
   * 文字 mid 50、图 top 100：+d 要求 effTop ≤ 5（中心 105）< 钳制 112 ⇒ 不可达；
   * 顶部覆盖在中心 112 生效。
   */
  it("高块上拖：阈值超出顶部钳制时到顶即到首", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 50, height: 20 }, // 上方文字（更靠近顶部）
        { mid: 200, height: 200 }, // 被拖图片（fromIdx = 1，top 100）
      ],
      fromIdx: 1,
      selfTop: 100,
      selfHeight: 200,
    });
    expect(noteSwapRule(ctx, 113)).toBe(1); // effTop=13；50 < 13+45=58
    expect(noteSwapRule(ctx, 112)).toBe(0); // effTop=12 ⇒ 顶部覆盖
  });

  /**
   * PATCH002.80 可达性：引擎把被拖块中心钳在卡片内，高块的最大中心比末尾行中线高；
   * 目标中线必须被钳进可达范围，否则拖到底也换不到最后一行（live 回归）。
   */
  it("末尾行：目标中线钳进可达中心（拖到底仍能换到最后一行）", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 100, height: 200 }, // 图片（fromIdx = 0）
        { mid: 350, height: 20 }, // 最后一行：中线 350 > maxCenter 288
      ],
      fromIdx: 0,
      selfTop: 0,
      selfHeight: 200,
      cardBottom: 400,
    });
    // maxCenter = 400 - 12 - 100 = 288；clamp(350) = 288 ⇒ 中心 287 不换、288 换
    expect(noteSwapRule(ctx, 287)).toBe(0);
    expect(noteSwapRule(ctx, 288)).toBe(1);
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
