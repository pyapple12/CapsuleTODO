<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type { BubbleItem, NoteBlock, NotePasteOutcome, TodoItem } from "../../types";
import { UI_TEXT_MAX_LEN } from "../../types";
import { useBoardRead } from "../composables/useBoardRead";
import { useGlassBar } from "../composables/useGlassBar";
import { installNoteDragHooks } from "../composables/useDragReorder";
import { bindOverlayState, syncVeils } from "../composables/useVeils";

// ===== 详情板（PL010.5 todo 模式；bubble 模式 PL012 填充；**PL027 详情改块序列**）：
// 一段一行一块 + 图块混排——Enter 光标处拆块 / Backspace@块首（上一块文字→合并、
// 上一块图→删该图）/ Delete 只作用本块内（永不涉及下方块）/ Ctrl+V 粘贴插图（唯一
// 插图入口，不做板内按钮）/ 图块悬停 ×、选中态键盘删、双击开原图、拖拽调序 /
// Ctrl+Enter 保存并关板（仅文本块响应）。标题行内改名沿用（≤12 字防抖）。
// 板内整板阅读 + 玻璃滑杆挂**滚动容器**（id 沿用 detail-note：帘联动与套件零改动）。

const props = defineProps<{
  /** 开板数据源：当前详情条目（null = 关板） */
  todo: TodoItem | null;
  /** 气泡全文板数据源（PL012.3 bubble 模式；todo 优先级低——两源互斥由父级保证） */
  bubble: BubbleItem | null;
  /** 被点行中心视口坐标（A3：飞出原点 = 行中心——design positionDetailOverlay 同款） */
  anchor: { x: number; y: number } | null;
}>();

const emit = defineEmits<{ changed: []; close: [] }>();

/** 双模式（V0.020 ⑩ 定案）：bubble = 单层玻璃（标签头摘除 + 只读落板面） */
const isBubbleMode = computed(() => props.bubble !== null);

const overlay = ref<HTMLElement | null>(null);
const titleDraft = ref("");
/** 气泡文本全文（bubble 模式只读展示） */
const bubbleText = ref("");
/** 气泡全文 textarea 元素（PATCH002.27：只读态不自己滚，高度由 autosize 定） */
const bubbleTextEl = ref<HTMLTextAreaElement | null>(null);
/** 气泡图片可渲染 src（PL025 载荷落盘化：开板按需 bubble_get_image_path 取绝对路径
 * → convertFileSrc 生成 asset URL；mock 环境返回 data URL 直用） */
const bubbleImgSrc = ref("");
/** 图片失效态文案（PL025：文件缺失 → 提示删除气泡；空串 = 正常） */
const imageError = ref("");

// —— PL027 块模型（一段一行一块 + 图块混排）——

/** 本地块（带客户端稳定 key：id 落库回填不重挂 DOM，防输入焦点/光标丢失） */
interface LocalBlock extends NoteBlock {
  _key: string;
}

const blocks = ref<LocalBlock[]>([]);
let keySeq = 0;

/** 生成客户端稳定 key */
function localKey(): string {
  keySeq += 1;
  return `blk-${keySeq}`;
}

/** 服务端块列表 → 本地块（附 key） */
function toLocal(list: NoteBlock[]): LocalBlock[] {
  return list.map((b) => ({ ...b, _key: localKey() }));
}

/** 块文本域登记表（key → 元素；聚焦/光标定位与自动增高用） */
const textEls = new Map<string, HTMLTextAreaElement>();

/** 文本域 ref 登记（挂/卸） */
function setTextEl(key: string, el: unknown): void {
  if (el) textEls.set(key, el as HTMLTextAreaElement);
  else textEls.delete(key);
}

/** 自动增高（内容换行显示，高度随内容） */
function autosize(el: HTMLTextAreaElement): void {
  el.style.height = "auto";
  el.style.height = `${el.scrollHeight}px`;
}

/** 气泡全文板自动增高（PATCH002.27）：只读 textarea 不再自己滚，滚动交回内层
 * `.note-scroll`；未渲染（图片气泡模式）时 no-op */
function autosizeBubble(): void {
  const el = bubbleTextEl.value;
  if (el) autosize(el);
}

/** 全部文本域重算高度（装载/重排后） */
function autosizeAll(): void {
  textEls.forEach((el) => autosize(el));
}

/** PATCH002.13：**结构性变更**（拆块/合并/粘贴拆行/插图/归一化/撤销）后新块立即撑开——
 * 此前只在 `@input` 与聚焦块上重算，新块停在单行高、要等光标移过去才长开 */
watch(blocks, () => void nextTick(() => autosizeAll()), { deep: true });

/** 图块 src（绝对路径 → asset 协议 URL；data: 直用——mock 环境） */
function blockSrc(block: LocalBlock): string {
  if (!block.image_path) return "";
  return block.image_path.startsWith("data:") ? block.image_path : convertFileSrc(block.image_path);
}

/** 图块选中态（单击选中；选中后 Backspace/Delete 删除） */
const selectedKey = ref("");
/** 正在塌缩的图块 key（删除动效：高度塌缩 + 淡出） */
const leavingKey = ref("");
/** 图片加载失败的块 key（失效态：提示删除） */
const brokenKeys = ref<string[]>([]);

/** 图块加载失败 → 失效态（可删除） */
function onBlockImageError(block: LocalBlock): void {
  if (!brokenKeys.value.includes(block._key)) {
    brokenKeys.value = [...brokenKeys.value, block._key];
  }
}

// —— 保存（防抖 300ms + 全量覆盖 + 基准比对；业务校验在 Rust 保存路径）——

/** 最近一次成功落库的块签名（基准比对；todoId 锚定防跨条目污染） */
let savedSignature = "";
let savedTodoId: number | null = null;
let saveTimer: number | undefined;

/** 块签名（id/kind/text；不含 _key/image_path——后者由服务端定） */
function signature(list: LocalBlock[]): string {
  return JSON.stringify(list.map((b) => ({ id: b.id, kind: b.kind, text: b.text })));
}

/** 排入防抖保存（无变更/未开板直接跳过） */
function scheduleSave(): void {
  clearTimeout(saveTimer);
  const todo = props.todo;
  if (todo == null) return;
  if (signature(blocks.value) === savedSignature) return;
  saveTimer = window.setTimeout(() => {
    void saveBlocks();
  }, 300);
}

/** 立即落库（防抖到点 / 关板 / 切源 / 粘贴前调用）；失败落 console（详情板不设提示行，UI 静默）。
 * 守卫：本地块尚未载入当前条目（savedTodoId 未锚定）时直接跳过——防"开板瞬间
 * 空列表被当成全量覆盖"把服务端块集清空（冒烟实证：开板即报"详情块为空"） */
async function saveBlocks(): Promise<void> {
  const todo = props.todo;
  if (todo == null) return;
  if (savedTodoId !== todo.id) return;
  const sig = signature(blocks.value);
  if (sig === savedSignature && savedTodoId === todo.id) return;
  const payload = blocks.value.map((b) => ({ id: b.id, kind: b.kind, text: b.text }));
  try {
    const saved = await invoke<NoteBlock[]>("note_blocks_save", {
      todoId: todo.id,
      blocks: payload,
    });
    if (props.todo?.id !== todo.id) return;
    // 就地回填 id（保 _key → 不重挂 DOM，输入焦点不丢）；基准取回填后的签名
    saved.forEach((b, i) => {
      const local = blocks.value[i];
      if (!local) return;
      local.id = b.id;
      local.kind = b.kind;
      local.image_path = b.image_path;
    });
    savedTodoId = todo.id;
    savedSignature = signature(blocks.value);
    emit("changed");
  } catch (err) {
    console.error("详情保存失败", err);
  }
}

/** 载入该条目的块（开板/切条目/收场重拉） */
async function loadBlocks(todoId: number): Promise<void> {
  try {
    const list = await invoke<NoteBlock[]>("note_blocks_list", { todoId });
    if (props.todo?.id !== todoId) return;
    blocks.value = toLocal(list);
    blocks.value = normalizeBlocks(blocks.value);
    savedTodoId = todoId;
    savedSignature = signature(blocks.value);
    brokenKeys.value = [];
    void nextTick(() => autosizeAll());
  } catch (err) {
    console.error("详情块拉取失败", err);
  }
}

// —— 块编辑（**前端即时本地拆合**，Rust 只管保存校验）——

/** 聚焦某文本块并置光标 */
function focusBlock(key: string, caret: number): void {
  void nextTick(() => {
    const el = textEls.get(key);
    if (!el) return;
    el.focus();
    el.setSelectionRange(caret, caret);
    autosize(el);
  });
}

/** 文本块输入：重算高度 + 排保存（v-model 已写入模型）；打字即退出选区 + 清结构性撤销栈 */
function onBlockInput(e: Event): void {
  autosize(e.target as HTMLTextAreaElement);
  if (selAnchor.value || selFocus.value) clearSel();
  dropUndo();
  scheduleSave();
}

/** 光标处拆块（Enter）：前段留原块、后段新块，光标落新块首 */
function splitBlock(index: number, at: number): void {
  const block = blocks.value[index];
  if (!block) return;
  pushUndo(); // 结构性操作入栈（Ctrl+Z 可回退本次拆块）
  const after = block.text.slice(at);
  block.text = block.text.slice(0, at);
  const created: LocalBlock = {
    id: 0,
    kind: "Text",
    text: after,
    image_path: null,
    _key: localKey(),
  };
  blocks.value.splice(index + 1, 0, created);
  scheduleSave();
  focusBlock(created._key, 0);
}

/** 与上一文本块合并（Backspace@块首）：拼接 + 删本块，光标落合并点 */
function mergeWithPrev(index: number): void {
  const prev = blocks.value[index - 1];
  const cur = blocks.value[index];
  if (!prev || !cur || prev.kind !== "Text") return;
  pushUndo();
  const joinAt = prev.text.length;
  prev.text += cur.text;
  blocks.value.splice(index, 1);
  scheduleSave();
  focusBlock(prev._key, joinAt);
}

/** 删除块（图块三入口共用；动效：塌缩淡出后再落库——文件引用由服务端归零释放） */
async function deleteBlock(index: number): Promise<void> {
  const block = blocks.value[index];
  if (!block) return;
  pushUndo();
  if (selectedKey.value === block._key) selectedKey.value = "";
  leavingKey.value = block._key;
  // 高度塌缩 + 淡出（先钉住实测高再收到 0，max-height 才可过渡）
  const el = overlay.value?.querySelector<HTMLElement>(`[data-row-id="${index + 1}"]`);
  if (el) {
    el.style.overflow = "hidden";
    el.style.maxHeight = `${el.offsetHeight}px`;
    void el.offsetHeight; // 强制回流，确保 max-height 过渡生效
    el.style.maxHeight = "0px";
    el.style.opacity = "0";
  }
  await new Promise((resolve) => window.setTimeout(resolve, 180));
  leavingKey.value = "";
  const at = blocks.value.findIndex((b) => b._key === block._key);
  if (at < 0) return;
  blocks.value.splice(at, 1);
  scheduleSave();
  await saveBlocks();
}

/** 文本块键盘：Enter 拆块 / Backspace@块首（合并或删上图）；Delete 不拦截
 * （只作用本块内，永不涉及下方块）；Ctrl+Enter 由**板级** window 监听处理（PATCH002）；
 * 全程 IME 守卫 */
function onBlockKeydown(index: number, e: KeyboardEvent): void {
  if (e.isComposing) return;
  const el = e.target as HTMLTextAreaElement;
  const block = blocks.value[index];
  if (!block) return;
  if (e.key === "Enter" && !e.shiftKey && !e.altKey && !e.ctrlKey && !e.metaKey) {
    e.preventDefault();
    splitBlock(index, el.selectionStart ?? 0);
    return;
  }
  if (e.key === "Backspace" && (el.selectionStart ?? 0) === 0 && (el.selectionEnd ?? 0) === 0) {
    const prev = blocks.value[index - 1];
    if (!prev) return;
    e.preventDefault();
    if (prev.kind === "Image") void deleteBlock(index - 1);
    else mergeWithPrev(index);
    return;
  }
  // PATCH002.9 方向键跨块（方案乙）：↑/← 在"第一视觉行/块首"→ 上一块；↓/→ 在"末视觉行/块尾"→ 下一块
  if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
    const at = el.selectionStart ?? 0;
    const atEdge =
      e.key === "ArrowLeft"
        ? at === 0 && at === (el.selectionEnd ?? 0)
        : caretVisualLine(el, at) === 0;
    if (atEdge) {
      e.preventDefault();
      focusAdjacent(index - 1, "end");
    }
    return;
  }
  if (e.key === "ArrowDown" || e.key === "ArrowRight") {
    const at = el.selectionStart ?? 0;
    const atEdge =
      e.key === "ArrowRight"
        ? at === el.value.length && at === (el.selectionEnd ?? 0)
        : caretVisualLine(el, at) === lastVisualLine(el);
    if (atEdge) {
      e.preventDefault();
      focusAdjacent(index + 1, "start");
    }
  }
}

/** 板级键位（PATCH002）：详情板**开着就响应**（焦点在标题框、或不在任何内容上也算）；
 * 板外一律不响应；输入法组合态守卫。Ctrl+Enter 保存关板；Ctrl+A 全选所有块；
 * Ctrl+Z 结构性撤销（栈空放行原生）；跨块选区在场时 Ctrl+C 复制 / Delete·Backspace 删除 */
function onWindowKeydown(e: KeyboardEvent): void {
  if (!isOpen.value || e.isComposing) return;
  const mod = e.ctrlKey || e.metaKey;
  if (e.key === "Enter" && mod) {
    e.preventDefault();
    void saveAndClose();
    return;
  }
  if (isBubbleMode.value) return; // 气泡全文板只读：键位一律交原生
  if (mod && !e.altKey && !e.shiftKey && (e.key === "a" || e.key === "A")) {
    e.preventDefault();
    selectAllBlocks();
    return;
  }
  if (mod && !e.altKey && !e.shiftKey && (e.key === "z" || e.key === "Z")) {
    if (undoStack.length === 0) return; // 栈空放行原生（打字撤销）
    e.preventDefault();
    undoStructural();
    return;
  }
  if (mod && !e.altKey && (e.key === "c" || e.key === "C") && selRange.value) {
    e.preventDefault();
    void copySelection();
    return;
  }
  if ((e.key === "Delete" || e.key === "Backspace") && selRange.value) {
    e.preventDefault();
    deleteSelection();
  }
}

/** 保存未决草稿并关板（Ctrl+Enter；**仅文本块响应**，其余输入面一律不响应） */
async function saveAndClose(): Promise<void> {
  await normalizeAndFlush();
  close();
}

// —— 粘贴（**唯一插图入口**：响应系统 Ctrl+V，不做板内按钮）——

/** 剪贴板是否带图/文件（同步预判——决定是否拦截原生粘贴；判定粗筛，真读在 Rust） */
function clipboardHasImage(e: ClipboardEvent): boolean {
  const types = e.clipboardData ? Array.from(e.clipboardData.types) : [];
  return types.includes("Files") || types.some((t) => t.startsWith("image/"));
}

/** 文本块粘贴：带图 → 拦截走 Rust 插图；多行文本 → 拦截按行拆块；单行文本 → 原生 */
async function onBlockPaste(index: number, e: ClipboardEvent): Promise<void> {
  const el = e.target as HTMLTextAreaElement;
  const block = blocks.value[index];
  if (!block) return;
  const start = el.selectionStart ?? 0;
  const end = el.selectionEnd ?? 0;
  if (clipboardHasImage(e)) {
    e.preventDefault();
    await pasteImageAt(index, start, end, e.clipboardData?.getData("text/plain") ?? "");
    return;
  }
  const text = e.clipboardData?.getData("text/plain") ?? "";
  if (text.includes("\n") || text.includes("\r")) {
    e.preventDefault();
    pasteTextLines(index, start, end, text);
  }
}

/** 插图：先 flush 未决保存（把 live 文本交上去）→ Rust 落盘/复用 + 切分插入 →
 * **外科式打补丁**（PATCH002.14：按 id 映射回既有块、保 `_key`，DOM 不重挂、原生
 * undo 栈不丢——整体替换会重建全部 textarea 的 DOM） */
async function pasteImageAt(
  index: number,
  start: number,
  end: number,
  fallbackText: string,
): Promise<void> {
  const todo = props.todo;
  const block = blocks.value[index];
  if (todo == null || !block) return;
  const before = block.text.slice(0, start);
  const after = block.text.slice(end);
  await saveBlocks(); // 保证块 id 已回填（本地新块 id=0 无法定位）
  try {
    const outcome = await invoke<NotePasteOutcome>("note_paste_image", {
      todoId: todo.id,
      blockId: block.id,
      before,
      after,
    });
    if (props.todo?.id !== todo.id) return;
    if (outcome.status === "image") {
      pushUndo(); // 结构性操作入栈（仅真插图入栈；文本回落走 pasteTextLines 自家入栈）
      blocks.value = normalizeBlocks(reconcileLocal(outcome.blocks));
      savedSignature = signature(blocks.value);
      brokenKeys.value = [];
      void nextTick(() => autosizeAll());
      emit("changed");
    } else {
      // 剪贴板无图（或非图片文件）→ 回落文本粘贴（含多行拆块），不吞粘贴
      pasteTextLines(index, start, end, fallbackText);
    }
  } catch (err) {
    console.error("插入图片失败", err);
  }
}

/** 多行文本粘贴：按行拆成多块（保持"一段一行一块"不变量）；单行不走此处（原生粘贴） */
function pasteTextLines(index: number, start: number, end: number, text: string): void {
  const block = blocks.value[index];
  if (!block) return;
  pushUndo();
  const head = block.text.slice(0, start);
  const tail = block.text.slice(end);
  const lines = text.split("\n").map((line) => line.replace(/\r$/, ""));
  // 首行并入当前块；其余行各成一块；末行接尾段
  block.text = head + (lines[0] ?? "");
  const created: LocalBlock[] = [];
  for (let i = 1; i < lines.length; i += 1) {
    created.push({
      id: 0,
      kind: "Text",
      text: lines[i],
      image_path: null,
      _key: localKey(),
    });
  }
  if (created.length > 0) {
    const last = created[created.length - 1];
    last.text += tail;
    blocks.value.splice(index + 1, 0, ...created);
    scheduleSave();
    focusBlock(last._key, last.text.length - tail.length);
  } else {
    block.text += tail;
    scheduleSave();
    focusBlock(block._key, block.text.length - tail.length);
  }
}

// —— 图块：单击选中 / 双击开原图 / 选中态键盘删 ——

/** 双击图块 → 系统默认程序打开**原图**（跳过预览）；互斥：开原图即清选区 */
async function openBlockImage(block: LocalBlock): Promise<void> {
  if (block.id <= 0) return;
  clearSel();
  try {
    await invoke("note_open_image", { blockId: block.id });
  } catch (err) {
    console.error("打开原图失败", err);
  }
}

/** 图块容器单击 → 选中（文本块的点击交给 textarea 自身处理）；互斥：清选区 */
function onBlockContainerClick(block: LocalBlock): void {
  if (block.kind === "Image") {
    clearSel();
    selectedKey.value = block._key;
  }
}

/** 图块容器键盘：选中态 Backspace/Delete 删；**方向键跳到相邻块**（PATCH002.9）；
 * 文本块交给 textarea 自身处理器 */
function onBlockContainerKeydown(index: number, e: KeyboardEvent): void {
  if (blocks.value[index]?.kind !== "Image") return;
  if (e.isComposing) return;
  if (e.key === "Backspace" || e.key === "Delete") {
    e.preventDefault();
    void deleteBlock(index);
    return;
  }
  if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
    e.preventDefault();
    focusAdjacent(index - 1, "end");
    return;
  }
  if (e.key === "ArrowDown" || e.key === "ArrowRight") {
    e.preventDefault();
    focusAdjacent(index + 1, "start");
  }
}

// —— 空块归一化（PATCH002：合并连续空块 / 删前导空块 / 结果为空补一个空文本块）——

/** 空文本块判定（trim 后为空） */
function isBlank(block: LocalBlock): boolean {
  return block.kind === "Text" && block.text.trim() === "";
}

/** 空块归一化：**合并连续空块 / 删前导空块 / 结果为空补一个空文本块**；
 * 复用同一批对象（保 `_key`，不重挂 DOM） */
function normalizeBlocks(list: LocalBlock[]): LocalBlock[] {
  const out: LocalBlock[] = [];
  for (const block of list) {
    if (isBlank(block)) {
      if (out.length === 0) continue; // 前导空块：丢弃
      if (isBlank(out[out.length - 1])) continue; // 连续空块：合并
    }
    out.push(block);
  }
  if (!out.some((b) => b.kind === "Text")) {
    out.push({ id: 0, kind: "Text", text: "", image_path: null, _key: localKey() });
  }
  return out;
}

/** **保存点**归一化 + 立即落库（PATCH002.8：Ctrl+Enter / 焦点离开块区 / 关板切源专用） */
async function normalizeAndFlush(): Promise<void> {
  clearTimeout(saveTimer);
  const next = normalizeBlocks(blocks.value);
  if (next.length !== blocks.value.length) {
    blocks.value = next;
  }
  await saveBlocks();
}

// —— 方向键跨块（PATCH002.9 方案乙：隐藏镜像测视觉行）——

/** 隐藏镜像元素（与文本块逐项同款样式；测光标视觉行） */
let caretMirror: HTMLDivElement | null = null;

/** 取/建镜像（惰性；样式每次同步——主题/字号可变） */
function mirrorFor(el: HTMLTextAreaElement): HTMLDivElement {
  if (!caretMirror) {
    caretMirror = document.createElement("div");
    caretMirror.style.position = "absolute";
    caretMirror.style.visibility = "hidden";
    caretMirror.style.pointerEvents = "none";
    caretMirror.style.top = "0";
    caretMirror.style.left = "-9999px";
    document.body.appendChild(caretMirror);
  }
  const cs = getComputedStyle(el);
  const m = caretMirror;
  m.style.font = cs.font;
  m.style.lineHeight = cs.lineHeight;
  m.style.letterSpacing = cs.letterSpacing;
  m.style.padding = cs.padding;
  m.style.border = cs.border;
  m.style.boxSizing = cs.boxSizing;
  m.style.width = `${el.clientWidth}px`;
  m.style.whiteSpace = "pre-wrap";
  m.style.wordBreak = "break-word";
  return m;
}

/** 光标所在视觉行（0 起）：文本前缀 + 标记 span → `span.offsetTop` ÷ 行高 */
function caretVisualLine(el: HTMLTextAreaElement, at: number): number {
  const m = mirrorFor(el);
  m.textContent = el.value.slice(0, at);
  const marker = document.createElement("span");
  marker.textContent = "\u200b";
  m.appendChild(marker);
  const lineHeight = parseFloat(getComputedStyle(el).lineHeight) || 1;
  return Math.max(0, Math.round(marker.offsetTop / lineHeight));
}

/** 文本总视觉行数 - 1（末行号）：标记落在全文末尾 */
function lastVisualLine(el: HTMLTextAreaElement): number {
  return caretVisualLine(el, el.value.length);
}

/** 跨块落点（PATCH002.9）：相邻块是文本 → 聚焦并把光标放到该端；是图块 → **选中它** */
function focusAdjacent(index: number, edge: "start" | "end"): void {
  const block = blocks.value[index];
  if (!block) return;
  if (block.kind === "Image") {
    selectedKey.value = block._key;
    return;
  }
  focusBlock(block._key, edge === "start" ? 0 : block.text.length);
}

/** 焦点离开块区（PATCH002.8 保存点）：归一化 + 立即落库 */
function onBlocksFocusOut(e: FocusEvent): void {
  const container = scrollEl.value;
  const next = e.relatedTarget as Node | null;
  if (container && next && container.contains(next)) return; // 仍在块区内
  void normalizeAndFlush();
}

// —— 结构性撤销（PATCH002.14 用户定案"乙"：只收结构性操作；打字走浏览器原生）——
// 栈非空 → Ctrl+Z 截走并还原快照；栈空 → 放行原生（打字撤销）；**打字即清空本栈**
// （"按删除 undo 删除、打字 undo 打字"：打字后 Ctrl+Z 交回原生，不回退到旧结构）。

/** 结构性操作快照栈（上限 30；浅拷贝块对象——_key 一并保留，还原时不重挂 DOM） */
const undoStack: Array<LocalBlock[]> = [];
const UNDO_LIMIT = 30;

/** 结构性操作**前**入栈（拆块/合并/删块/插图/多行粘贴/选区删除） */
function pushUndo(): void {
  undoStack.push(blocks.value.map((b) => ({ ...b })));
  if (undoStack.length > UNDO_LIMIT) undoStack.shift();
}

/** 打字即清空结构性栈（原生接管；见段首语义） */
function dropUndo(): void {
  if (undoStack.length > 0) undoStack.length = 0;
}

/** 撤销一次结构性操作（Ctrl+Z；栈空由调用方放行原生） */
function undoStructural(): void {
  const snapshot = undoStack.pop();
  if (!snapshot) return;
  clearSel();
  selectedKey.value = "";
  blocks.value = snapshot.map((b) => ({ ...b }));
  void nextTick(() => autosizeAll());
  scheduleSave();
}

/** 服务端块列表 → **就地映射回既有本地块**（按 id 保 `_key`：DOM 不重挂、原生 undo
 * 栈不丢）；仅真正新增的块取新 key（PATCH002.14 外科式打补丁——整体替换会重建全部
 * textarea 的 DOM，"贴图后连打字也 undo 不了"的根因） */
function reconcileLocal(list: NoteBlock[]): LocalBlock[] {
  const byId = new Map<number, LocalBlock>();
  for (const b of blocks.value) if (b.id > 0) byId.set(b.id, b);
  return list.map((b) => {
    const existing = b.id > 0 ? byId.get(b.id) : undefined;
    if (!existing) return { ...b, _key: localKey() };
    existing.kind = b.kind;
    existing.text = b.text;
    existing.image_path = b.image_path;
    return existing;
  });
}

// —— 跨块选区（PATCH002.15/.16/.17：Ctrl+A 全选 + 鼠标跨块框选 + 复制/删除三段拼接）——
// 每块是独立 textarea、浏览器选区不跨元素 → 自绘：端点模型（块下标 + 块内字符偏移）+
// 覆盖层按字符区间画高亮（首尾块区间、中间整块）+ 复制/删除走三段拼接。

/** 选区端点（块下标 + 块内字符偏移） */
interface SelPoint {
  index: number;
  offset: number;
}

/** 拖选锚点 / 焦点（原始方向）；任一为 null = 无选区 */
const selAnchor = ref<SelPoint | null>(null);
const selFocus = ref<SelPoint | null>(null);
/** 鼠标拖选中（覆盖层临时接收指针事件供命中测试） */
const selDragging = ref(false);
/** 已进入跨块（覆盖层接管指针事件；单块内一律交原生） */
const crossSelecting = ref(false);

/** 规范化选区（排序后）；**跨块才成立**——单块内选择仍走原生 */
const selRange = computed<{ from: SelPoint; to: SelPoint } | null>(() => {
  const a = selAnchor.value;
  const f = selFocus.value;
  if (!a || !f || a.index === f.index) return null;
  const forward = a.index < f.index || (a.index === f.index && a.offset <= f.offset);
  return forward ? { from: a, to: f } : { from: f, to: a };
});

/** 块内选区区间（供高亮与拼接）；null = 该块未入选 */
function blockSelRange(i: number): { start: number; end: number } | null {
  const r = selRange.value;
  if (!r || i < r.from.index || i > r.to.index) return null;
  const block = blocks.value[i];
  if (!block) return null;
  const len = block.kind === "Text" ? block.text.length : 0;
  const start = i === r.from.index ? r.from.offset : 0;
  const end = i === r.to.index ? r.to.offset : len;
  return { start: Math.max(0, Math.min(start, len)), end: Math.max(0, Math.min(end, len)) };
}

/** 高亮层三段（head/mark/tail）；未入选时 head = 全文（覆盖层始终可命中测试） */
function selParts(i: number): { head: string; mark: string; tail: string } {
  const block = blocks.value[i];
  const text = block && block.kind === "Text" ? block.text : "";
  const seg = blockSelRange(i);
  if (!seg) return { head: text, mark: "", tail: "" };
  return {
    head: text.slice(0, seg.start),
    mark: text.slice(seg.start, seg.end),
    tail: text.slice(seg.end),
  };
}

/** 清选区（任一互斥态进入即调：收板 / 图块单击 / 拖拽起拖 / 板内点空白 / 打字） */
function clearSel(): void {
  selAnchor.value = null;
  selFocus.value = null;
  selDragging.value = false;
  crossSelecting.value = false;
}

/** 全选所有块（Ctrl+A 板级拦截；仅一块时等价原生，直接放行） */
function selectAllBlocks(): void {
  const last = blocks.value.length - 1;
  if (last <= 0) return;
  selectedKey.value = ""; // 与图块单击选中态互斥
  selAnchor.value = { index: 0, offset: 0 };
  selFocus.value = { index: last, offset: blocks.value[last].text.length };
}

/** 选区文本（三段拼接：起点块尾段 + 中间整块 + 终点块首段；图块按占位文案） */
function selectionText(): string {
  const r = selRange.value;
  if (!r) return "";
  const parts: string[] = [];
  for (let i = r.from.index; i <= r.to.index; i += 1) {
    const block = blocks.value[i];
    const seg = blockSelRange(i);
    if (!block || !seg) continue;
    parts.push(
      block.kind === "Text" ? block.text.slice(seg.start, seg.end) : block.text || "🖼 图片",
    );
  }
  return parts.join("\n");
}

/** 复制选区到系统剪贴板（跨块时接管 Ctrl+C；失败落 console，不吞） */
async function copySelection(): Promise<void> {
  const text = selectionText();
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch (err) {
    console.error("复制选区失败", err);
  }
}

/** 删除选区（三段拼接的删除：起点块留前段 + 接终点块后段，中间块整体删除；
 * 起点块对象复用 → 保 `_key` 不重挂） */
function deleteSelection(): void {
  const r = selRange.value;
  if (!r) return;
  const first = blocks.value[r.from.index];
  const last = blocks.value[r.to.index];
  if (!first || !last) return;
  pushUndo();
  const head = first.kind === "Text" ? first.text.slice(0, r.from.offset) : "";
  const tail = last.kind === "Text" ? last.text.slice(r.to.offset) : "";
  first.kind = "Text";
  first.text = head + tail;
  first.image_path = null;
  blocks.value.splice(r.from.index + 1, r.to.index - r.from.index);
  clearSel();
  void nextTick(() => autosizeAll());
  void normalizeAndFlush();
}

/** 命中测试取块内字符偏移（焦点端精度）：借覆盖层文本节点（内容 = 该块文本，
 * 段上有 data-sel-base 基准）→ 失败返回 null 由调用方降级 */
function offsetFromPoint(x: number, y: number): SelPoint | null {
  const pos = document.caretPositionFromPoint?.(x, y);
  const node = pos?.offsetNode ?? null;
  if (!node || node.nodeType !== Node.TEXT_NODE) return null;
  const host = (node.parentElement as HTMLElement | null)?.closest<HTMLElement>(
    ".note-block-sel-overlay",
  );
  if (!host) return null;
  const index = Number(host.dataset.selIndex);
  const base = Number((node.parentElement as HTMLElement | null)?.dataset.selBase ?? NaN);
  if (!Number.isInteger(index) || Number.isNaN(base)) return null;
  return { index, offset: base + (pos?.offset ?? 0) };
}

/** 拖选焦点落点：优先命中测试；失败按"越过块中线"取首/尾（降级不失效） */
function focusPointAt(x: number, y: number): SelPoint | null {
  const el = document.elementFromPoint(x, y)?.closest<HTMLElement>(".note-block");
  if (!el) return null;
  const index = Number(el.dataset.rowId) - 1;
  if (!Number.isInteger(index) || index < 0) return null;
  const hit = offsetFromPoint(x, y);
  if (hit && hit.index === index) return hit;
  const block = blocks.value[index];
  const len = block?.kind === "Text" ? block.text.length : 0;
  const rect = el.getBoundingClientRect();
  return { index, offset: y < rect.top + rect.height / 2 ? 0 : len };
}

/** 拖选源块下标（文本块 mousedown 记；-1 = 无） */
let selDragIndex = -1;

/** 文本块 mousedown：记拖源块 + 清互斥态（选区/图块选中） */
function onTextMouseDown(index: number): void {
  selDragIndex = index;
  clearSel();
  selDragging.value = true;
  if (selectedKey.value) selectedKey.value = "";
}

/** 文档级 mousemove：指针越出拖源块即进跨块模式（锚点 = 拖源 textarea 的选区起点） */
function onDocMouseMove(e: MouseEvent): void {
  if (selDragIndex < 0) return;
  const src = blocks.value[selDragIndex];
  if (!src || src.kind !== "Text") return;
  const point = focusPointAt(e.clientX, e.clientY);
  if (!point || point.index === selDragIndex) return;
  if (!selAnchor.value) {
    const el = textEls.get(src._key);
    selAnchor.value = { index: selDragIndex, offset: el?.selectionStart ?? 0 };
  }
  crossSelecting.value = true;
  selFocus.value = point;
}

/** 文档级 mouseup：结束拖源（选区保留，供 Ctrl+C / Delete） */
function onDocMouseUp(): void {
  selDragIndex = -1;
  selDragging.value = false;
  crossSelecting.value = false;
}

// —— 拖拽调序（**仅图块可拖**；机制复用 useDragReorder，落点提交走 note_reorder）——

installNoteDragHooks({
  onDown: () => {
    clearSel(); // 互斥（PATCH002.17）：图块长按拖拽优先，起拖即清选区
    clearTimeout(saveTimer);
    void saveBlocks();
  },
  commit: (slots) => {
    const todo = props.todo;
    if (todo == null) return;
    const bySlot = new Map(blocks.value.map((b, i) => [i + 1, b]));
    const reordered = slots
      .map((slot) => bySlot.get(slot))
      .filter((b): b is LocalBlock => b != null);
    if (reordered.length !== blocks.value.length) return;
    blocks.value = reordered;
    savedSignature = signature(blocks.value);
    void invoke("note_reorder", { todoId: todo.id, ids: reordered.map((b) => b.id) }).catch((err) =>
      console.error("详情块重排失败", err),
    );
  },
  rerender: () => {
    const todo = props.todo;
    if (todo) void loadBlocks(todo.id);
  },
  forceRemount: () => {
    // 块列表就地重建（拖拽重挂后 vnode↔DOM 断链修复）：换 key 强制刷新 DOM
    blocks.value = toLocal(blocks.value);
    void nextTick(() => autosizeAll());
  },
});

// —— bubble 模式（图片/文本只读，沿用 PL024/PL025）——

/** 解析图片 src（单点收敛）：data:/asset:/http(s) 直用，否则转 asset 协议 URL */
function resolveImageSrc(value: string): string {
  if (value.startsWith("data:") || value.startsWith("asset:") || value.startsWith("http")) {
    return value;
  }
  return convertFileSrc(value);
}

/** 图片加载失败（asset 协议取不到/文件缺失）→ 失效态 */
function onImageError(): void {
  imageError.value = "文件已移动或删除，请删除气泡";
}

/** 双击详情图 → 系统默认程序打开原图（PL025，ShellExecuteW） */
async function onImageDblClick(): Promise<void> {
  const b = props.bubble;
  if (!b) return;
  try {
    await invoke("bubble_open_image", { id: b.id });
  } catch (err) {
    console.error("打开原图失败", err);
  }
}

/** 删除失效图片气泡（失效态删除按钮）：删条目 + 上抛变更 + 收板 */
async function onDeleteImageBubble(): Promise<void> {
  const b = props.bubble;
  if (!b) return;
  try {
    await invoke("bubble_remove", { id: b.id });
    emit("changed");
    close();
  } catch (err) {
    console.error("删除失效图片气泡失败", err);
  }
}

const isOpen = ref(false);

// 帘联动注册（FIX003.7）：详情开合进 useVeils 全局读取器（按容器 id=detail-note 特判）
bindOverlayState("detail", () => isOpen.value);

// 板内整板阅读 + 玻璃滑杆：挂**内层滚动容器**（PATCH002.11 内外分层；id 沿用 detail-note → 帘特判零改动）
const scrollEl = ref<HTMLElement | null>(null);
interface DetailKit {
  boardRead: { sync: () => void; settle: () => void; destroy: () => void };
  glassBar: { sync: () => void; destroy: () => void };
}
let kit: DetailKit | null = null;

/** 挂/卸套件（幂等：关板即卸——FIX013.5 显式清理先例）。
 * PATCH002.10/.11：套件挂**内层滚动容器**（mask 只作用内容，不碰外壳的描边/落影）；
 * `rowSel` **不传** → `settle()` 找不到行 → 回落 `.board-read` 默认 `--fade-btm`
 * （**固定 14px 软边**，不再有"跨底缘扩大"的半截块溶解动效） */
function syncKit(): void {
  kit?.boardRead.destroy();
  kit?.glassBar.destroy();
  kit = null;
  if (!isOpen.value || !scrollEl.value) return;
  kit = {
    boardRead: useBoardRead(scrollEl.value, { gate: () => isOpen.value, layout: true }),
    // 气泡模式：滑杆在玻璃内部但本板有内外板 → 轨道按**内层容器的 inset** 定上下（用户定案）；
    // 清单（todo）模式的视觉逻辑已正确（见 y.problems #10），故**不传 inset**、保持原样
    glassBar: useGlassBar(scrollEl.value, {
      anchor: overlay.value ?? undefined,
      right: 4,
      inset: isBubbleMode.value,
    }),
  };
}

/** 开板：揭示动画几何注入（页签顶实测上扩 4px；飞出原点 = 被点行中心——A3） */
function open(): void {
  const ov = overlay.value;
  if (!ov) return;
  const boardEl = document.getElementById("board");
  const tabsEl = document.querySelector(".tabs");
  if (boardEl && tabsEl) {
    const cr = boardEl.getBoundingClientRect();
    const tr = tabsEl.getBoundingClientRect();
    ov.style.top = `${Math.max(0, tr.top - cr.top - 4)}px`;
  }
  isOpen.value = true;
  syncVeils();
  // 首帧挂载板内组件（滚动容器出现后才有滚动几何）+ 450ms 后 settle 重算
  void nextTick(() => {
    if (props.anchor) {
      const or = ov.getBoundingClientRect();
      ov.style.setProperty("--origin-x", `${Math.round(props.anchor.x - or.left)}px`);
      ov.style.setProperty("--origin-y", `${Math.round(props.anchor.y - or.top)}px`);
    }
    syncKit();
    autosizeAll();
    autosizeBubble(); // PATCH002.27：气泡全文板只读 textarea 同步增高（不自己滚）
    // 滚动复位（A4 = design resetDetailScroll）：换内容必归零
    const shell = scrollEl.value;
    if (shell) {
      shell.scrollTop = 0;
      shell.dispatchEvent(new Event("scroll"));
    }
    window.setTimeout(() => {
      if (!isOpen.value) return; // 早关板防污染
      kit?.boardRead.settle();
      kit?.glassBar.sync();
    }, 450);
  });
}

/** 关板：flush 未决保存 + 摘 open 类 + 帘布复位 + 卸套件 */
function close(): void {
  if (!isOpen.value) return; // 幂等守卫：已关再调不重复 flush/emit（FIX003.10⑧）
  clearSel(); // 选区随板关闭复位（PATCH002.16）
  void flushPending();
  isOpen.value = false;
  syncKit();
  syncVeils();
  emit("close");
}

// 数据源变化驱动开合与草稿装载（todo / bubble 双模式互斥）
watch(
  () => props.todo,
  (t) => {
    if (t) {
      void flushPending(); // 切源先 flush 旧条目未决保存（快照写回旧 id，FIX003.4）
      bubbleImgSrc.value = ""; // 图片残留清防串板
      imageError.value = "";
      titleDraft.value = t.text;
      blocks.value = [];
      savedSignature = "";
      savedTodoId = null;
      brokenKeys.value = [];
      selectedKey.value = "";
      void loadBlocks(t.id);
      open();
    } else if (!props.bubble) {
      close();
    }
  },
);

watch(
  () => props.bubble,
  (b) => {
    if (b) {
      void flushPending(); // todo→bubble 切源同款先 flush（FIX003.4）
      if (b.kind === "Image") {
        // PL025 图片气泡：板面 = 落盘图片（按需取绝对路径 → asset 协议直读）
        bubbleText.value = "";
        bubbleImgSrc.value = "";
        imageError.value = "";
        void invoke<string>("bubble_get_image_path", { id: b.id })
          .then((path) => {
            if (props.bubble?.id === b.id) bubbleImgSrc.value = resolveImageSrc(path);
          })
          .catch((err) => {
            console.error("气泡图片取数失败", err);
            if (props.bubble?.id === b.id) {
              imageError.value = "文件已移动或删除，请删除气泡";
            }
          });
      } else {
        bubbleImgSrc.value = "";
        imageError.value = "";
        bubbleText.value = b.text; // 气泡全文只读展示
      }
      open();
    } else if (!props.todo) {
      bubbleImgSrc.value = "";
      imageError.value = "";
      bubbleText.value = "";
      close();
    }
  },
);

// PATCH002.27：气泡全文就位后重算只读 textarea 高度（不自己滚，高度随内容）
watch(bubbleText, () => void nextTick(() => autosizeBubble()));

// 图片按需拉取异步到达（PL024.8c）：就位后再补一次 settle（显隐/几何重算）
watch(bubbleImgSrc, () => {
  void nextTick(() => {
    syncKit();
    window.setTimeout(() => {
      if (!isOpen.value) return;
      kit?.boardRead.settle();
      kit?.glassBar.sync();
    }, 450);
  });
});

// —— 标题改名（maxlength 12，input 实时同步回清单——debounce 300ms 合并 IPC）。
// bubble 模式只读：head 整个隐藏无 input，此 watch 天然不触发 ——
// FIX003.4：待保存快照 = { 触发时刻 id + 草稿 }，回调与 flush 只用快照、不回读 props.todo

/** 改名待保存快照（null = 无未决保存） */
let renamePending: { id: number; text: string } | null = null;
let renameTimer: number | undefined;
/** 最近一次成功落库的标题基准（FIX007.5 基准机制 + FIX008.2 带条目 id） */
let lastSavedText: { id: number; text: string } | null = null;
watch(titleDraft, (text) => {
  const todo = props.todo;
  const savedBase =
    lastSavedText != null && lastSavedText.id === todo?.id
      ? lastSavedText.text
      : (todo?.text ?? null);
  if (todo == null || text === savedBase) {
    clearTimeout(renameTimer);
    renamePending = null;
    return;
  }
  clearTimeout(renameTimer);
  renamePending = { id: todo.id, text };
  renameTimer = window.setTimeout(() => {
    const pending = renamePending;
    renamePending = null;
    if (pending == null) return;
    void invoke("todo_rename", { id: pending.id, text: pending.text })
      .then(() => {
        if (props.todo?.id === pending.id) {
          lastSavedText = { id: pending.id, text: pending.text };
        }
        emit("changed");
      })
      .catch((err) => {
        console.error("改名失败", err);
      });
  }, 300);
});

/** 立即保存未决草稿（关板/切源/卸载的 flush——保存不丢弃，FIX003.4）；快照自带
 * id，切换数据源后旧条目的草稿仍写回旧 id */
async function flushPending(): Promise<void> {
  clearTimeout(renameTimer);
  clearTimeout(saveTimer);
  const rename = renamePending;
  renamePending = null;
  const jobs: Array<Promise<void>> = [];
  if (rename != null) {
    jobs.push(
      invoke("todo_rename", { id: rename.id, text: rename.text })
        .then(() => {
          if (props.todo?.id === rename.id) {
            lastSavedText = { id: rename.id, text: rename.text };
          }
          emit("changed");
        })
        .catch((err) => {
          console.error("改名失败", err);
        }),
    );
  }
  // 详情块未决保存（PL027）：仅在 todo 模式、块已载入本条目、且有变更时落库（保存点先归一化）
  if (
    props.todo != null &&
    savedTodoId === props.todo.id &&
    signature(blocks.value) !== savedSignature
  ) {
    jobs.push(normalizeAndFlush());
  }
  await Promise.all(jobs);
}

/** 视口尺寸变化后重算自动增高（PATCH002.27）：宽度一变文本就重排 → 不重算会把文字截掉
 * （textarea 已不自己滚，高度必须跟着内容走） */
function onViewportResize(): void {
  autosizeAll();
  autosizeBubble();
}

// 板外收板（design 定案语义）+ 板内点非文本块清选中/清选区（PATCH002）
function onGlobalDown(e: MouseEvent): void {
  if (!isOpen.value) return;
  const ov = overlay.value;
  const target = e.target as HTMLElement | null;
  if (ov && ov.contains(e.target as Node)) {
    if (selectedKey.value && !target?.closest(".note-block-image")) selectedKey.value = "";
    // 文本块 mousedown 由 onTextMouseDown 自处理（拖选锚点）；其余位置一律清选区
    if (!target?.closest(".note-block-text")) clearSel();
    return;
  }
  close();
}
window.addEventListener("mousedown", onGlobalDown, true);
window.addEventListener("keydown", onWindowKeydown, true);
window.addEventListener("mousemove", onDocMouseMove, true);
window.addEventListener("mouseup", onDocMouseUp, true);
window.addEventListener("resize", onViewportResize);

onBeforeUnmount(() => {
  void flushPending();
  kit?.boardRead.destroy(); // 卸载即卸套件（FIX013.5 显式清理）
  kit?.glassBar.destroy();
  kit = null;
  caretMirror?.remove(); // PATCH002.9：镜像元素一并清理
  caretMirror = null;
  window.removeEventListener("mousedown", onGlobalDown, true);
  window.removeEventListener("keydown", onWindowKeydown, true);
  window.removeEventListener("mousemove", onDocMouseMove, true);
  window.removeEventListener("mouseup", onDocMouseUp, true);
  window.removeEventListener("resize", onViewportResize);
});
</script>

<template>
  <div ref="overlay" class="detail-overlay" :class="{ open: isOpen, 'bubble-mode': isBubbleMode }">
    <div class="board-glass detail-glass">
      <div v-if="!isBubbleMode" class="detail-head">
        <span class="detail-label">标题</span>
        <input
          v-model="titleDraft"
          class="detail-title"
          :maxlength="UI_TEXT_MAX_LEN"
          placeholder="最多 12 字"
        />
      </div>
      <!-- 滚动容器：id 沿用 detail-note（帘特判与板内套件零改动） -->
      <!-- PATCH002.11 内外分层：外壳只做圆角/描边/落影；套件挂**内层**（mask 不吃壳装饰） -->
      <!-- PATCH002.27：`glass-scroll`/`board-read` 写进**静态 class**（沿 #todo-active / #bubble-list
           先例）——关板时 `isBubbleMode` 翻转会触发 Vue 按 vdom 整体重写 class，抹掉套件运行时
           add 的类（原生滑杆现形、遮罩丢失）；静态 + 动态合并后 patch 两者恒在 -->
      <div class="note-shell">
        <div
          ref="scrollEl"
          id="detail-note"
          class="note-scroll glass-scroll board-read"
          :class="{
            'bubble-shell': isBubbleMode,
            'image-shell': isBubbleMode && bubbleImgSrc !== '',
            'note-blocks': !isBubbleMode,
          }"
          @focusout="onBlocksFocusOut"
        >
          <!-- bubble 模式：图片 / 全文只读（PL024/PL025 原样） -->
          <template v-if="isBubbleMode">
            <img
              v-if="bubbleImgSrc && !imageError"
              class="detail-image"
              :src="bubbleImgSrc"
              alt="气泡图片"
              @error="onImageError"
              @dblclick="onImageDblClick"
            />
            <div v-else-if="imageError" class="detail-image-broken">
              <p class="detail-image-broken-text">{{ imageError }}</p>
              <button class="detail-image-broken-del" @click="onDeleteImageBubble">删除气泡</button>
            </div>
            <textarea
              v-else
              ref="bubbleTextEl"
              class="detail-note bubble-note"
              :value="bubbleText"
              readonly
              spellcheck="false"
            ></textarea>
          </template>
          <!-- todo 模式：块序列（一段一行一块 + 图块混排） -->
          <template v-else>
            <div
              v-for="(block, i) in blocks"
              :key="block._key"
              class="note-block"
              :class="{
                'note-block-image': block.kind === 'Image',
                'note-block-selected': selectedKey === block._key,
                'note-block-leaving': leavingKey === block._key,
                'note-block-sel': block.kind === 'Image' && blockSelRange(i) !== null,
              }"
              :data-row-id="i + 1"
              :tabindex="block.kind === 'Image' ? 0 : undefined"
              @click="onBlockContainerClick(block)"
              @keydown="onBlockContainerKeydown(i, $event)"
            >
              <template v-if="block.kind === 'Image'">
                <img
                  v-if="!brokenKeys.includes(block._key)"
                  class="note-block-img"
                  :src="blockSrc(block)"
                  alt="详情图片"
                  draggable="false"
                  @click="selectedKey = block._key"
                  @dblclick="openBlockImage(block)"
                  @error="onBlockImageError(block)"
                />
                <p v-else class="note-block-broken">图片文件已移动或删除</p>
                <button
                  class="note-block-del"
                  type="button"
                  title="删除图片"
                  @click.stop="deleteBlock(i)"
                >
                  ×
                </button>
              </template>
              <textarea
                v-else
                :ref="(el) => setTextEl(block._key, el)"
                v-model="block.text"
                class="note-block-text"
                :placeholder="i === 0 ? '添加描述、清单、想法…' : ''"
                spellcheck="false"
                rows="1"
                @mousedown="onTextMouseDown(i)"
                @input="onBlockInput"
                @keydown="onBlockKeydown(i, $event)"
                @paste="onBlockPaste(i, $event)"
              ></textarea>
              <!-- 跨块选区高亮层（PATCH002.16/.17）：文本透明只显底色；拖选期间接管指针事件 -->
              <div
                v-if="block.kind === 'Text' && (selDragging || selRange)"
                class="note-block-sel-overlay"
                :class="{ 'sel-live': crossSelecting }"
                :data-sel-index="i"
                aria-hidden="true"
              >
                <span :data-sel-base="0">{{ selParts(i).head }}</span
                ><span class="sel-mark" :data-sel-base="selParts(i).head.length">{{
                  selParts(i).mark
                }}</span
                ><span :data-sel-base="selParts(i).head.length + selParts(i).mark.length">{{
                  selParts(i).tail
                }}</span>
              </div>
            </div>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@import "../styles/detail.css";
</style>
