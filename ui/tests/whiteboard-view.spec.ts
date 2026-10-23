import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import WhiteboardView from "../src/components/WhiteboardView.vue";

describe("WhiteboardView 保存期间输入（FIX015.27）", () => {
  let wrapper: VueWrapper | undefined;
  let releaseFirstSave!: () => void;
  const saved: string[] = [];

  beforeEach(async () => {
    const board = document.createElement("div");
    board.id = "board";
    document.body.append(board);
    vi.useFakeTimers();
    invokeMock.mockReset();
    saved.length = 0;
    invokeMock.mockImplementation(
      async (cmd: string, args?: Record<string, unknown>): Promise<unknown> => {
        if (cmd === "whiteboard_load") return "";
        if (cmd === "whiteboard_save") {
          saved.push(String(args?.content ?? ""));
          if (saved.length === 1) {
            await new Promise<void>((resolve) => {
              releaseFirstSave = resolve;
            });
          }
          return undefined;
        }
        return undefined;
      },
    );
    wrapper = mount(WhiteboardView);
    await flushPromises();
  });

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
    document.getElementById("board")?.remove();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("保存飞行中继续输入，成功后仍会再次保存新文本", async () => {
    const textarea = wrapper?.find<HTMLTextAreaElement>("textarea#wb-board");
    expect(textarea).toBeDefined();
    await textarea!.setValue("第一版");
    const firstFlush = (wrapper!.vm as unknown as { flush: () => Promise<boolean> }).flush();
    await flushPromises();
    expect(saved).toEqual(["第一版"]);

    await textarea!.setValue("第二版");
    releaseFirstSave();
    await firstFlush;
    await flushPromises();

    vi.advanceTimersByTime(800);
    await flushPromises();
    expect(saved).toEqual(["第一版", "第二版"]);
    expect(wrapper?.find("p.error").exists()).toBe(false);
  });
});
