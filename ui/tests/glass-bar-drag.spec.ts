import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useGlassBar } from "../src/composables/useGlassBar";

/** 给元素固定矩形，避免 jsdom 全零几何导致 thumb 拖拽直接早退。 */
function setRect(el: HTMLElement, height: number, width = 100): void {
  el.getBoundingClientRect = () =>
    ({
      top: 0,
      left: 0,
      right: width,
      bottom: height,
      width,
      height,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    }) as DOMRect;
}

describe("useGlassBar 拖拽收尾（FIX015.31）", () => {
  let board: HTMLElement;
  let scroller: HTMLElement;
  let kit: ReturnType<typeof useGlassBar> | undefined;

  beforeEach(() => {
    board = document.createElement("div");
    board.id = "board";
    document.body.append(board);
    scroller = document.createElement("div");
    scroller.className = "board-read";
    board.append(scroller);
    setRect(board, 400);
    setRect(scroller, 100);
    Object.defineProperty(scroller, "clientHeight", { configurable: true, value: 100 });
    Object.defineProperty(scroller, "scrollHeight", { configurable: true, value: 300 });
    scroller.scrollTop = 0;
    kit = useGlassBar(scroller);
    const bar = board.querySelector<HTMLElement>(".glass-bar");
    const thumb = board.querySelector<HTMLElement>(".glass-thumb");
    expect(bar).not.toBeNull();
    expect(thumb).not.toBeNull();
    setRect(bar!, 100, 4);
    setRect(thumb!, 16, 4);
  });

  afterEach(() => {
    kit?.destroy();
    kit = undefined;
    board.remove();
    vi.restoreAllMocks();
  });

  it("无按键 move 不再滚动；destroy 清掉在飞监听", () => {
    const thumb = board.querySelector<HTMLElement>(".glass-thumb")!;
    thumb.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true, button: 0, clientY: 10 }));
    window.dispatchEvent(new MouseEvent("pointermove", { buttons: 0, clientY: 80 }));
    expect(scroller.scrollTop).toBe(0);

    thumb.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true, button: 0, clientY: 10 }));
    window.dispatchEvent(new MouseEvent("pointermove", { buttons: 1, clientY: 52 }));
    const moved = scroller.scrollTop;
    expect(moved).toBeGreaterThan(0);

    kit?.destroy();
    window.dispatchEvent(new MouseEvent("pointermove", { buttons: 1, clientY: 90 }));
    expect(scroller.scrollTop).toBe(moved);
  });
});
