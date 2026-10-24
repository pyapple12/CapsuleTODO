import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { nextTick } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock, startDraggingMock, isFocusedMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
  startDraggingMock: vi.fn(),
  isFocusedMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    label: "main",
    isFocused: isFocusedMock,
    onCloseRequested: vi.fn().mockResolvedValue(() => {}),
    startDragging: startDraggingMock,
  }),
}));

vi.mock("../src/composables/titleParticles", () => ({
  initTitleParticles: () => ({ refresh: vi.fn(), destroy: vi.fn() }),
}));

// App 级测试只关心主卡类绑定；子页面各自有独立测试，避免拖入 useBoardRead 异步链。
vi.mock("../src/components/TodoList.vue", () => ({ default: { template: "<div />" } }));
vi.mock("../src/components/BubblesView.vue", () => ({ default: { template: "<div />" } }));
vi.mock("../src/components/WhiteboardView.vue", () => ({ default: { template: "<div />" } }));
vi.mock("../src/components/ArchiveOverlay.vue", () => ({ default: { template: "<div />" } }));
vi.mock("../src/components/SettingsOverlay.vue", () => ({ default: { template: "<div />" } }));
vi.mock("../src/components/DetailOverlay.vue", () => ({ default: { template: "<div />" } }));
vi.mock("../src/components/AddBar.vue", () => ({ default: { template: "<div />" } }));
vi.mock("../src/components/TabsBar.vue", () => ({ default: { template: "<div />" } }));

import App from "../App.vue";

describe("App 拖拽期聚焦态（FIX015.34）", () => {
  let wrapper: VueWrapper | undefined;

  beforeEach(async () => {
    invokeMock.mockReset();
    listenMock.mockReset();
    startDraggingMock.mockReset();
    startDraggingMock.mockResolvedValue(undefined);
    isFocusedMock.mockReset();
    isFocusedMock.mockResolvedValue(false);
    listenMock.mockResolvedValue(() => {});
    invokeMock.mockImplementation(async (cmd: string): Promise<unknown> => {
      if (cmd === "todo_list" || cmd === "todo_archive_list") return [];
      if (cmd === "bubble_list") return { items: [] };
      if (cmd === "whiteboard_load") return "";
      if (cmd === "settings_get_max_bubbles" || cmd === "settings_get_bubble_max_limit") {
        return 5;
      }
      if (cmd === "settings_get_bubble_hotkey") return "Ctrl+Alt+C";
      if (cmd === "settings_get_always_on_top" || cmd === "settings_get_snap_to_edge") {
        return false;
      }
      if (cmd === "settings_get_theme") return 0;
      return undefined;
    });
    wrapper = mount(App);
    await flushPromises();
    await nextTick();
  });

  afterEach(async () => {
    await flushPromises();
    await nextTick();
    wrapper?.unmount();
    wrapper = undefined;
    vi.restoreAllMocks();
  });

  it("窗口失焦但拖拽中时主卡保持 focused", async () => {
    expect(wrapper!.find(".glass-card").classes()).not.toContain("focused");

    const title = wrapper!.find(".title");
    await title.trigger("mousedown", { button: 0, clientX: 10, clientY: 10 });
    window.dispatchEvent(
      new MouseEvent("mousemove", { buttons: 1, clientX: 10, clientY: 30, bubbles: true }),
    );
    await flushPromises();

    expect(startDraggingMock).toHaveBeenCalled();
    expect(wrapper!.find(".glass-card").classes()).toContain("focused");
  });

  it("startDragging 失败后清掉 windowDragging 聚焦态", async () => {
    // FIX017.11（A017 P3-12）：失败回调必须立即回清，不能等 10s 安全阀。
    startDraggingMock.mockRejectedValueOnce(new Error("denied"));
    const title = wrapper!.find(".title");
    await title.trigger("mousedown", { button: 0, clientX: 10, clientY: 10 });
    window.dispatchEvent(
      new MouseEvent("mousemove", { buttons: 1, clientX: 10, clientY: 30, bubbles: true }),
    );
    await flushPromises();
    await nextTick();

    expect(startDraggingMock).toHaveBeenCalled();
    expect(wrapper!.find(".glass-card").classes()).not.toContain("focused");
  });
});
