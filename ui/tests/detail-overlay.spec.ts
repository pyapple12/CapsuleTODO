import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { NoteBlock, TodoItem } from "../types";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  convertFileSrc: (path: string) => path,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

import DetailOverlay from "../src/components/DetailOverlay.vue";

const TODO: TodoItem = {
  id: 1,
  text: "测试详情",
  done: false,
  created_at: null,
  done_at: null,
  has_note: true,
};
const TODO2: TodoItem = {
  id: 2,
  text: "另一条详情",
  done: false,
  created_at: null,
  done_at: null,
  has_note: true,
};

const TEXT_BLOCKS: NoteBlock[] = [
  { id: 11, kind: "Text", text: "abc", image_path: null },
  { id: 12, kind: "Text", text: "def", image_path: null },
];

const IMAGE_BLOCKS: NoteBlock[] = [
  { id: 21, kind: "Text", text: "前", image_path: null },
  { id: 22, kind: "Image", text: "", image_path: "C:/tmp/mock.png" },
  { id: 23, kind: "Text", text: "后", image_path: null },
];

/** 安装读取/保存 mock；保存按入参回填 id，模拟 Rust 侧。 */
function mockNoteInvoke(blocks: NoteBlock[]): void {
  invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "note_blocks_list") return blocks;
    if (cmd === "note_blocks_save") {
      const input = (args?.blocks ?? []) as NoteBlock[];
      return input.map((block, index) => ({
        ...block,
        id: block.id || 100 + index,
      }));
    }
    return undefined;
  });
}

async function openDetail(wrapper: VueWrapper, blocks: NoteBlock[]): Promise<void> {
  mockNoteInvoke(blocks);
  await wrapper.setProps({ todo: TODO, bubble: null, anchor: { x: 0, y: 0 } });
  await flushPromises();
}

describe("DetailOverlay 组件级交互（FIX014.35）", () => {
  let wrapper: VueWrapper | undefined;

  beforeEach(() => {
    const board = document.createElement("div");
    board.id = "board";
    document.body.append(board);
    invokeMock.mockReset();
    listenMock.mockReset();
    listenMock.mockResolvedValue(() => {});
  });

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
    document.getElementById("board")?.remove();
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it("Shift+Enter 拆块且不产生换行", async () => {
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, TEXT_BLOCKS);

    const textareas = wrapper.findAll<HTMLTextAreaElement>("textarea.note-block-text");
    expect(textareas).toHaveLength(2);
    const first = textareas[0].element;
    first.focus();
    first.setSelectionRange(1, 1);

    await textareas[0].trigger("keydown", { key: "Enter", shiftKey: true });
    await flushPromises();

    const after = wrapper.findAll<HTMLTextAreaElement>("textarea.note-block-text");
    expect(after).toHaveLength(3);
    expect(after.map((el) => el.element.value)).toEqual(["a", "bc", "def"]);
    expect(after.some((el) => el.element.value.includes("\n"))).toBe(false);
  });

  it("删除图块后 Ctrl+Z 恢复同一图块，且不提前收场", async () => {
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, IMAGE_BLOCKS);

    let finishDelete!: () => void;
    const finished = new Promise<void>((resolve) => {
      finishDelete = resolve;
    });
    vi.spyOn(Element.prototype, "animate").mockImplementation(
      () =>
        ({
          finished,
          cancel: vi.fn(),
          finish: vi.fn(),
          addEventListener: vi.fn(),
          removeEventListener: vi.fn(),
        }) as unknown as Animation,
    );

    expect(wrapper.findAll(".note-block-image")).toHaveLength(1);
    await wrapper.find(".note-block-image .note-block-del").trigger("click");
    await flushPromises();
    expect(Element.prototype.animate).toHaveBeenCalled();
    expect(wrapper.findAll(".note-block-image")).toHaveLength(1);

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", ctrlKey: true }));
    await flushPromises();
    expect(wrapper.findAll(".note-block-image")).toHaveLength(1);

    finishDelete();
    await flushPromises();
    expect(wrapper.findAll(".note-block-image")).toHaveLength(1);
  });

  it("删除动效中关板，settle 仍按旧条目落库", async () => {
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, IMAGE_BLOCKS);

    let finishDelete!: () => void;
    const finished = new Promise<void>((resolve) => {
      finishDelete = resolve;
    });
    vi.spyOn(Element.prototype, "animate").mockImplementation(
      () =>
        ({
          finished,
          cancel: vi.fn(),
          finish: vi.fn(),
          addEventListener: vi.fn(),
          removeEventListener: vi.fn(),
        }) as unknown as Animation,
    );

    await wrapper.find(".note-block-image .note-block-del").trigger("click");
    await flushPromises();

    // 动画尚未结束就关板：props.todo 清空；settle 仍必须用捕获的旧条目保存。
    await wrapper.setProps({ todo: null, bubble: null, anchor: null });
    await flushPromises();
    finishDelete();
    await flushPromises();

    const saved = invokeMock.mock.calls
      .filter(([cmd]) => cmd === "note_blocks_save")
      .map(([, args]) => args as { todoId: number; blocks: NoteBlock[] });
    expect(
      saved.some(
        ({ todoId, blocks }) =>
          todoId === TODO.id && blocks.length === 2 && blocks.every((b) => b.kind === "Text"),
      ),
    ).toBe(true);
  });

  it("删除动效中切源，settle 仍按旧条目落库", async () => {
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, IMAGE_BLOCKS);

    let finishDelete!: () => void;
    const finished = new Promise<void>((resolve) => {
      finishDelete = resolve;
    });
    vi.spyOn(Element.prototype, "animate").mockImplementation(
      () =>
        ({
          finished,
          cancel: vi.fn(),
          finish: vi.fn(),
          addEventListener: vi.fn(),
          removeEventListener: vi.fn(),
        }) as unknown as Animation,
    );
    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "note_blocks_list") {
        return args?.todoId === TODO2.id
          ? [{ id: 31, kind: "Text", text: "新条目", image_path: null }]
          : IMAGE_BLOCKS;
      }
      if (cmd === "note_blocks_save") {
        const input = (args?.blocks ?? []) as NoteBlock[];
        return input.map((block, index) => ({ ...block, id: block.id || 200 + index }));
      }
      return undefined;
    });

    await wrapper.find(".note-block-image .note-block-del").trigger("click");
    await flushPromises();
    await wrapper.setProps({ todo: TODO2, bubble: null, anchor: null });
    await flushPromises();
    finishDelete();
    await flushPromises();

    const saved = invokeMock.mock.calls
      .filter(([cmd]) => cmd === "note_blocks_save")
      .map(([, args]) => args as { todoId: number; blocks: NoteBlock[] });
    expect(
      saved.some(
        ({ todoId, blocks }) =>
          todoId === TODO.id && blocks.length === 2 && blocks.every((b) => b.kind === "Text"),
      ),
    ).toBe(true);
  });

  it("非选区图片粘贴在飞行中失焦不保存旧列表", async () => {
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, TEXT_BLOCKS);

    let releasePaste!: () => void;
    const pasteGate = new Promise<void>((resolve) => {
      releasePaste = resolve;
    });
    let saveCalls = 0;
    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "note_blocks_list") return TEXT_BLOCKS;
      if (cmd === "note_blocks_save") {
        saveCalls += 1;
        const input = (args?.blocks ?? []) as NoteBlock[];
        return input.map((block, index) => ({ ...block, id: block.id || 200 + index }));
      }
      if (cmd === "note_paste_image") {
        await pasteGate;
        return { status: "image", blocks: TEXT_BLOCKS };
      }
      return undefined;
    });

    const textarea = wrapper.find<HTMLTextAreaElement>("textarea.note-block-text");
    const paste = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(paste, "clipboardData", {
      value: { types: ["image/png"], getData: () => "" },
    });
    textarea.element.dispatchEvent(paste);
    await flushPromises();
    textarea.element.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
    await flushPromises();
    expect(saveCalls).toBe(0);

    releasePaste();
    await flushPromises();
  });

  it("失败恢复后不会被粘贴前武装的防抖保存覆盖", async () => {
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, TEXT_BLOCKS);
    vi.useFakeTimers();
    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "note_blocks_list") return TEXT_BLOCKS;
      if (cmd === "note_blocks_save") {
        const input = (args?.blocks ?? []) as NoteBlock[];
        return input.map((block, index) => ({ ...block, id: block.id || 200 + index }));
      }
      if (cmd === "note_paste_image") throw new Error("模拟图片粘贴失败");
      return undefined;
    });

    const textarea = wrapper.find<HTMLTextAreaElement>("textarea.note-block-text");
    await textarea.setValue("abcX");
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "a", ctrlKey: true }));
    const paste = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(paste, "clipboardData", {
      value: { types: ["image/png"], getData: () => "" },
    });
    textarea.element.dispatchEvent(paste);
    await flushPromises();
    await vi.advanceTimersByTimeAsync(300);
    await flushPromises();

    const saved = invokeMock.mock.calls
      .filter(([cmd]) => cmd === "note_blocks_save")
      .map(([, args]) => (args as { blocks: NoteBlock[] }).blocks);
    expect(saved.length).toBeGreaterThan(0);
    const last = saved[saved.length - 1].map((block) => block.text);
    expect(last).toContain("abcX");
    expect(last).toContain("def");
  });

  it("选区替换保存 await 后切源，不向新条目粘贴", async () => {
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, TEXT_BLOCKS);

    let releaseSave!: () => void;
    const saveGate = new Promise<void>((resolve) => {
      releaseSave = resolve;
    });
    let saveCount = 0;
    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "note_blocks_list") {
        return args?.todoId === TODO2.id
          ? [{ id: 31, kind: "Text", text: "新条目", image_path: null }]
          : TEXT_BLOCKS;
      }
      if (cmd === "note_blocks_save") {
        saveCount += 1;
        if (saveCount === 1) await saveGate;
        const input = (args?.blocks ?? []) as NoteBlock[];
        return input.map((block, index) => ({ ...block, id: block.id || 200 + index }));
      }
      if (cmd === "note_paste_html") {
        return { outcome: { status: "no_image" }, pending: [] };
      }
      return undefined;
    });

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "a", ctrlKey: true }));
    const textarea = wrapper.find<HTMLTextAreaElement>("textarea.note-block-text");
    const paste = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(paste, "clipboardData", {
      value: {
        types: ["text/html", "text/plain"],
        getData: (type: string) =>
          type === "text/html" ? '<img src="data:image/png;base64,AA==">' : "x",
      },
    });
    textarea.element.dispatchEvent(paste);
    await flushPromises();

    // 保存尚未返回时切源；旧条目的粘贴必须中止，不能落到 TODO2。
    await wrapper.setProps({ todo: TODO2, bubble: null, anchor: null });
    await flushPromises();
    releaseSave();
    await flushPromises();

    const saveCalls = invokeMock.mock.calls
      .filter(([cmd]) => cmd === "note_blocks_save")
      .map(([, args]) => args as { todoId: number; blocks: NoteBlock[] });
    expect(saveCalls.length).toBeGreaterThan(0);
    expect(saveCalls.every((args) => args.todoId === TODO.id)).toBe(true);
    expect(saveCalls[saveCalls.length - 1].blocks.map((block) => block.text)).toEqual([
      "abc",
      "def",
    ]);
    expect(invokeMock.mock.calls.some(([cmd]) => cmd === "note_paste_html")).toBe(false);
  });
});
