import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock, deferDuringDragMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
  deferDuringDragMock: vi.fn(() => false),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("../src/composables/useDragReorder", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/composables/useDragReorder")>();
  return { ...actual, deferDuringDrag: deferDuringDragMock };
});

import BubblesView from "../src/components/BubblesView.vue";

describe("BubblesView 捕获在飞锁（FIX017.11）", () => {
  let wrapper: VueWrapper | undefined;
  let bubbleChanged: (() => void) | undefined;

  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
    deferDuringDragMock.mockReset();
    deferDuringDragMock.mockReturnValue(false);
    bubbleChanged = undefined;
    listenMock.mockImplementation(async (event: string, cb: () => void) => {
      if (event === "bubble-changed") bubbleChanged = cb;
      return () => {};
    });
    const board = document.createElement("div");
    board.id = "board";
    document.body.append(board);
  });

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
    document.body.innerHTML = "";
    vi.restoreAllMocks();
  });

  it("捕获在飞期间重复点击只发一次 bubble_capture", async () => {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "bubble_list") return { items: [] };
      if (cmd === "bubble_capture") {
        await gate;
        return { status: "captured" };
      }
      return undefined;
    });

    wrapper = mount(BubblesView, { props: { maxBubbles: 5 } });
    await flushPromises();

    const button = wrapper.find<HTMLButtonElement>("button.capture");
    button.element.click();
    button.element.click();
    await flushPromises();

    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === "bubble_capture")).toHaveLength(1);

    release();
    await flushPromises();
  });

  it("拖拽中 bubble-changed 不替换 items，收场后恢复刷新", async () => {
    let text = "旧气泡";
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "bubble_list") {
        return { items: [{ id: 1, text, kind: "Text" }] };
      }
      return undefined;
    });
    wrapper = mount(BubblesView, { props: { maxBubbles: 5 } });
    await flushPromises();
    expect(wrapper.find(".b-text").text()).toBe("旧气泡");

    deferDuringDragMock.mockReturnValue(true);
    text = "拖拽中到达";
    bubbleChanged?.();
    await flushPromises();
    expect(wrapper.find(".b-text").text()).toBe("旧气泡");

    deferDuringDragMock.mockReturnValue(false);
    bubbleChanged?.();
    await flushPromises();
    expect(wrapper.find(".b-text").text()).toBe("拖拽中到达");
  });

  it("拖拽前已在飞的旧响应不能绕过冻结", async () => {
    let releaseOld!: (value: { items: Array<{ id: number; text: string; kind: string }> }) => void;
    const oldGate = new Promise<{ items: Array<{ id: number; text: string; kind: string }> }>(
      (resolve) => {
        releaseOld = resolve;
      },
    );
    let listCalls = 0;
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "bubble_list") {
        listCalls += 1;
        if (listCalls === 1) return oldGate;
        return { items: [{ id: 2, text: "新气泡", kind: "Text" }] };
      }
      return undefined;
    });
    wrapper = mount(BubblesView, { props: { maxBubbles: 5 } });
    await flushPromises();

    deferDuringDragMock.mockReturnValue(true);
    bubbleChanged?.();
    await flushPromises();
    releaseOld({ items: [{ id: 1, text: "旧气泡", kind: "Text" }] });
    await flushPromises();
    expect(wrapper.findAll(".b-text")).toHaveLength(0);

    deferDuringDragMock.mockReturnValue(false);
    bubbleChanged?.();
    await flushPromises();
    expect(wrapper.find(".b-text").text()).toBe("新气泡");
  });
});
