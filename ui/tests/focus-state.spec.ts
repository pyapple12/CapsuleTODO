import { describe, expect, it } from "vitest";

import { isEffectivelyFocused } from "../src/composables/focusState";

describe("有效聚焦态（FIX015.34）", () => {
  it("拖拽期间即使系统报告失焦也保持聚焦", () => {
    expect(isEffectivelyFocused(false, true)).toBe(true);
    expect(isEffectivelyFocused(true, false)).toBe(true);
    expect(isEffectivelyFocused(false, false)).toBe(false);
  });
});
