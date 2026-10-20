import { effectScope } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useAutoHideError } from "../src/composables/useAutoHideError";

describe("useAutoHideError（PATCH002.66）", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("10s 到点自隐；重复 show 重新计时；hide 立即清", () => {
    const scope = effectScope();
    const api = scope.run(() => useAutoHideError(10_000));
    if (!api) throw new Error("scope 未返回");
    api.show("添加失败！");
    expect(api.message.value).toBe("添加失败！");
    vi.advanceTimersByTime(9_999);
    expect(api.message.value).toBe("添加失败！");
    vi.advanceTimersByTime(1);
    expect(api.message.value).toBe("");

    api.show("添加失败！");
    vi.advanceTimersByTime(5_000);
    api.show("捕获失败");
    vi.advanceTimersByTime(5_000);
    expect(api.message.value).toBe("捕获失败");
    api.hide();
    expect(api.message.value).toBe("");
    scope.stop();
  });

  it("scope 销毁时清计时器，不再写回消息", () => {
    const scope = effectScope();
    const api = scope.run(() => useAutoHideError(5_000));
    if (!api) throw new Error("scope 未返回");
    api.show("清空失败");
    scope.stop();
    vi.advanceTimersByTime(5_000);
    expect(api.message.value).toBe("清空失败");
  });
});
