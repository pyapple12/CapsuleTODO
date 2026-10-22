import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { TodoView } from "../types";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import TodoList from "../src/components/TodoList.vue";

const ITEM: TodoView = {
  id: 1,
  text: "测试行",
  done: false,
  created_at: null,
  done_at: null,
  has_note: false,
  age_level: "None",
};

describe("TodoList 组件级交互（FIX014.35）", () => {
  let wrapper: VueWrapper | undefined;

  beforeEach(() => {
    const board = document.createElement("div");
    board.id = "board";
    document.body.append(board);
    vi.useFakeTimers();
    invokeMock.mockReset();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "todo_toggle") return { ...ITEM, done: true };
      return undefined;
    });
  });

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
    document.getElementById("board")?.remove();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("主拍结束后刷新到达前双击，行消失时清除编辑态", async () => {
    wrapper = mount(TodoList, { props: { items: [] } });
    await wrapper.setProps({ items: [ITEM] });
    await flushPromises();

    const row = wrapper.find(".todo-row");
    await row.trigger("click", { clientX: 0, clientY: 0 });
    await flushPromises();
    vi.advanceTimersByTime(300);
    await flushPromises();

    await row.trigger("dblclick");
    await flushPromises();
    expect(wrapper.find(".t-edit").exists()).toBe(true);

    await wrapper.setProps({ items: [] });
    await flushPromises();
    expect(wrapper.find(".t-edit").exists()).toBe(false);
  });

  it("勾选 300ms 主拍内双击不进入行内编辑", async () => {
    wrapper = mount(TodoList, { props: { items: [] } });
    await wrapper.setProps({ items: [ITEM] });
    await flushPromises();

    const row = wrapper.find(".todo-row");
    expect(row.exists()).toBe(true);

    await row.trigger("click", { clientX: 0, clientY: 0 });
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith("todo_toggle", { id: ITEM.id });

    await row.trigger("dblclick");
    await flushPromises();
    expect(wrapper.find(".t-edit").exists()).toBe(false);

    vi.advanceTimersByTime(300);
    await flushPromises();
    expect(wrapper.emitted("archived")).toHaveLength(1);
  });
});
