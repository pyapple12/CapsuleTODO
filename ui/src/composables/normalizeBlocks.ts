import type { NoteBlock } from "../../types";

/** 本地块（带客户端稳定 key：id 落库回填不重挂 DOM，防输入焦点/光标丢失） */
export interface LocalBlock extends NoteBlock {
  _key: string;
}

/** 归一化选项（PATCH002.75） */
export interface NormalizeOptions {
  /** 当前焦点块的 `_key`：全空补块时**优先复用它** ⇒ 焦点元素不被 Vue 销毁（光标不丢） */
  preferredKey?: string;
  /** 新建块用的 key 生成器——**必须由调用方传入**：与组件共用同一序列，防 key 撞车 */
  makeKey: () => string;
}

/** 空文本块判定（trim 后为空；图块永不算空块） */
export function isBlank(block: LocalBlock): boolean {
  return block.kind === "Text" && block.text.trim() === "";
}

/**
 * 空块归一化：合并连续空块 / 删前导空块 / 全空补一个文本块 / 末块不得为图块。
 * 尽量复用同一批对象（保 `_key` ⇒ DOM 不重挂）；**全空补块也复用**——PATCH002.75 实证：
 * 旧实现在这里新建对象，会把"焦点所在的空块"换掉 ⇒ Vue 销毁该 textarea ⇒ 光标退回 body
 * （`Ctrl+A` → `Enter` 后"只剩 1 块 + 打字无反应"的根因）。
 * @param list 块列表（**可能被就地改写**：被复用的空块文本归一为空串）
 * @param options `preferredKey` = 焦点块 key；`makeKey` = 新块 key 生成器
 * @returns 归一化后的块列表（尽量复用入参对象）
 */
export function normalizeBlocks(list: LocalBlock[], options: NormalizeOptions): LocalBlock[] {
  const out: LocalBlock[] = [];
  for (const block of list) {
    if (isBlank(block)) {
      if (out.length === 0) continue; // 前导空块：丢弃
      if (isBlank(out[out.length - 1])) continue; // 连续空块：合并
    }
    out.push(block);
  }
  if (!out.some((b) => b.kind === "Text")) {
    // PATCH002.75：全空补块**复用既有空块对象**（顺序 = 焦点块 → 最后一个空块 → 新建）
    const blanks = list.filter(isBlank);
    const reuse =
      (options.preferredKey ? blanks.find((b) => b._key === options.preferredKey) : undefined) ??
      blanks[blanks.length - 1];
    if (reuse) {
      reuse.text = ""; // 与旧行为一致：空白串块归一为空文本（否则库里留不可见空白）
      out.push(reuse);
    } else {
      out.push({ id: 0, kind: "Text", text: "", image_path: null, _key: options.makeKey() });
    }
  }
  // PATCH002.56（口径改定，2026-10-19）：**末块不得为图块** → 补一个空文本块（保"图后能输入"）。
  // 归一化本身是"保存点"语义（PATCH002.8），故补块也只在保存点发生；**补块由前端唯一负责**
  // （storage 侧不再自行补：两边都管会导致"补 → 下次保存被差分删"的来回抖动）
  if (out[out.length - 1]?.kind === "Image") {
    out.push({ id: 0, kind: "Text", text: "", image_path: null, _key: options.makeKey() });
  }
  return out;
}
