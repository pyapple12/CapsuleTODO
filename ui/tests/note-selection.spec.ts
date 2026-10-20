import { describe, expect, it } from "vitest";

import {
  initialNoteSelectionState,
  noteSelectionStep,
  useNoteSelection,
  type NoteSelectionEvent,
  type NoteSelectionHooks,
  type SelPoint,
} from "../src/composables/useNoteSelection";

/** 副作用记录器：断言“收原生选区”发生在正确的迁移点 */
function makeHooks(): { calls: string[]; hooks: NoteSelectionHooks } {
  const calls: string[] = [];
  return {
    calls,
    hooks: {
      collapseAll: (exceptKey?: string) => calls.push(`all:${exceptKey ?? ""}`),
      collapseTo: (key: string, offset: number) => calls.push(`to:${key}:${offset}`),
    },
  };
}

/** 跑一串事件（从初始状态） */
function run(events: NoteSelectionEvent[], hooks: NoteSelectionHooks) {
  let state = initialNoteSelectionState();
  for (const event of events) state = noteSelectionStep(state, event, hooks);
  return state;
}

const P = (index: number, offset: number): SelPoint => ({ index, offset });

describe("noteSelectionStep（PATCH002.77）：相位迁移", () => {
  it("press → pressed；收所有原生并清掉图块选中", () => {
    const { calls, hooks } = makeHooks();
    let s = initialNoteSelectionState();
    s.imageKey = "img";
    s = noteSelectionStep(s, { type: "press", index: 1, down: { x: 10, y: 20 } }, hooks);
    expect(s.phase).toBe("pressed");
    expect(s.sourceIndex).toBe(1);
    expect(s.down).toEqual({ x: 10, y: 20 });
    expect(s.imageKey).toBeNull();
    expect(calls).toEqual(["all:"]);
  });

  it("块内移动 → native；跨块 → cross 并记录 dragAnchor/隐藏源块原生", () => {
    const { calls, hooks } = makeHooks();
    let s = run(
      [
        { type: "press", index: 0, down: { x: 1, y: 1 } },
        { type: "moveIn", sourceKey: "a" },
      ],
      hooks,
    );
    expect(s.phase).toBe("native");
    s = noteSelectionStep(
      s,
      { type: "cross", anchor: P(0, 2), focus: P(1, 5), hideKey: "a" },
      hooks,
    );
    expect(s.phase).toBe("cross");
    expect(s.dragAnchor).toEqual(P(0, 2));
    expect(s.nativeHideKey).toBe("a");
    expect(s.hoverIndex).toBe(1);
    expect(calls[calls.length - 1]).toBe("all:a");
  });

  it("focus 只在 cross 生效", () => {
    const { hooks } = makeHooks();
    let s = run(
      [
        { type: "press", index: 0, down: { x: 1, y: 1 } },
        { type: "cross", anchor: P(0, 2), focus: P(1, 5), hideKey: "a" },
      ],
      hooks,
    );
    s = noteSelectionStep(s, { type: "focus", focus: P(2, 3) }, hooks);
    expect(s.focus).toEqual(P(2, 3));
    expect(s.hoverIndex).toBe(2);
  });

  it("回源块 → native：清自绘但保留 dragAnchor（再越出复用同一锚点）", () => {
    const { calls, hooks } = makeHooks();
    let s = run(
      [
        { type: "press", index: 0, down: { x: 1, y: 1 } },
        { type: "cross", anchor: P(0, 2), focus: P(2, 5), hideKey: "a" },
      ],
      hooks,
    );
    s = noteSelectionStep(s, { type: "exit", sourceKey: "a" }, hooks);
    expect(s.phase).toBe("native");
    expect(s.anchor).toBeNull();
    expect(s.focus).toBeNull();
    expect(s.dragAnchor).toEqual(P(0, 2));
    expect(calls[calls.length - 1]).toBe("all:a");
    // 再越出：调用方用 dragAnchor，step 不反推
    s = noteSelectionStep(
      s,
      { type: "cross", anchor: s.dragAnchor!, focus: P(3, 1), hideKey: "a" },
      hooks,
    );
    expect(s.phase).toBe("cross");
    expect(s.dragAnchor).toEqual(P(0, 2));
  });

  it("up：cross → selected（保留选区与隐藏 key）；native → idle", () => {
    const { hooks } = makeHooks();
    let s = run(
      [
        { type: "press", index: 0, down: { x: 1, y: 1 } },
        { type: "cross", anchor: P(0, 2), focus: P(1, 5), hideKey: "a" },
        { type: "up" },
      ],
      hooks,
    );
    expect(s.phase).toBe("selected");
    expect(s.anchor).toEqual(P(0, 2));
    expect(s.focus).toEqual(P(1, 5));
    expect(s.nativeHideKey).toBe("a");
    expect(s.sourceIndex).toBeNull();
    expect(s.dragAnchor).toBeNull();

    let n = run([{ type: "press", index: 1, down: { x: 1, y: 1 } }, { type: "up" }], hooks);
    expect(n.phase).toBe("idle");
  });

  it("Shift+点击：有锚点时设 focus/hideKey；无锚点时用传入锚点；图块选中环退场", () => {
    const { calls, hooks } = makeHooks();
    let s = initialNoteSelectionState();
    s.imageKey = "img";
    s = noteSelectionStep(
      s,
      { type: "shiftClick", anchor: P(0, 3), focus: P(1, 2), hideKey: "a" },
      hooks,
    );
    expect(s.phase).toBe("cross");
    expect(s.anchor).toEqual(P(0, 3));
    expect(s.focus).toEqual(P(1, 2));
    expect(s.imageKey).toBeNull();
    expect(calls[calls.length - 1]).toBe("all:a");
  });

  it("Shift+点击锚点块 → shiftExit：收锚点光标并清自绘", () => {
    const { calls, hooks } = makeHooks();
    let s = run(
      [
        { type: "press", index: 0, down: { x: 1, y: 1 } },
        { type: "cross", anchor: P(0, 2), focus: P(1, 5), hideKey: "a" },
      ],
      hooks,
    );
    s = noteSelectionStep(s, { type: "shiftExit", key: "a", offset: 2 }, hooks);
    expect(s.phase).toBe("idle");
    expect(s.anchor).toBeNull();
    expect(s.nativeHideKey).toBeNull();
    expect(calls.slice(-2)).toEqual(["all:a", "to:a:2"]);
  });

  it("Ctrl+A：selected + 清所有原生", () => {
    const { calls, hooks } = makeHooks();
    const s = run([{ type: "selectAll", from: P(0, 0), to: P(2, 4) }], hooks);
    expect(s.phase).toBe("selected");
    expect(s.anchor).toEqual(P(0, 0));
    expect(s.focus).toEqual(P(2, 4));
    expect(calls[calls.length - 1]).toBe("all:");
  });

  it("clear 不动原生（打字安全）；clearAll 收所有原生并清 imageKey", () => {
    const { calls, hooks } = makeHooks();
    let s = run(
      [
        { type: "press", index: 0, down: { x: 1, y: 1 } },
        { type: "cross", anchor: P(0, 2), focus: P(1, 5), hideKey: "a" },
      ],
      hooks,
    );
    const before = calls.length;
    s = noteSelectionStep(s, { type: "clear" }, hooks);
    expect(s.phase).toBe("idle");
    expect(calls.length).toBe(before);
    s = noteSelectionStep(s, { type: "clearAll" }, hooks);
    expect(calls[calls.length - 1]).toBe("all:");
  });

  it("图块单击：idle + imageKey + 收所有原生；清 imageKey 单独事件", () => {
    const { calls, hooks } = makeHooks();
    let s = noteSelectionStep(
      initialNoteSelectionState(),
      { type: "selectImage", key: "img" },
      hooks,
    );
    expect(s.phase).toBe("idle");
    expect(s.imageKey).toBe("img");
    expect(calls[calls.length - 1]).toBe("all:");
    s = noteSelectionStep(s, { type: "clearImage" }, hooks);
    expect(s.imageKey).toBeNull();
  });

  it("load 复位全部（含 imageKey）并收原生；sampling/hover/suppress 为纯标记", () => {
    const { calls, hooks } = makeHooks();
    let s = initialNoteSelectionState();
    s = noteSelectionStep(s, { type: "selectImage", key: "img" }, hooks);
    s = noteSelectionStep(s, { type: "load" }, hooks);
    expect(s.imageKey).toBeNull();
    expect(s.phase).toBe("idle");
    expect(calls[calls.length - 1]).toBe("all:");
    s = noteSelectionStep(s, { type: "sample", on: true }, hooks);
    s = noteSelectionStep(s, { type: "hover", index: 3 }, hooks);
    s = noteSelectionStep(s, { type: "suppress", on: true }, hooks);
    expect(s.sampling).toBe(true);
    expect(s.hoverIndex).toBe(3);
    expect(s.suppressImageClick).toBe(true);
  });
});

describe("useNoteSelection（包装层）：派生值与单一选中", () => {
  it("range 只在跨块 cross/selected 派生；回源不清 dragAnchor，up 后定格", () => {
    const { hooks } = makeHooks();
    const sel = useNoteSelection(hooks);
    expect(sel.range.value).toBeNull();
    sel.press(0, { x: 1, y: 1 });
    sel.beginCross("a");
    expect(sel.crossing.value).toBe(true);
    sel.enterCross(P(0, 2), P(2, 3), "a");
    expect(sel.range.value).toEqual({ from: P(0, 2), to: P(2, 3) });
    sel.exitToNative("a");
    expect(sel.range.value).toBeNull();
    expect(sel.crossing.value).toBe(false);
    expect(sel.sourceIndex.value).toBe(0);
    expect(sel.dragAnchor.value).toEqual(P(0, 2));
    sel.beginCross("a");
    sel.enterCross(sel.dragAnchor.value!, P(3, 1), "a");
    expect(sel.range.value).toEqual({ from: P(0, 2), to: P(3, 1) });
    sel.up();
    expect(sel.crossing.value).toBe(false);
    expect(sel.range.value).not.toBeNull();
    expect(sel.sourceIndex.value).toBe(-1);
  });

  it("shiftFocus 清 imageKey（单一可见选中）且 range 生效", () => {
    const sel = useNoteSelection(makeHooks().hooks);
    sel.selectImage("img");
    expect(sel.imageKey.value).toBe("img");
    sel.shiftFocus(P(2, 4), "a", P(0, 1));
    expect(sel.imageKey.value).toBeNull();
    expect(sel.range.value).toEqual({ from: P(0, 1), to: P(2, 4) });
    expect(sel.crossing.value).toBe(true);
  });

  it("selectAll 清图块选中；clear 不碰原生、clearAll 收原生", () => {
    const { calls, hooks } = makeHooks();
    const sel = useNoteSelection(hooks);
    sel.selectImage("img");
    sel.selectAll(P(0, 0), P(1, 2));
    expect(sel.imageKey.value).toBeNull();
    expect(sel.range.value).toEqual({ from: P(0, 0), to: P(1, 2) });
    const before = calls.length;
    sel.clear();
    expect(calls.length).toBe(before);
    expect(sel.range.value).toBeNull();
    sel.clearAll();
    expect(calls[calls.length - 1]).toBe("all:");
  });
});
