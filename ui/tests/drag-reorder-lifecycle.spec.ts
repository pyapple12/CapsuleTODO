import { mount } from "@vue/test-utils";
import { defineComponent, h } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useDragReorder } from "../src/composables/useDragReorder";

function rect(left: number, right: number): DOMRect {
  return {
    left,
    right,
    top: 0,
    bottom: 10,
    width: right - left,
    height: 10,
    x: left,
    y: 0,
    toJSON: () => ({}),
  } as DOMRect;
}

const Host = defineComponent({
  setup() {
    useDragReorder();
    return () =>
      h("div", { id: "board" }, [
        h("ul", { id: "todo-active", class: "board-read" }, [
          h("li", { class: "todo-item" }, [
            h("div", { class: "todo-row", "data-row-id": "1" }, [
              h("span", { class: "neon-checkbox" }),
            ]),
          ]),
        ]),
      ]);
  },
});

describe("useDragReorder 生命周期（FIX016.17）", () => {
  let wrapper: ReturnType<typeof mount> | undefined;

  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("无按键 move 清掉长按拍子", () => {
    wrapper = mount(Host, { attachTo: document.body });
    const box = wrapper.find(".neon-checkbox").element as HTMLElement;
    box.getBoundingClientRect = () => rect(20, 30);
    wrapper
      .find(".todo-row")
      .element.dispatchEvent(
        new MouseEvent("mousedown", { bubbles: true, button: 0, clientX: 0, clientY: 5 }),
      );
    expect(vi.getTimerCount()).toBe(1);

    document.dispatchEvent(new MouseEvent("mousemove", { buttons: 0, clientX: 0, clientY: 20 }));
    expect(vi.getTimerCount()).toBe(0);
  });

  it("卸载时清掉未到点的长按拍子", () => {
    wrapper = mount(Host, { attachTo: document.body });
    const box = wrapper.find(".neon-checkbox").element as HTMLElement;
    box.getBoundingClientRect = () => rect(20, 30);
    wrapper
      .find(".todo-row")
      .element.dispatchEvent(
        new MouseEvent("mousedown", { bubbles: true, button: 0, clientX: 0, clientY: 5 }),
      );
    expect(vi.getTimerCount()).toBe(1);

    wrapper.unmount();
    wrapper = undefined;
    expect(vi.getTimerCount()).toBe(0);
  });
});
