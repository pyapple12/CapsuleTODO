import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { BubbleItem, NoteBlock, TodoItem } from "../types";

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

const IMAGE_FIRST_BLOCKS: NoteBlock[] = [
  { id: 31, kind: "Image", text: "", image_path: "C:/tmp/first.png" },
  { id: 32, kind: "Text", text: "尾", image_path: null },
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
    vi.unstubAllGlobals();
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

  it("大写 IMG 的 HTML-only 粘贴仍进入 HTML 分支", async () => {
    // FIX016.6：前端图片判定必须大小写不敏感。
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, TEXT_BLOCKS);
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "note_blocks_list") return TEXT_BLOCKS;
      if (cmd === "note_paste_html") {
        return { outcome: { status: "no_image" }, pending: [] };
      }
      return undefined;
    });

    const textarea = wrapper.find<HTMLTextAreaElement>("textarea.note-block-text");
    const paste = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(paste, "clipboardData", {
      value: {
        types: ["text/html"],
        getData: (type: string) => (type === "text/html" ? "<IMG SRC='x.png'>" : ""),
      },
    });
    textarea.element.dispatchEvent(paste);
    await flushPromises();

    expect(invokeMock.mock.calls.some(([cmd]) => cmd === "note_paste_html")).toBe(true);
  });

  it("CR-only 多行粘贴归一为 LF 并落库", async () => {
    // FIX016.3：裸 CR 不能进入文本块，否则 Rust 校验会拒绝整条保存。
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, TEXT_BLOCKS);
    vi.useFakeTimers();

    const textarea = wrapper.find<HTMLTextAreaElement>("textarea.note-block-text");
    textarea.element.focus();
    textarea.element.setSelectionRange(0, 0);
    const paste = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(paste, "clipboardData", {
      value: {
        types: ["text/plain"],
        getData: (type: string) => (type === "text/plain" ? "a\rb" : ""),
      },
    });
    textarea.element.dispatchEvent(paste);
    await flushPromises();

    const values = wrapper
      .findAll<HTMLTextAreaElement>("textarea.note-block-text")
      .map((el) => el.element.value);
    expect(values).toEqual(["a", "babc", "def"]);
    expect(values.some((value) => value.includes("\r"))).toBe(false);

    await vi.advanceTimersByTimeAsync(300);
    await flushPromises();
    const saved = invokeMock.mock.calls
      .filter(([cmd]) => cmd === "note_blocks_save")
      .map(([, args]) => (args as { blocks: NoteBlock[] }).blocks);
    const last = saved[saved.length - 1];
    expect(last.map((block) => block.text)).toEqual(["a", "babc", "def"]);
    expect(last.every((block) => !block.text.includes("\r"))).toBe(true);
  });

  it("方向键落到图块后焦点交给图块容器，可继续返回文本", async () => {
    // FIX016.10：只 selectImage 不聚焦会让图块 keydown 永远收不到。
    wrapper = mount(DetailOverlay, {
      attachTo: document.body,
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, IMAGE_BLOCKS);
    const first = wrapper.find<HTMLTextAreaElement>("textarea.note-block-text");
    first.element.focus();
    first.element.setSelectionRange(first.element.value.length, first.element.value.length);

    await first.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();

    const image = wrapper.find<HTMLElement>('.note-block[data-row-id="2"]');
    expect(document.activeElement).toBe(image.element);

    await image.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    const textareas = wrapper.findAll<HTMLTextAreaElement>("textarea.note-block-text");
    expect(document.activeElement).toBe(textareas[textareas.length - 1].element);
  });

  it("图块首块选区删除新建文本块，撤销恢复原图块", async () => {
    // FIX015.2：首块 Image 不得复用图块 id，否则 image_file 被清空且文件释放。
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, IMAGE_FIRST_BLOCKS);
    vi.useFakeTimers();

    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "a",
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    await flushPromises();
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "Backspace",
        bubbles: true,
        cancelable: true,
      }),
    );
    await flushPromises();
    await vi.advanceTimersByTimeAsync(300);
    await flushPromises();

    const saved = invokeMock.mock.calls
      .filter(([cmd]) => cmd === "note_blocks_save")
      .map(([, args]) => (args as { blocks: NoteBlock[] }).blocks);
    const deleted = saved[saved.length - 1];
    expect(deleted).toEqual([expect.objectContaining({ id: 0, kind: "Text", text: "" })]);

    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "z",
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    await flushPromises();
    await vi.advanceTimersByTimeAsync(300);
    await flushPromises();

    const afterUndoSaved = invokeMock.mock.calls
      .filter(([cmd]) => cmd === "note_blocks_save")
      .map(([, args]) => (args as { blocks: NoteBlock[] }).blocks);
    const afterUndo = afterUndoSaved[afterUndoSaved.length - 1];
    expect(afterUndo).toEqual(
      expect.arrayContaining([expect.objectContaining({ id: 31, kind: "Image" })]),
    );
  });

  it("图块首块可打印字符插入新文本块，不丢首字", async () => {
    // FIX015.2：图块首块删除后原生字符没有稳定 textarea，必须显式插入。
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, IMAGE_FIRST_BLOCKS);
    vi.useFakeTimers();

    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "a",
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    await flushPromises();
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "X",
        bubbles: true,
        cancelable: true,
      }),
    );
    await flushPromises();

    const values = wrapper
      .findAll<HTMLTextAreaElement>("textarea.note-block-text")
      .map((el) => el.element.value);
    expect(values).toContain("X");
  });

  it("标题 Ctrl+A / Backspace 只作用于标题，不删除笔记", async () => {
    // FIX015.3：标题等输入面必须放行原生；先选中整篇再聚焦标题也必须清选区。
    // attachTo：本用例依赖 document.activeElement，脱离 document 的 wrapper 无法真正 focus。
    wrapper = mount(DetailOverlay, {
      attachTo: document.body,
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, TEXT_BLOCKS);
    const title = wrapper.find<HTMLInputElement>("input.detail-title");
    const before = wrapper
      .findAll<HTMLTextAreaElement>("textarea.note-block-text")
      .map((el) => el.element.value);

    // 先制造整篇自绘选区，再聚焦标题（模拟 Tab/点击离开块区）。
    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "a",
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    await flushPromises();
    title.element.focus();
    await flushPromises();
    expect(document.activeElement).toBe(title.element);

    const ctrlA = new KeyboardEvent("keydown", {
      key: "a",
      ctrlKey: true,
      bubbles: true,
      cancelable: true,
    });
    expect(window.dispatchEvent(ctrlA)).toBe(true);
    expect(ctrlA.defaultPrevented).toBe(false);

    await title.trigger("keydown", {
      key: "Backspace",
    });
    await flushPromises();

    expect(
      wrapper
        .findAll<HTMLTextAreaElement>("textarea.note-block-text")
        .map((el) => el.element.value),
    ).toEqual(before);
  });

  it("跨块选区 Backspace 在板级处理后不再触发块级 keydown", async () => {
    // FIX015.3：stopPropagation 必须阻止 textarea 的块级 handler 用陈旧 index 二次删除。
    wrapper = mount(DetailOverlay, {
      attachTo: document.body,
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, TEXT_BLOCKS);
    const first = wrapper.findAll<HTMLTextAreaElement>("textarea.note-block-text")[0];
    first.element.focus();

    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "a",
        ctrlKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    await flushPromises();

    const blockHandler = vi.fn();
    first.element.addEventListener("keydown", blockHandler);
    first.element.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "Backspace",
        bubbles: true,
        cancelable: true,
      }),
    );
    await flushPromises();

    expect(blockHandler).not.toHaveBeenCalled();
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

  it("粘贴窗口内关板仍保存标题改名", async () => {
    // FIX015.13：标题不依赖块模型，不能在 pasteInFlight 守卫前被 clearTimeout 丢掉。
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });
    await openDetail(wrapper, TEXT_BLOCKS);
    let releasePaste!: () => void;
    const pasteGate = new Promise<void>((resolve) => {
      releasePaste = resolve;
    });
    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "note_blocks_list") return TEXT_BLOCKS;
      if (cmd === "note_blocks_save") {
        const input = (args?.blocks ?? []) as NoteBlock[];
        return input.map((block, index) => ({ ...block, id: block.id || 200 + index }));
      }
      if (cmd === "note_paste_image") {
        await pasteGate;
        return { status: "no_image" };
      }
      return undefined;
    });

    const title = wrapper.find<HTMLInputElement>("input.detail-title");
    await title.setValue("粘贴中的新标题");
    const textarea = wrapper.find<HTMLTextAreaElement>("textarea.note-block-text");
    const paste = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(paste, "clipboardData", {
      value: { types: ["image/png"], getData: () => "" },
    });
    textarea.element.dispatchEvent(paste);
    await flushPromises();

    await wrapper.setProps({ todo: null, bubble: null, anchor: null });
    await flushPromises();

    const rename = invokeMock.mock.calls
      .filter(([cmd]) => cmd === "todo_rename")
      .map(([, args]) => args as { id: number; text: string });
    expect(rename).toContainEqual({ id: TODO.id, text: "粘贴中的新标题" });

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

  it("图片路径已返回、解码仍挂起时关板，第二道代际守卫仍拦截", async () => {
    let resolveDecode!: () => void;
    const decodeGate = new Promise<void>((resolve) => {
      resolveDecode = resolve;
    });
    vi.stubGlobal(
      "Image",
      class {
        src = "";
        decode(): Promise<void> {
          return decodeGate;
        }
      },
    );
    invokeMock.mockImplementation((cmd: string) =>
      cmd === "bubble_get_image_path" ? Promise.resolve("asset://localhost/a.png") : undefined,
    );
    const bubble: BubbleItem = { id: 8, text: "🖼️ 图片", kind: "Image" };
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });

    await wrapper.setProps({ todo: null, bubble, anchor: null });
    await flushPromises();
    await wrapper.setProps({ todo: null, bubble: null, anchor: null });
    await flushPromises();
    resolveDecode();
    await flushPromises();
    await flushPromises();

    expect(wrapper.find(".detail-overlay").classes()).not.toContain("open");
  });

  it("图片气泡解码中关板，晚到结果不得重新开板", async () => {
    // FIX015.33：切页/关板后旧异步 get/decode 必须失效。
    let releasePath!: (path: string) => void;
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "bubble_get_image_path") {
        return new Promise<string>((resolve) => {
          releasePath = resolve;
        });
      }
      return undefined;
    });
    const bubble: BubbleItem = { id: 7, text: "🖼️ 图片", kind: "Image" };
    wrapper = mount(DetailOverlay, {
      props: { todo: null, bubble: null, anchor: null },
    });

    await wrapper.setProps({ todo: null, bubble, anchor: null });
    await flushPromises();
    await wrapper.setProps({ todo: null, bubble: null, anchor: null });
    await flushPromises();
    releasePath("C:/tmp/mock.png");
    await flushPromises();

    expect(wrapper.find(".detail-overlay").classes()).not.toContain("open");
  });
});
