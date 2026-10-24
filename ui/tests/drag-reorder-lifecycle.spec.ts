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

  it("卸载时完整收掉 engaged 拖拽残留（ghost/RAF/userSelect/drag-live）", () => {
    wrapper = mount(Host, { attachTo: document.body });
    const list = document.getElementById("todo-active") as HTMLElement;
    list.getBoundingClientRect = () =>
      ({ top: 0, bottom: 1000, left: 0, right: 100, height: 1000, width: 100 }) as DOMRect;
    const row = wrapper.find(".todo-row").element as HTMLElement;
    row.getBoundingClientRect = () =>
      ({ top: 100, bottom: 140, left: 0, right: 100, height: 40, width: 100 }) as DOMRect;
    row.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, button: 0, clientX: 10, clientY: 120 }),
    );
    vi.advanceTimersByTime(300);

    expect(document.body.style.userSelect).toBe("none");
    expect(list.classList.contains("drag-live")).toBe(true);
    expect(document.querySelector(".todo-ghost")).not.toBeNull();

    wrapper.unmount();
    wrapper = undefined;
    expect(document.body.style.userSelect).toBe("");
    expect(list.classList.contains("drag-live")).toBe(false);
    expect(document.querySelector(".todo-ghost")).toBeNull();
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
