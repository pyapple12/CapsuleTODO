import { describe, expect, it } from "vitest";

import { isBlank, normalizeBlocks, type LocalBlock } from "../src/composables/normalizeBlocks";

/** 造一个文本块 */
function textBlock(text: string, key: string, id = 0): LocalBlock {
  return { id, kind: "Text", text, image_path: null, _key: key };
}

/** 造一个图块 */
function imageBlock(key: string, id = 0): LocalBlock {
  return { id, kind: "Image", text: "", image_path: "note_x/1.png", _key: key };
}

/** 新块 key 生成器（用例内确定性；真实调用方传组件 `localKey`） */
function makeKeyGen(): () => string {
  let n = 0;
  return () => {
    n += 1;
    return `new-${n}`;
  };
}

/** 跑一次归一化 */
function run(list: LocalBlock[], preferredKey?: string): LocalBlock[] {
  return normalizeBlocks(list, { preferredKey, makeKey: makeKeyGen() });
}

/**
 * PATCH002.75 回归用例：**全空补块必须复用既有空块对象**（保 `_key`）。
 * 反向断言 = 引用相等——旧实现此处新建对象，导致"焦点所在块"被 Vue 销毁、光标退回 body
 * （`Ctrl+A` → `Enter` 后只剩 1 块且打字无反应）。
 */
describe("normalizeBlocks：全空补块复用对象（PATCH002.75）", () => {
  it("两块皆空 ⇒ 留一个，且复用的是**最后**一块（拆块新建、光标所在）", () => {
    const a = textBlock("", "A");
    const b = textBlock("", "B");
    const out = run([a, b]);
    expect(out).toHaveLength(1);
    expect(out[0]).toBe(b); // 引用相等 = 焦点块保活
  });

  it("单块皆空 ⇒ 复用同一对象（不再新建）", () => {
    const a = textBlock("", "A");
    const out = run([a]);
    expect(out).toHaveLength(1);
    expect(out[0]).toBe(a);
  });

  it("preferredKey 优先于“最后一块”（焦点块不是末块时也保得住）", () => {
    const a = textBlock("", "A");
    const b = textBlock("", "B");
    const out = run([a, b], "A");
    expect(out).toHaveLength(1);
    expect(out[0]).toBe(a);
  });

  it("空白串块被复用时文本归一为空串（保持旧语义）", () => {
    const ws = textBlock("   ", "WS");
    const b = textBlock("", "B");
    const out = run([ws, b]);
    expect(out).toHaveLength(1);
    expect(out[0]).toBe(b); // 最后一块优先
    expect(out[0].text).toBe("");
  });

  it("只有空白串块 ⇒ 复用它并清成空串", () => {
    const ws = textBlock("  \t ", "WS");
    const out = run([ws]);
    expect(out[0]).toBe(ws);
    expect(ws.text).toBe("");
  });

  it("末块图 ⇒ 补新块（这一处**应当**新建）", () => {
    const img = imageBlock("I");
    const out = run([img]);
    expect(out).toHaveLength(2);
    expect(out[0]).toBe(img);
    expect(out[1].kind).toBe("Text");
    expect(out[1]._key).toBe("new-1"); // 走 makeKey ⇒ 新建
  });

  it("前导空块 + 图 ⇒ 图在前、空块复用补到末尾", () => {
    const a = textBlock("", "A");
    const img = imageBlock("I");
    const out = run([a, img]);
    expect(out.map((b) => b._key)).toEqual(["I", "A"]);
    expect(out[1]).toBe(a); // 复用，不新建
  });
});

describe("normalizeBlocks：既有规则不变", () => {
  it("文本 + 尾部空块 ⇒ 各自保留（只删前导 / 并连续）", () => {
    const t = textBlock("正文", "T");
    const b = textBlock("", "B");
    const out = run([t, b]);
    expect(out).toHaveLength(2);
    expect(out[0]).toBe(t);
    expect(out[1]).toBe(b);
  });

  it("前导空块删掉、末块空块保留、非空块引用不变", () => {
    const a = textBlock("", "A");
    const t = textBlock("正文", "T");
    const b = textBlock("", "B");
    const out = run([a, t, b]);
    expect(out.map((x) => x._key)).toEqual(["T", "B"]);
    expect(out[0]).toBe(t);
    expect(out[1]).toBe(b);
  });

  it("连续空块合并为一个", () => {
    const t = textBlock("正文", "T");
    const a = textBlock("", "A");
    const b = textBlock("", "B");
    const out = run([t, a, b]);
    expect(out.map((x) => x._key)).toEqual(["T", "A"]);
  });

  it("空列表 ⇒ 补一个新建空块", () => {
    const out = run([]);
    expect(out).toHaveLength(1);
    expect(out[0].kind).toBe("Text");
    expect(out[0]._key).toBe("new-1");
  });
});

describe("isBlank", () => {
  it("文本块：trim 后为空才算空", () => {
    expect(isBlank(textBlock("", "A"))).toBe(true);
    expect(isBlank(textBlock("  \n ", "B"))).toBe(true);
    expect(isBlank(textBlock("x", "C"))).toBe(false);
  });

  it("图块永不算空块", () => {
    expect(isBlank(imageBlock("I"))).toBe(false);
  });
});
