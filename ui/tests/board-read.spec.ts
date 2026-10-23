import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useBoardRead } from "../src/composables/useBoardRead";

/** 可控 ResizeObserver：测试里手动触发内容增长回调。 */
class CapturingResizeObserver {
  static latest: CapturingResizeObserver | null = null;
  readonly observe = vi.fn();
  readonly unobserve = vi.fn();
  readonly disconnect = vi.fn();

  constructor(readonly callback: ResizeObserverCallback) {
    CapturingResizeObserver.latest = this;
  }
}

/** 给 jsdom 容器补可控几何。 */
function setGeometry(el: HTMLElement, height: number, scrollHeight: number): void {
  Object.defineProperty(el, "clientHeight", {
    configurable: true,
    get: () => height,
  });
  Object.defineProperty(el, "scrollHeight", {
    configurable: true,
    get: () => scrollHeight,
  });
}

describe("useBoardRead 贴底状态（FIX015.7）", () => {
  let board: HTMLElement;
  let scroller: HTMLElement;
  let kit: ReturnType<typeof useBoardRead> | undefined;

  beforeEach(() => {
    board = document.createElement("div");
    board.id = "board";
    document.body.append(board);
    scroller = document.createElement("div");
    board.append(scroller);
    CapturingResizeObserver.latest = null;
    vi.stubGlobal("ResizeObserver", CapturingResizeObserver);
  });

  afterEach(() => {
    kit?.destroy();
    kit = undefined;
    board.remove();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("短列表首次增长不把视口甩到底部", () => {
    setGeometry(scroller, 100, 100);
    scroller.scrollTop = 0;
    kit = useBoardRead(scroller);

    setGeometry(scroller, 100, 220);
    CapturingResizeObserver.latest?.callback(
      [],
      CapturingResizeObserver.latest as unknown as ResizeObserver,
    );

    expect(scroller.scrollTop).toBe(0);
  });

  it("原本贴底的列表增长时仍补到底部", () => {
    setGeometry(scroller, 100, 220);
    scroller.scrollTop = 120;
    kit = useBoardRead(scroller);

    setGeometry(scroller, 100, 320);
    CapturingResizeObserver.latest?.callback(
      [],
      CapturingResizeObserver.latest as unknown as ResizeObserver,
    );

    expect(scroller.scrollTop).toBe(320);
  });
});
