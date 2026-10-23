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

  it("行离场事件透传条目 id", async () => {
    // FIX015.9：App 依赖 rowLeave 在清单移除动画结束后关闭详情板。
    wrapper = mount(TodoList, { props: { items: [ITEM] } });
    const group = wrapper.findComponent({ name: "TransitionGroup" });
    const li = document.createElement("li");
    const row = document.createElement("div");
    row.className = "todo-row";
    row.dataset.rowId = String(ITEM.id);
    li.append(row);

    group.vm.$emit("after-leave", li);
    await flushPromises();

    expect(wrapper.emitted("rowLeave")).toEqual([[ITEM.id]]);
  });

  it("编辑中经托盘归档：先提交草稿再清编辑态", async () => {
    // FIX015.9：done 行仍在 props.items，不能沿用“列表找不到才清”的旧判断。
    wrapper = mount(TodoList, { props: { items: [ITEM] } });
    await flushPromises();
    await wrapper.find(".todo-row").trigger("dblclick");
    await flushPromises();

    const edit = wrapper.find<HTMLInputElement>(".t-edit");
    expect(edit.exists()).toBe(true);
    await edit.setValue("编辑后的标题");

    await wrapper.setProps({ items: [{ ...ITEM, done: true }] });
    await flushPromises();

    expect(wrapper.find(".t-edit").exists()).toBe(false);
    expect(invokeMock).toHaveBeenCalledWith("todo_rename", {
      id: ITEM.id,
      text: "编辑后的标题",
    });
  });

  it("全部已完成时显示清空空态", async () => {
    // FIX015.8：空态必须按未完成显示集判断，不能按含 done 的 props.items.length。
    wrapper = mount(TodoList, {
      props: { items: [{ ...ITEM, done: true }] },
    });
    await flushPromises();
    expect(wrapper.find(".empty").exists()).toBe(true);

    await wrapper.setProps({ items: [{ ...ITEM, done: false }] });
    await flushPromises();
    expect(wrapper.find(".empty").exists()).toBe(false);
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
