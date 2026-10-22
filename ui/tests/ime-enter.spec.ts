import { describe, expect, it } from "vitest";

import { shouldIgnoreImeEnter } from "../src/composables/imeEnter";

describe("shouldIgnoreImeEnter（FIX014.30）", () => {
  it("组字中的 Enter 必须忽略", () => {
    expect(shouldIgnoreImeEnter({ isComposing: true, keyCode: 13 })).toBe(true);
  });

  it("兼容 keyCode 229 的输入法事件", () => {
    expect(shouldIgnoreImeEnter({ isComposing: false, keyCode: 229 })).toBe(true);
  });

  it("普通 Enter 不被误拦", () => {
    expect(shouldIgnoreImeEnter({ isComposing: false, keyCode: 13 })).toBe(false);
  });
});
