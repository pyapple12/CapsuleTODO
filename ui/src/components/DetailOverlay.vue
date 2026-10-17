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
/** 气泡图片可渲染 src（PL025 载荷落盘化：开板按需 bubble_get_image_path 取绝对路径
 * → convertFileSrc 生成 asset URL；mock 环境返回 data URL 直用） */
const bubbleImgSrc = ref("");
/** 图片失效态文案（PL025：文件缺失 → 提示删除气泡；空串 = 正常） */
const imageError = ref("");
/** 保存失败可见反馈（FIX007.7，对齐 AddBar 错误行模式；成功/新输入即清） */
const saveError = ref("");

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

/** 全部文本域重算高度（装载/重排后） */
function autosizeAll(): void {
  textEls.forEach((el) => autosize(el));
}

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
  saveError.value = "";
  clearTimeout(saveTimer);
  const todo = props.todo;
  if (todo == null) return;
  if (signature(blocks.value) === savedSignature) return;
  saveTimer = window.setTimeout(() => {
    void saveBlocks();
  }, 300);
}

/** 立即落库（防抖到点 / 关板 / 切源 / 粘贴前调用）；失败红字可见。
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
    if (props.todo?.id === todo.id) saveError.value = `详情保存失败：${String(err)}`;
    console.error("详情保存失败", err);
  }
}

/** 载入该条目的块（开板/切条目/收场重拉） */
async function loadBlocks(todoId: number): Promise<void> {
  try {
    const list = await invoke<NoteBlock[]>("note_blocks_list", { todoId });
    if (props.todo?.id !== todoId) return;
    blocks.value = toLocal(list);
    savedTodoId = todoId;
    savedSignature = signature(blocks.value);
    brokenKeys.value = [];
    void nextTick(() => autosizeAll());
  } catch (err) {
    console.error("详情块拉取失败", err);
    saveError.value = `详情载入失败：${String(err)}`;
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

/** 文本块输入：重算高度 + 排保存（v-model 已写入模型） */
function onBlockInput(e: Event): void {
  autosize(e.target as HTMLTextAreaElement);
  scheduleSave();
}

/** 光标处拆块（Enter）：前段留原块、后段新块，光标落新块首 */
function splitBlock(index: number, at: number): void {
  const block = blocks.value[index];
  if (!block) return;
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
  await saveBlocks();
}

/** 文本块键盘：Enter 拆块 / Ctrl+Enter 保存关板 / Backspace@块首（合并或删上图）；
 * Delete 不拦截（只作用本块内，永不涉及下方块）；全程 IME 守卫 */
function onBlockKeydown(index: number, e: KeyboardEvent): void {
  if (e.isComposing) return;
  const el = e.target as HTMLTextAreaElement;
  const block = blocks.value[index];
  if (!block) return;
  if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    void saveAndClose();
    return;
  }
  if (e.key === "Enter" && !e.shiftKey && !e.altKey) {
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
  }
}

/** 保存未决草稿并关板（Ctrl+Enter；**仅文本块响应**，其余输入面一律不响应） */
async function saveAndClose(): Promise<void> {
  clearTimeout(saveTimer);
  await saveBlocks();
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

/** 插图：先 flush 未决保存（把 live 文本交上去）→ Rust 落盘/复用 + 切分插入 → 整体替换本地态 */
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
      blocks.value = toLocal(outcome.blocks);
      savedSignature = signature(blocks.value);
      brokenKeys.value = [];
      void nextTick(() => autosizeAll());
      emit("changed");
    } else {
      // 剪贴板无图（或非图片文件）→ 回落文本粘贴（含多行拆块），不吞粘贴
      pasteTextLines(index, start, end, fallbackText);
    }
  } catch (err) {
    if (props.todo?.id === todo.id) saveError.value = `插入图片失败：${String(err)}`;
    console.error("插入图片失败", err);
  }
}

/** 多行文本粘贴：按行拆成多块（保持"一段一行一块"不变量）；单行不走此处（原生粘贴） */
function pasteTextLines(index: number, start: number, end: number, text: string): void {
  const block = blocks.value[index];
  if (!block) return;
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

/** 双击图块 → 系统默认程序打开**原图**（跳过预览） */
async function openBlockImage(block: LocalBlock): Promise<void> {
  if (block.id <= 0) return;
  try {
    await invoke("note_open_image", { blockId: block.id });
  } catch (err) {
    console.error("打开原图失败", err);
    saveError.value = `打开原图失败：${String(err)}`;
  }
}

/** 图块键盘（选中态）：Backspace / Delete 均删 */
function onImageKeydown(index: number, e: KeyboardEvent): void {
  if (e.isComposing) return;
  if (e.key === "Backspace" || e.key === "Delete") {
    e.preventDefault();
    void deleteBlock(index);
  }
}

// —— 拖拽调序（**仅图块可拖**；机制复用 useDragReorder，落点提交走 note_reorder）——

installNoteDragHooks({
  onDown: () => {
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
    saveError.value = `打开原图失败：${String(err)}`;
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
    saveError.value = `删除失败：${String(err)}`;
  }
}

const isOpen = ref(false);

// 帘联动注册（FIX003.7）：详情开合进 useVeils 全局读取器（按容器 id=detail-note 特判）
bindOverlayState("detail", () => isOpen.value);

// 板内整板阅读 + 玻璃滑杆：挂**滚动容器**（id 沿用 detail-note → 帘特判与套件零改动）
const shellEl = ref<HTMLElement | null>(null);
interface DetailKit {
  boardRead: { sync: () => void; settle: () => void; destroy: () => void };
  glassBar: { sync: () => void; destroy: () => void };
}
let kit: DetailKit | null = null;

/** 挂/卸套件（幂等：关板即卸——FIX013.5 显式清理先例） */
function syncKit(): void {
  kit?.boardRead.destroy();
  kit?.glassBar.destroy();
  kit = null;
  if (!isOpen.value || !shellEl.value) return;
  kit = {
    boardRead: useBoardRead(shellEl.value, {
      gate: () => isOpen.value,
      layout: true,
      rowSel: ".note-block",
    }),
    glassBar: useGlassBar(shellEl.value, { anchor: overlay.value ?? undefined, right: 4 }),
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
    // 滚动复位（A4 = design resetDetailScroll）：换内容必归零
    const shell = shellEl.value;
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
  saveError.value = ""; // 新输入即清（FIX007.7）
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
        if (props.todo?.id === pending.id) {
          saveError.value = `改名保存失败：${String(err)}`;
        }
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
          if (props.todo?.id === rename.id) {
            saveError.value = `改名保存失败：${String(err)}`;
          }
          console.error("改名失败", err);
        }),
    );
  }
  // 详情块未决保存（PL027）：仅在 todo 模式、块已载入本条目、且有变更时落库
  if (
    props.todo != null &&
    savedTodoId === props.todo.id &&
    signature(blocks.value) !== savedSignature
  ) {
    jobs.push(saveBlocks());
  }
  await Promise.all(jobs);
}

// 板外收板（design 定案语义）：mousedown 落在 overlay 外即收板
function onGlobalDown(e: MouseEvent): void {
  if (!isOpen.value) return;
  const ov = overlay.value;
  if (ov && !ov.contains(e.target as Node)) close();
}
window.addEventListener("mousedown", onGlobalDown, true);

onBeforeUnmount(() => {
  void flushPending();
  kit?.boardRead.destroy(); // 卸载即卸套件（FIX013.5 显式清理）
  kit?.glassBar.destroy();
  kit = null;
  window.removeEventListener("mousedown", onGlobalDown, true);
});
</script>

<template>
  <div ref="overlay" class="detail-overlay" :class="{ open: isOpen, 'bubble-mode': isBubbleMode }">
    <div class="board-glass detail-glass">
      <!-- FIX007.7 保存失败可见反馈（对齐 AddBar 错误行模式） -->
      <p v-if="saveError" class="detail-error">{{ saveError }}</p>
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
      <div
        ref="shellEl"
        id="detail-note"
        class="note-shell"
        :class="{
          'bubble-shell': isBubbleMode,
          'image-shell': isBubbleMode && bubbleImgSrc !== '',
          'note-blocks': !isBubbleMode,
        }"
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
            }"
            :data-row-id="i + 1"
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
              <span
                class="note-block-hit"
                tabindex="0"
                @keydown="onImageKeydown(i, $event)"
                @focus="selectedKey = block._key"
              ></span>
            </template>
            <textarea
              v-else
              :ref="(el) => setTextEl(block._key, el)"
              v-model="block.text"
              class="note-block-text"
              placeholder="添加描述、清单、想法…"
              spellcheck="false"
              rows="1"
              @input="onBlockInput"
              @keydown="onBlockKeydown(i, $event)"
              @paste="onBlockPaste(i, $event)"
            ></textarea>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
@import "../styles/detail.css";
</style>
