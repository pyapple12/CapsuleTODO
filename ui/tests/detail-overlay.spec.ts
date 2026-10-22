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
});
