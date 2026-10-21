import { describe, expect, it } from "vitest";

import type { DragCtx } from "../src/composables/useDragReorder";
import {
  clampListScrollTop,
  defaultSwapRule,
  reachableTargetMid,
} from "../src/composables/useDragReorder";

/**
 * 造一个只含默认换位规则所需字段的上下文（其余字段对该规则无意义 ⇒ 断言隔离）。
 * @param opts 行中线/行高、起拖槽位、被拖行上缘/高度、卡片边界、滚动补偿
 */
function makeCtx(opts: {
  rows: { mid: number; height: number }[];
  fromIdx: number;
  selfTop: number;
  selfHeight: number;
  cardTop?: number;
  cardBottom?: number;
  scrollDelta?: number;
}): DragCtx {
  const cardTop = opts.cardTop ?? 0;
  const cardBottom = opts.cardBottom ?? 10000;
  const scrollDelta = opts.scrollDelta ?? 0;
  return {
    rows: opts.rows.map((r) => ({
      el: {} as HTMLElement,
      top: r.mid - r.height / 2,
      mid: r.mid,
      height: r.height,
    })),
    fromIdx: opts.fromIdx,
    selfTop: opts.selfTop,
    selfMid: opts.selfTop + opts.selfHeight / 2,
    selfHeight: opts.selfHeight,
    cardRect: { top: cardTop, bottom: cardBottom, left: 0, right: 1000 } as DOMRect,
    centerMin: cardTop + 12 + opts.selfHeight / 2 + scrollDelta,
    centerMax: cardBottom - 12 - opts.selfHeight / 2 + scrollDelta,
    scrollDelta,
  } as unknown as DragCtx;
}

describe("reachableTargetMid（PATCH002.87 可达中点钳制）", () => {
  it("下拖只钳上界；上拖不钳（顶部由 atTop 归 0）", () => {
    const ctx = makeCtx({
      rows: [],
      fromIdx: 0,
      selfTop: 0,
      selfHeight: 100,
      cardBottom: 400,
    });
    // centerMin = 12 + 50 = 62；centerMax = 400 - 12 - 50 = 338
    expect(reachableTargetMid({ mid: 50 }, ctx, true)).toBe(50);
    expect(reachableTargetMid({ mid: 350 }, ctx, true)).toBe(338);
    expect(reachableTargetMid({ mid: 200 }, ctx, true)).toBe(200);
    expect(reachableTargetMid({ mid: 50 }, ctx, false)).toBe(50);
    expect(reachableTargetMid({ mid: 350 }, ctx, false)).toBe(350);
  });
});

describe("clampListScrollTop（PATCH002.89 收场视口恢复）", () => {
  it("正常范围原样保留", () => {
    expect(clampListScrollTop(500, 1000, 300)).toBe(500);
  });

  it("超过可滚范围时夹到底", () => {
    expect(clampListScrollTop(900, 1000, 300)).toBe(700);
  });

  it("负数夹到 0；内容不足一屏也回 0", () => {
    expect(clampListScrollTop(-5, 1000, 300)).toBe(0);
    expect(clampListScrollTop(120, 200, 300)).toBe(0);
  });
});

describe("defaultSwapRule（PATCH002.87 清单/气泡默认换位）", () => {
  it("等高行下拖：严格越中心才换，d=0 手感不变", () => {
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
    expect(defaultSwapRule(ctx, 60)).toBe(0); // 60 < 60 为假（原 `<` 语义）
    expect(defaultSwapRule(ctx, 60.5)).toBe(1); // 60 < 60.5 ⇒ 越过一行
  });

  it("等高行上拖：未到中心不换，d=0 手感不变", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 60, height: 40 },
        { mid: 100, height: 40 },
      ],
      fromIdx: 1,
      selfTop: 80,
      selfHeight: 40,
    });
    expect(defaultSwapRule(ctx, 60)).toBe(0); // 60 < 60 为假
    expect(defaultSwapRule(ctx, 60.5)).toBe(1); // 60 < 60.5 ⇒ 越过一行
  });

  it("末行很矮：目标中点钳到 centerMax，拖到底能换", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 100, height: 200 }, // 被拖图片（fromIdx = 0）
        { mid: 350, height: 20 }, // 末行：350 > centerMax 288
      ],
      fromIdx: 0,
      selfTop: 0,
      selfHeight: 200,
      cardBottom: 400,
    });
    // centerMax = 400 - 12 - 100 = 288；d = 0.25 × (200 - 20) = 45
    expect(defaultSwapRule(ctx, 242.9)).toBe(0); // 288 < 287.9 为假
    expect(defaultSwapRule(ctx, 243)).toBe(0); // 288 < 288 为假
    expect(defaultSwapRule(ctx, 243.1)).toBe(1); // 288 < 288.1 ⇒ 换到最后一行
    expect(defaultSwapRule(ctx, 288)).toBe(1); // 钳制边界仍可达
  });

  it("等高末行恰好落在 centerMax：边界补一次触发", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 100, height: 40 },
        { mid: 380, height: 40 }, // centerMax = 400 - 12 - 20 = 368；末行中点被钳到 368
      ],
      fromIdx: 0,
      selfTop: 0,
      selfHeight: 40,
      cardBottom: 400,
    });
    expect(defaultSwapRule(ctx, 367.9)).toBe(0); // 368 < 367.9 为假，边界未到
    expect(defaultSwapRule(ctx, 368)).toBe(1); // 边界条件补一次
  });

  it("拖到顶部：首行中点够不到时也归 0", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 50, height: 20 }, // 首行
        { mid: 200, height: 200 }, // 被拖图片（fromIdx = 1，top 100）
      ],
      fromIdx: 1,
      selfTop: 100,
      selfHeight: 200,
      cardBottom: 400,
    });
    // centerMin = 12 + 100 = 112
    expect(defaultSwapRule(ctx, 112)).toBe(0); // 到顶即到首
    expect(defaultSwapRule(ctx, 113)).toBe(1); // 越过首行
  });

  it("高行上拖矮行：用 eff - d，多拖 d 才越", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 80, height: 20 }, // 上方矮行
        { mid: 300, height: 200 }, // 被拖高块（fromIdx = 1，top 200）
      ],
      fromIdx: 1,
      selfTop: 200,
      selfHeight: 200,
    });
    // d = 0.25 × (200 - 20) = 45；上拖阈值 eff - 45 ⇒ 80 < eff - 45 ⇒ eff > 125
    expect(defaultSwapRule(ctx, 125)).toBe(0); // 80 < 80 为假
    expect(defaultSwapRule(ctx, 126)).toBe(1); // 80 < 81
  });

  it("scrollDelta 进入 centerMax：滚到底的钳制仍生效", () => {
    const ctx = makeCtx({
      rows: [
        { mid: 100, height: 200 },
        { mid: 350, height: 20 },
      ],
      fromIdx: 0,
      selfTop: 0,
      selfHeight: 200,
      cardBottom: 400,
      scrollDelta: 10,
    });
    // centerMax = 400 - 12 - 100 + 10 = 298；d = 45
    expect(defaultSwapRule(ctx, 253)).toBe(0); // 298 < 298 为假
    expect(defaultSwapRule(ctx, 254)).toBe(1); // 298 < 299
  });
});
