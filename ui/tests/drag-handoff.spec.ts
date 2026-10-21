import { describe, expect, it } from "vitest";

import { expectedOrderMatches, sameIdOrder } from "../src/composables/dragHandoff";

describe("sameIdOrder（PATCH002.89 拖拽收场顺序）", () => {
  it("完全同序才通过", () => {
    expect(sameIdOrder([3, 1, 2], [3, 1, 2])).toBe(true);
  });

  it("同集合但顺序不同必须拒绝", () => {
    expect(sameIdOrder([1, 2, 3], [3, 1, 2])).toBe(false);
  });

  it("长度不同拒绝", () => {
    expect(sameIdOrder([1, 2], [1, 2, 3])).toBe(false);
  });
});

describe("expectedOrderMatches（PATCH002.89 交接门槛）", () => {
  it("无目标序（未换位 / 重排失败）直接放行", () => {
    expect(expectedOrderMatches([1, 2], null)).toBe(true);
    expect(expectedOrderMatches([1, 2], undefined)).toBe(true);
  });

  it("有目标序时必须命中", () => {
    expect(expectedOrderMatches([2, 1], [2, 1])).toBe(true);
    expect(expectedOrderMatches([1, 2], [2, 1])).toBe(false);
  });
});
