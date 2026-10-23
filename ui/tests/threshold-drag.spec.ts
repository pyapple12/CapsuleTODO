import { afterEach, describe, expect, it, vi } from "vitest";

const { startDraggingMock } = vi.hoisted(() => ({
  startDraggingMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ startDragging: startDraggingMock }),
}));

import { useThresholdDrag } from "../src/composables/useThresholdDrag";

describe("useThresholdDrag 收尾（FIX015.31）", () => {
  let handle: HTMLElement | undefined;
  let destroy: (() => void) | undefined;

  afterEach(() => {
    destroy?.();
    destroy = undefined;
    handle?.remove();
    handle = undefined;
    startDraggingMock.mockReset();
    vi.restoreAllMocks();
  });

  it("失焦后不再把后续无按键移动当成拖窗", () => {
    handle = document.createElement("div");
    document.body.append(handle);
    const onDragStart = vi.fn();
    destroy = useThresholdDrag(handle, { onDragStart });

    handle.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, button: 0, clientX: 0, clientY: 0 }),
    );
    window.dispatchEvent(new Event("blur"));
    window.dispatchEvent(
      new MouseEvent("mousemove", { bubbles: true, buttons: 1, clientX: 100, clientY: 0 }),
    );

    expect(startDraggingMock).not.toHaveBeenCalled();
    expect(onDragStart).not.toHaveBeenCalled();
  });

  it("无按键 move 立即收尾", () => {
    handle = document.createElement("div");
    document.body.append(handle);
    destroy = useThresholdDrag(handle);

    handle.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, button: 0, clientX: 0, clientY: 0 }),
    );
    window.dispatchEvent(
      new MouseEvent("mousemove", { bubbles: true, buttons: 0, clientX: 100, clientY: 0 }),
    );
    window.dispatchEvent(
      new MouseEvent("mousemove", { bubbles: true, buttons: 1, clientX: 100, clientY: 0 }),
    );

    expect(startDraggingMock).not.toHaveBeenCalled();
  });

  it("边缘起手离开 handle 后仍能启动拖窗", async () => {
    startDraggingMock.mockResolvedValue(undefined);
    handle = document.createElement("div");
    document.body.append(handle);
    destroy = useThresholdDrag(handle);

    handle.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, button: 0, clientX: 0, clientY: 0 }),
    );
    handle.dispatchEvent(new MouseEvent("mouseleave", { bubbles: true }));
    window.dispatchEvent(
      new MouseEvent("mousemove", { bubbles: true, buttons: 1, clientX: 10, clientY: 0 }),
    );
    await Promise.resolve();

    expect(startDraggingMock).toHaveBeenCalledTimes(1);
  });

  it("startDragging 失败时回调清拖拽态", async () => {
    const error = new Error("denied");
    startDraggingMock.mockRejectedValue(error);
    const onDragStartError = vi.fn();
    handle = document.createElement("div");
    document.body.append(handle);
    destroy = useThresholdDrag(handle, { onDragStartError });

    handle.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, button: 0, clientX: 0, clientY: 0 }),
    );
    window.dispatchEvent(
      new MouseEvent("mousemove", { bubbles: true, buttons: 1, clientX: 10, clientY: 0 }),
    );
    await Promise.resolve();
    await Promise.resolve();

    expect(onDragStartError).toHaveBeenCalledWith(error);
  });
});
