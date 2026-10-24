import { describe, expect, it } from "vitest";

import {
  normalizePasteNewlines,
  notePasteFitsLimits,
  projectPasteLimits,
} from "../src/composables/notePasteLimits";
import { UI_NOTE_MAX_BLOCKS, UI_NOTE_MAX_LEN } from "../types";

const textBlock = (text: string) => ({ kind: "Text" as const, text });

describe("notePasteFitsLimits（FIX014.6）：粘贴前静默闸门", () => {
  it("总字符超过上限时拒绝", () => {
    const blocks = [textBlock("a".repeat(UI_NOTE_MAX_LEN - 1))];
    expect(notePasteFitsLimits(blocks, null, "ab", "", false)).toBe(false);
  });

  it("恰好到上限时允许", () => {
    const blocks = [textBlock("a".repeat(UI_NOTE_MAX_LEN - 1))];
    expect(notePasteFitsLimits(blocks, null, "a", "", false)).toBe(true);
  });

  it("多行文本按块数拒绝", () => {
    const blocks = Array.from({ length: UI_NOTE_MAX_BLOCKS - 1 }, (_, i) => textBlock(`第${i}块`));
    expect(notePasteFitsLimits(blocks, null, "甲\n乙\n丙", "", false)).toBe(false);
  });

  it("选区内文本被替换时扣除原文，不误拒", () => {
    const blocks = [textBlock("a".repeat(UI_NOTE_MAX_LEN))];
    const range = {
      from: { index: 0, offset: 0 },
      to: { index: 0, offset: UI_NOTE_MAX_LEN },
    };
    expect(notePasteFitsLimits(blocks, range, "b", "", false)).toBe(true);
  });

  it("图片按图块预占，接近块数上限时拒绝", () => {
    const blocks = Array.from({ length: UI_NOTE_MAX_BLOCKS - 1 }, (_, i) => textBlock(`第${i}块`));
    expect(notePasteFitsLimits(blocks, null, "", "", true)).toBe(false);
  });

  it("projectPasteLimits 返回可断言的投影", () => {
    const projection = projectPasteLimits([textBlock("ab")], null, "c\n d", "", false);
    expect(projection.textChars).toBe(5);
    expect(projection.blockCount).toBe(3);
  });

  it("HTML-only 剪贴板也计入正文长度", () => {
    // FIX015.36：plainText 缺席时不能只数图片，否则超长正文会绕过前端闸门。
    const blocks = [textBlock("a".repeat(UI_NOTE_MAX_LEN - 1))];
    expect(notePasteFitsLimits(blocks, null, "", `<p>${"字".repeat(2)}</p>`, false)).toBe(false);
  });

  it("CR-only / 混合换行统一归一为 LF", () => {
    // FIX016.3：粘贴路径与预检共用同一入口，不能把裸 CR 留进文本块。
    expect(normalizePasteNewlines("a\rb\r\nc\nd")).toBe("a\nb\nc\nd");
  });

  it("img-foo 伪标签不增加图块数，image 别名增加", () => {
    // FIX017.3：块数预检与 Rust 解析使用同一图片标签语法。
    expect(
      projectPasteLimits([textBlock("a")], null, "", "<img-foo src='x'>", false).blockCount,
    ).toBe(1);
    expect(
      projectPasteLimits([textBlock("a")], null, "", "<image src='x'>", false).blockCount,
    ).toBe(4);
    expect(projectPasteLimits([textBlock("a")], null, "", "<IMG SRC='x'>", false).blockCount).toBe(
      4,
    );
  });
});
