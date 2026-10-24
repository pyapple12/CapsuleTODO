import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import SettingsOverlay from "../src/components/SettingsOverlay.vue";

describe("SettingsOverlay 热键失败恢复（FIX015.26）", () => {
  let wrapper: VueWrapper | undefined;
  let resumeCalls = 0;

  beforeEach(async () => {
    resumeCalls = 0;
    invokeMock.mockReset();
    invokeMock.mockImplementation(async (cmd: string): Promise<unknown> => {
      if (cmd === "settings_get_bubble_hotkey") return "Ctrl+Alt+C";
      if (cmd === "settings_get_bubble_max_limit") return 20;
      if (cmd === "settings_set_bubble_hotkey") throw new Error("mock save fail");
      if (cmd === "settings_resume_bubble_hotkey") {
        resumeCalls += 1;
        return undefined;
      }
      return undefined;
    });
    wrapper = mount(SettingsOverlay, {
      props: {
        maxBubbles: 5,
        alwaysOnTop: false,
        snapToEdge: false,
        theme: 0,
      },
    });
    await flushPromises();
  });

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
    vi.restoreAllMocks();
  });

  it("收起态透明根不再把 mousedown 翻回开板（FIX017.11）", async () => {
    const overlay = wrapper!.find(".settings-overlay");
    expect(overlay.classes()).not.toContain("open");
    await overlay.trigger("mousedown");
    await flushPromises();
    expect(wrapper!.find(".settings-overlay").classes()).not.toContain("open");
  });

  it("保存/parse 早退失败时调用幂等 resume", async () => {
    await wrapper!.find(".hotkey-btn").trigger("click");
    await flushPromises();

    document.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "b",
        ctrlKey: true,
        altKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    await flushPromises();

    expect(resumeCalls).toBe(1);
  });
});
