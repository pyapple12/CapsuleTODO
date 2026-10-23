import type { NoteBlockKind } from "../../types";
import { UI_NOTE_MAX_BLOCKS, UI_NOTE_MAX_LEN } from "../../types";

/** 粘贴限长输入块（只依赖块类型与文本，便于纯函数测试） */
export interface PasteLimitBlock {
  kind: NoteBlockKind;
  text: string;
}

/** 自绘选区端点（与 useNoteSelection::SelPoint 同形；不 import 具体组合式，保持纯逻辑） */
export interface PasteLimitPoint {
  index: number;
  offset: number;
}

/** 自绘选区范围 */
export interface PasteLimitRange {
  from: PasteLimitPoint;
  to: PasteLimitPoint;
}

/** 保守投影：文本总字符 / 块数（用于粘贴前静默闸门） */
export interface PasteLimitProjection {
  textChars: number;
  blockCount: number;
}

/** 统计 UTF-16 字符串的码点数（与 Rust `chars().count()` 对齐） */
function charCount(text: string): number {
  return Array.from(text).length;
}

/** 统计自绘选区覆盖的文本字符数（图块不计） */
function selectedChars(blocks: readonly PasteLimitBlock[], range: PasteLimitRange): number {
  const from = Math.max(0, Math.min(range.from.index, range.to.index));
  const to = Math.min(blocks.length - 1, Math.max(range.from.index, range.to.index));
  if (from > to) return 0;
  if (from === to) {
    const text = blocks[from]?.kind === "Text" ? blocks[from].text : "";
    const start = Math.min(range.from.offset, range.to.offset);
    const end = Math.max(range.from.offset, range.to.offset);
    return charCount(text.slice(start, end));
  }
  let total = 0;
  const first = blocks[from];
  const last = blocks[to];
  if (first?.kind === "Text") {
    total += charCount(first.text.slice(Math.max(0, range.from.offset)));
  }
  for (let i = from + 1; i < to; i += 1) {
    const block = blocks[i];
    if (block?.kind === "Text") total += charCount(block.text);
  }
  if (last?.kind === "Text") {
    total += charCount(last.text.slice(0, Math.max(0, range.to.offset)));
  }
  return total;
}

/** HTML 纯文本上界（仅 plainText 缺席时使用）：去标签后按字符计，保守拦截。 */
function htmlTextUpperBound(html: string): number {
  return charCount(html.replace(/<[^>]*>/g, ""));
}

/** 按 HTML 中的 `<img>` 数量计图块（大小写不敏感；srcset 不会额外增加） */
function htmlImageCount(html: string): number {
  const lower = html.toLowerCase();
  let count = 0;
  let from = 0;
  while (true) {
    const at = lower.indexOf("<img", from);
    if (at < 0) return count;
    count += 1;
    from = at + 4;
  }
}

/**
 * 保守估算“当前笔记 + 本次粘贴”的最终文本字符与块数（FIX014.6）。
 * 只用于粘贴前静默拒绝；真实最终块集仍由 Rust 在写入前校验。
 * HTML 的精确分段在 Rust 解析，这里按纯文本行 + `<img>` 数保守上界。
 */
export function projectPasteLimits(
  blocks: readonly PasteLimitBlock[],
  range: PasteLimitRange | null,
  plainText: string,
  html: string,
  hasImage: boolean,
): PasteLimitProjection {
  const currentChars = blocks.reduce(
    (sum, block) => sum + (block.kind === "Text" ? charCount(block.text) : 0),
    0,
  );
  const removedChars = range ? selectedChars(blocks, range) : 0;
  const normalized = plainText.replace(/\r\n?/g, "\n");
  // HTML-only 剪贴板（无 text/plain）不能把正文长度漏算；仅此时用 HTML 去标签上界。
  const incomingChars =
    charCount(normalized.replace(/\n/g, "")) +
    (normalized.length === 0 ? htmlTextUpperBound(html) : 0);
  const textLines = normalized.length === 0 ? 0 : normalized.split("\n").length;
  const imageCount = htmlImageCount(html) || (hasImage ? 1 : 0);
  // 图粘贴会切出 before / after 两段；文本行按每行一块计——都取保守上界
  const incomingBlocks = textLines + imageCount + (imageCount > 0 ? 2 : 0);
  const removedBlocks = range ? Math.max(0, range.to.index - range.from.index) : 0;
  return {
    textChars: Math.max(0, currentChars - removedChars + incomingChars),
    blockCount: Math.max(1, blocks.length - removedBlocks + incomingBlocks),
  };
}

/** 粘贴是否仍在笔记上限内（超限 → 调用方静默 preventDefault，不写库） */
export function notePasteFitsLimits(
  blocks: readonly PasteLimitBlock[],
  range: PasteLimitRange | null,
  plainText: string,
  html: string,
  hasImage: boolean,
): boolean {
  const projection = projectPasteLimits(blocks, range, plainText, html, hasImage);
  return projection.textChars <= UI_NOTE_MAX_LEN && projection.blockCount <= UI_NOTE_MAX_BLOCKS;
}
