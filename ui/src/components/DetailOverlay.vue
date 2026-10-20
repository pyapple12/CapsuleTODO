<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type { BubbleItem, NoteBlock, NotePasteOutcome, TodoItem } from "../../types";
import { UI_TEXT_MAX_LEN } from "../../types";
import {
  normalizeBlocks as normalizeBlankBlocks,
  type LocalBlock,
} from "../composables/normalizeBlocks";
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
// 本地块类型 `LocalBlock` 与空块归一化统一在 `composables/normalizeBlocks.ts`（PATCH002.75）

const blocks = ref<LocalBlock[]>([]);
/** 整篇是否为空（全部文本块 trim 后皆空）——占位文案**仅此时**显示（PATCH002.23）：
 * 多块且不全空时首块留空不再冒"添加描述、清单、想法…" */
const noteEmpty = computed(() =>
  blocks.value.every((b) => b.kind !== "Text" || b.text.trim() === ""),
);
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

/** 最近一次获得焦点的文本块 key（PATCH002.77：焦点已丢时提升原生选区锚点的兜底） */
let lastTextKey = "";

/** 文本块获得焦点：记下最近聚焦块（PATCH002.77） */
function onTextFocus(key: string): void {
  lastTextKey = key;
}

// —— 临时诊断（PATCH002.44 打点，测完即删）已按纪律全清 ——

/** 浏览器原生按内容定高（Chromium 123+）：可用时**不做** JS 增高——内联 `height` 会压过它。
 * PATCH002.44：从根上杀掉 `height="auto"` 的"塌一下"中间态（滚动跳动的触发源之一） */
const NATIVE_FIELD_SIZING =
  typeof CSS !== "undefined" && typeof CSS.supports === "function"
    ? CSS.supports("field-sizing", "content")
    : false;

/** 自动增高（内容换行显示，高度随内容）；原生 `field-sizing` 可用时交由浏览器（no-op） */
function autosize(el: HTMLTextAreaElement): void {
  if (NATIVE_FIELD_SIZING) return;
  el.style.height = "auto";
  el.style.height = `${el.scrollHeight}px`;
}

/** 结构性变更期间**钉住视口**（PATCH002.44）：变更前记锚块屏幕位置 → 变更后把 `scrollTop`
 * 补偿同样的差值，使锚块在屏幕上**不动**；变更前已到底 → 改为维持底部（原条目 ②）。
 * `mutate` 可为异步（删块要跨塌缩动画）；`anchorEl` 缺省由调用方给"位置稳定的参照块" */
function pinViewport(
  anchorEl: HTMLElement | null,
  mutate: () => void | Promise<void>,
): Promise<void> {
  const el = scrollEl.value;
  const atBottom = el ? el.scrollTop >= el.scrollHeight - el.clientHeight - 1 : false;
  kit?.boardRead.setPinned(atBottom); // PATCH002.69：结构变更前显式声明贴底态（RO 据此 paint 前贴底）
  const before = anchorEl ? anchorEl.getBoundingClientRect().top : null;
  return Promise.resolve(mutate()).then(() =>
    nextTick(() => {
      if (!el) return;
      if (atBottom) {
        el.scrollTop = el.scrollHeight; // 到底态：维持底部（增行不把视口甩上去）
        return;
      }
      if (before == null || !anchorEl?.isConnected) return;
      const delta = anchorEl.getBoundingClientRect().top - before;
      if (delta) el.scrollTop += delta;
    }),
  );
}

/** 当前聚焦块元素（`pinViewport` 缺省锚点：光标所在块，无聚焦则 null） */
function focusedBlockEl(): HTMLElement | null {
  const active = document.activeElement as HTMLElement | null;
  return active?.closest<HTMLElement>(".note-block") ?? null;
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

/** 签名短指纹（PATCH002.72，**临时诊断专用**）：FNV-1a 32 位 → 8 位十六进制。
 * 只用于日志里比对"是不是同一代状态"，**不参与任何业务判定**；与 `pre-save`/`save`
 * 两条诊断配对读（两行指纹相同 = 服务端没改块集；不同 = 发生了归一化/裁剪）。 */
function sigFingerprint(sig: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < sig.length; i += 1) {
    h ^= sig.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h.toString(16).padStart(8, "0");
}

/** 块列表 id 短序列（**临时诊断专用**）：`T12,I0` —— 用于日志里比对"是哪一代块集"
 * （`0` = 本地未落库块）。只拼接字符串，不参与任何业务判定。 */
function idsOf(list: LocalBlock[]): string {
  return list.map((b) => `${b.kind === "Image" ? "I" : "T"}${b.id}`).join(",");
}

/** 撤销栈登记（**临时诊断专用**）：记录每次入栈时的代际 id 序列 */
function undoLog(line: string): void {
  void invoke("debug_log", { line })
    .then(() => undefined)
    .catch((err) => console.error("诊断上报失败", err));
}

/** 排入防抖保存（无变更/未开板直接跳过）。**捕获** {todoId, list} 再排定（PATCH002.47）：
 * 切源会整体换数组 → 捕获引用即"那一条目的那一份"（同条目内 v-model 原地改仍取到最新文本） */
function scheduleSave(): void {
  if (pasteInFlight > 0 || editInFlight > 0) return; // PATCH002.72 C5：粘贴窗口内不排保存（最终态由收尾统一写）
  clearTimeout(saveTimer);
  const todo = props.todo;
  if (todo == null) return;
  if (signature(blocks.value) === savedSignature) return;
  const todoId = todo.id;
  const list = blocks.value;
  saveTimer = window.setTimeout(() => {
    // 临时诊断（.72 排查）：哪个保存入口
    undoLog(`save:caller=debounce inFlight=${pasteInFlight} len=${list.length}`);
    void saveBlocksFor(todoId, list);
  }, 300);
}

/** 立即落库**指定条目**的块列表（防抖到点 / 关板 / 切源 / 粘贴前调用）；失败落 console
 * （详情板不设提示行，UI 静默）。**按入参而非当前 props 落库**——切源后仍能把旧条目的
 * 未决编辑写回旧 id（PATCH002.47）；`force` 绕开签名比对（给本地独有的 id=0 块拿 id，
 * 治"幻影块永不落库"，PATCH002.46）。
 * 守卫：该条目尚未锚定（`savedTodoId` 不符）时直接跳过——防"开板瞬间空列表被当成
 * 全量覆盖"把服务端块集清空（冒烟实证：开板即报"详情块为空"） */
async function saveBlocksFor(todoId: number, list: LocalBlock[], force = false): Promise<void> {
  if (!force && savedTodoId !== todoId) return;
  const sig = signature(list);
  if (!force && sig === savedSignature && savedTodoId === todoId) return;
  // 临时诊断（抖动定位，测完即删）：构建载荷**之前**的本地末块 + **签名指纹**
  // （PATCH002.72：指纹用于比对"这次要保存的是不是刚才那一代"）
  const preLast = list[list.length - 1];
  void invoke("debug_log", {
    line: `pre-save todo=${todoId} len=${list.length} last=${preLast ? `${preLast.kind}#${preLast.id}` : "none"} sig=${sig.length}/${sigFingerprint(sig)}`,
  }).catch((err) => console.error("诊断上报失败", err));
  const payload = list.map((b) => ({ id: b.id, kind: b.kind, text: b.text }));
  try {
    const saved = await invoke<NoteBlock[]>("note_blocks_save", { todoId, blocks: payload });
    // 仅当该列表仍是"当前列表"才回填本地状态；切源后 list 已被换掉 → 只落库、不动当前条目态
    if (blocks.value !== list) return;
    // 就地回填 id（保 _key → 不重挂 DOM，输入焦点不丢）；基准取回填后的签名
    saved.forEach((b, i) => {
      const local = list[i];
      if (!local) return;
      local.id = b.id;
      local.kind = b.kind;
      local.image_path = b.image_path;
    });
    savedTodoId = todoId;
    savedSignature = signature(list);
    // 临时诊断（抖动定位，测完即删）：送出的块数 / 服务端回来数 / **采纳后的签名指纹**
    // （PATCH002.72：与 `pre-save` 行配对读——两行指纹相同 = 服务端没改块集 ✓）
    const savedLast = saved[saved.length - 1];
    void invoke("debug_log", {
      line: `save todo=${todoId} sent=${payload.length} got=${saved.length} lastGot=${savedLast ? `${savedLast.kind}#${savedLast.id}` : "none"} sig=${savedSignature.length}/${sigFingerprint(savedSignature)}`,
    }).catch((err) => console.error("诊断上报失败", err));
    emit("changed");
  } catch (err) {
    // 临时诊断（.72 排查）：保存失败原先只落 console ⇒ 日志里完全隐身 ✗（本次就靠它才能定位）
    console.error("详情保存失败", err);
    undoLog(`save:fail todo=${todoId} len=${payload.length} err=${String(err)}`);
  }
}

/** 当前条目的立即落库（薄壳；`force` 见 `saveBlocksFor`） */
async function saveBlocks(force = false): Promise<void> {
  const todo = props.todo;
  if (todo == null) return;
  await saveBlocksFor(todo.id, blocks.value, force);
}

/** 载入该条目的块（开板/切条目/收场重拉） */
async function loadBlocks(todoId: number, ghost: HTMLElement | null = null): Promise<void> {
  try {
    const list = await invoke<NoteBlock[]>("note_blocks_list", { todoId });
    if (props.todo?.id !== todoId) {
      ghost?.remove(); // 板已切走：占位符一并清掉
      return;
    }
    const serverList = toLocal(list);
    // PATCH002.82：占位符与新列表**同一任务交接**——先撤 ghost 再赋值；浏览器只在任务结束后
    // paint，因此不会出现"新列表 + ghost"或"矮一截"的中间帧
    ghost?.remove();
    blocks.value = normalizeBlocks(serverList);
    dragDiag("loadBlocks:done"); // 临时诊断（抖动定位，测完即删）：重拉赋值后的本地列表
    savedTodoId = todoId;
    // PATCH002.56 修正：**基准必须反映服务端真实状态**（归一化**之前**的列表）——
    // 否则"库里有、本地被归一化掉"的块不算变更 → `saveBlocks` 永远跳过 → 库与本地计数
    // 永久不一致 → `reorder_note_blocks` 的 id 集合校验恒失败（重排被拒、块弹回原位）
    savedSignature = signature(serverList);
    // PATCH002.72：**撤销快照不得跨"重新载入"生效** —— 载入后 id / _key 全换，旧快照与库
    // 已无对应关系；若仍可恢复，Ctrl+Z 会把模型换回旧状态，再被保存点写回库（实证：
    // 载入 3 块 #611 → focusout 时模型变 1 块 #609 → 落库 1 块）
    const undoDepth = undoStack.length; // 临时诊断（.72 排查）：清栈前深度
    undoStack.length = 0;
    undoSeq += 1; // 在飞的删块动效收场按代际放弃（沿 .44 口径）
    // 临时诊断（.72 排查）：清栈前多深（>0 说明旧代快照确实存在过 ✓）+ 本次载入的代际
    undoLog(`undo:clear depth=${undoDepth} arr=${idsOf(blocks.value)}`);
    brokenKeys.value = [];
    lastTextKey = ""; // PATCH002.77：载入即换代，旧的"最近聚焦块"不再代表当前文档
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
    el.focus({ preventScroll: true }); // PATCH002.44：位移由 pinViewport 显式补偿，别让浏览器替我们滚
    el.setSelectionRange(caret, caret);
    autosize(el);
  });
}

/** 文本块输入：重算高度 + 排保存（v-model 已写入模型）；打字即退出选区 + 清结构性撤销栈 */
function onBlockInput(e: Event): void {
  autosize(e.target as HTMLTextAreaElement);
  if (selAnchor.value || selFocus.value) clearSel();
  // PATCH002.73：替换/组字的那一下输入**不清栈**（刚压的快照要活到撤销时 ✓）；
  // 其余输入照旧"打字即清结构性栈" ✓
  if (keepUndoComposing) {
    undoLog("undo:keep consume reason=ime");
  } else if (keepUndoOnce) {
    keepUndoOnce = false;
    if (editInFlight > 0) editInFlight -= 1; // 字符已写入 ⇒ 关闭结构性编辑窗口 ✓
    undoLog(`undo:keep consume reason=input editInFlight=${editInFlight}`);
  } else {
    dropUndo();
  }
  scheduleSave();
}

/** 光标处拆块（Enter）：前段留原块、后段新块，光标落新块首 */
function splitBlock(index: number, at: number, push = true): void {
  const block = blocks.value[index];
  if (!block) return;
  if (push) pushUndo(); // 结构性操作入栈（Ctrl+Z 可回退本次拆块）
  const created: LocalBlock = {
    id: 0,
    kind: "Text",
    text: block.text.slice(at),
    image_path: null,
    _key: localKey(),
  };
  // PATCH002.44：拆块改内容高 → 钉住视口（锚 = 光标块，其顶不变）
  void pinViewport(focusedBlockEl(), () => {
    block.text = block.text.slice(0, at);
    blocks.value.splice(index + 1, 0, created);
    scheduleSave();
  });
  focusBlock(created._key, 0);
}

/** 与上一文本块合并（Backspace@块首）：拼接 + 删本块，光标落合并点 */
function mergeWithPrev(index: number): void {
  const prev = blocks.value[index - 1];
  const cur = blocks.value[index];
  if (!prev || !cur || prev.kind !== "Text") return;
  pushUndo();
  const joinAt = prev.text.length;
  // PATCH002.44：合并改内容高 → 钉住视口（锚 = 光标块，其顶不变）
  void pinViewport(focusedBlockEl(), () => {
    prev.text += cur.text;
    blocks.value.splice(index, 1);
    scheduleSave();
  });
  focusBlock(prev._key, joinAt);
}

/** 删块动效时长（PATCH002.44：塌缩淡出 180ms） */
const DELETE_ANIM_MS = 180;

/** 撤掉塌缩内联样式（动效被放弃时把行还原） */
function clearCollapse(el: HTMLElement | null): void {
  if (!el) return;
  el.style.removeProperty("max-height");
  el.style.removeProperty("opacity");
  el.style.removeProperty("overflow");
}

/** 删除块（图块三入口共用；动效：塌缩淡出后再落库——文件引用由服务端归零释放）。
 * `caretTo`（PATCH002.37）：删后把光标交还给相邻**文本**块（"prev" 落其末尾 / "next" 落其
 * 开头）；**甲案定案**——邻居是图块则不推焦点（维持现状），无 caretTo 的入口（图块 × /
 * 选中态键盘删）也不推。
 * **动效（PATCH002.44 乙案）**：改由 **Web Animations API** 驱动（实测高 → 0 + 淡出），
 * `.finished` 才摘数据——不再依赖"Vue 打拍后才挂 `.note-block-leaving`"的时机（那会让过渡
 * 永不成立、行瞬间消失）。**动效与"视口钉住"在删除场景互斥**：有动效 → 下面内容平滑上移
 * （刻意动效，不钉）；无动效（`el.animate` 缺席的降级分支）→ 才钉住视口兜跳动。
 * **可回退**：动效期间用户撤销（`undoSeq` 变）→ 放弃收场并还原行观感，不覆盖其回退 */
async function deleteBlock(index: number, caretTo: "prev" | "next" | null = null): Promise<void> {
  const block = blocks.value[index];
  if (!block) return;
  pushUndo();
  const seq = undoSeq; // 收场前复核代际（见函数头"可回退"）
  if (selectedKey.value === block._key) selectedKey.value = "";
  const rowEl: HTMLElement | null =
    overlay.value?.querySelector<HTMLElement>(`[data-row-id="${index + 1}"]`) ?? null;
  // 锚 = 被删块的下一块（位置稳定；无下一块取上一块）——**仅供降级分支**钉视口
  const anchorRow = blocks.value[index + 1] ? index + 2 : index;
  const anchorEl =
    overlay.value?.querySelector<HTMLElement>(`[data-row-id="${anchorRow}"]`) ?? null;
  let anim: Animation | null = null;
  /** 收场：摘数据 + 交还光标 + 落库；**代际不符（用户已撤销）则放弃并还原行观感** */
  const settle = async (): Promise<void> => {
    if (undoSeq !== seq) {
      leavingKey.value = "";
      anim?.cancel(); // 撤掉 forwards 填充
      clearCollapse(rowEl);
      return;
    }
    leavingKey.value = "";
    const at = blocks.value.findIndex((b) => b._key === block._key);
    if (at < 0) return;
    blocks.value.splice(at, 1);
    // PATCH002.37（甲案）：邻居是**文本**块才交还光标；是图块则维持现状（不强推焦点）
    if (caretTo) {
      const neighbor = caretTo === "prev" ? blocks.value[at - 1] : blocks.value[at];
      if (neighbor?.kind === "Text") {
        focusBlock(neighbor._key, caretTo === "prev" ? neighbor.text.length : 0);
      }
    }
    scheduleSave();
    await saveBlocks();
  };
  leavingKey.value = block._key; // 语义标记（淡出视觉由动画 / 内联样式负责）
  if (rowEl && typeof rowEl.animate === "function") {
    // 乙案：Web Animations 驱动（与 Vue 渲染解耦；finished 精确收场，慢帧也不移位）
    rowEl.style.overflow = "hidden";
    anim = rowEl.animate(
      [
        { maxHeight: `${rowEl.offsetHeight}px`, opacity: 1 },
        { maxHeight: "0px", opacity: 0 },
      ],
      { duration: DELETE_ANIM_MS, easing: "ease", fill: "forwards" },
    );
    await anim.finished.catch((err) => {
      console.error("删除动效被中断（继续收场）", err);
    });
    await settle();
    return;
  }
  // 降级：无 Web Animations → 瞬间消失，由钉住视口兜住跳动（PATCH002.44 前行为）
  await pinViewport(anchorEl, async () => {
    if (rowEl) {
      rowEl.style.overflow = "hidden";
      rowEl.style.maxHeight = `${rowEl.offsetHeight}px`;
      void rowEl.offsetHeight; // 强制回流，确保 max-height 过渡生效
      rowEl.style.maxHeight = "0px";
      rowEl.style.opacity = "0";
    }
    await new Promise((resolve) => window.setTimeout(resolve, DELETE_ANIM_MS));
    await settle();
  });
}

/** 与下一文本块合并（Delete@块尾，PATCH002.22）：本块接上下块全文 + 删下块，光标留原位置 */
function mergeWithNext(index: number): void {
  const cur = blocks.value[index];
  const next = blocks.value[index + 1];
  if (!cur || !next || next.kind !== "Text") return;
  pushUndo();
  const caret = cur.text.length;
  // PATCH002.44：合并改内容高 → 钉住视口（锚 = 光标块，其顶不变）
  void pinViewport(focusedBlockEl(), () => {
    cur.text += next.text;
    blocks.value.splice(index + 1, 1);
    scheduleSave();
  });
  focusBlock(cur._key, caret);
}

/** 文本块键盘（PATCH002.22 语义定案：两键删的是"**分隔**"而非"删块"）：空块 →
 * Backspace/Delete 删本块自身（**首/末块保持**，不吃相邻图）；非空块**块首 Backspace** →
 * 上块文本合并 / 上块图删图；非空块**块尾 Delete**（本批新增）→ 下块文本合并上来 /
 * 下块图删掉；块中两键不拦截（原生删字符）；Ctrl+Enter 由**板级** window 监听处理；
 * 全程 IME 守卫 */
function onBlockKeydown(index: number, e: KeyboardEvent): void {
  if (e.isComposing) return;
  const el = e.target as HTMLTextAreaElement;
  const block = blocks.value[index];
  if (!block) return;
  const start = el.selectionStart ?? 0;
  const end = el.selectionEnd ?? 0;
  const collapsed = start === end; // 无选区：两键的"分隔"语义只在光标态成立
  if (e.key === "Enter" && !e.shiftKey && !e.altKey && !e.ctrlKey && !e.metaKey) {
    e.preventDefault();
    // PATCH002.73：有自绘选区 ⇒ **先删选区，再在选区起点拆块**（首块留空、光标落第二行行首 ✓，
    // 等价"空行上回车"）；全程**一次**撤销（快照取在删除前 ✓）
    if (selRange.value) {
      pushUndo();
      const at = removeSelectedRange(false);
      const afterDel = blocks.value.length;
      if (at) splitBlock(at.index, at.offset, false);
      // 临时诊断（.73 排查）：Enter 走的是哪条路 + 删/拆之后的块数
      undoLog(
        `enter:sel at=${at ? `${at.index}:${at.offset}` : "null"} afterDel=${afterDel} afterSplit=${blocks.value.length}`,
      );
      return;
    }
    undoLog(`enter:plain index=${index} start=${start}`);
    splitBlock(index, start);
    return;
  }
  // ① 空块：删掉"本块这一行"本身（首块 Backspace / 末块 Delete 无处可并 → 保持原状）；
  // PATCH002.37（甲案）：删后光标交还相邻**文本**块（Backspace → 上一文本块末尾、
  // Delete → 下一文本块首字前）；邻居是图块则不推焦点（维持现状）
  if (collapsed && block.text === "") {
    if (e.key === "Backspace" && index > 0) {
      e.preventDefault();
      void deleteBlock(index, "prev");
      return;
    }
    if (e.key === "Delete" && index < blocks.value.length - 1) {
      e.preventDefault();
      void deleteBlock(index, "next");
      return;
    }
  }
  // ② 非空块块首 Backspace：上块文本 → 合并；上块图 → 删图（图与文本不能同行 = 删分隔）
  if (e.key === "Backspace" && collapsed && start === 0) {
    const prev = blocks.value[index - 1];
    if (!prev) return;
    e.preventDefault();
    if (prev.kind === "Image") void deleteBlock(index - 1);
    else mergeWithPrev(index);
    return;
  }
  // ③ 非空块块尾 Delete（本批新增）：下块文本 → 合并上来；下块图 → 删图；末块 → 保持
  if (e.key === "Delete" && collapsed && start === el.value.length) {
    const next = blocks.value[index + 1];
    if (!next) return;
    e.preventDefault();
    if (next.kind === "Image") void deleteBlock(index + 1);
    else mergeWithNext(index);
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
  if (
    mod &&
    !e.altKey &&
    (e.key === "c" || e.key === "C") &&
    (selRange.value || selectedKey.value) // PATCH002.24：图块单击选中态也接管 Ctrl+C
  ) {
    e.preventDefault();
    void copySelection();
    return;
  }
  if ((e.key === "Delete" || e.key === "Backspace") && selRange.value) {
    e.preventDefault();
    // PATCH002.73：**只删选区**，删后**光标落到选区起点**（整篇全选 ⇒ 正好是第一块开头 ✓）
    const at = removeSelectedRange();
    if (at) focusBlock(blocks.value[at.index]?._key ?? "", at.offset);
    scheduleSave(); // 删除必须落库（原 `deleteSelection` 走 normalizeAndFlush，已随死代码移除）
    return;
  }
  // PATCH002.73（IME 前置，2026-10-20 live 修正）：**输入法的第一个键**（`keyCode === 229` /
  // `key === "Process"`，此刻 `isComposing` 仍为 false）就删选区。若等到 `compositionstart`
  // 才删，输入法**已接管 textarea 的 value** ⇒ `v-model` 写入落不到 DOM ⇒ **旧文本残留 +
  // 预编辑串插在行首** ✗（实证）。这里**不 preventDefault**：输入法照常起手 ✓
  if (selRange.value && !e.isComposing && (e.keyCode === 229 || e.key === "Process")) {
    const at = removeSelectedRange();
    if (at) focusBlock(blocks.value[at.index]?._key ?? "", at.offset);
    armKeepUndo("ime");
    return;
  }
  // PATCH002.73：**可打印字符 / 空格 = 原生"替换选区"语义** —— 删选区 → 光标落起点 → **放行**这次
  // 按键（不 preventDefault：浏览器把字打进已就位的焦点文本域 ✓）
  if (
    selRange.value &&
    !e.ctrlKey &&
    !e.metaKey &&
    !e.altKey &&
    !e.isComposing &&
    e.key.length === 1
  ) {
    const at = removeSelectedRange();
    if (at) focusBlock(blocks.value[at.index]?._key ?? "", at.offset);
    armKeepUndo("char"); // 这一下输入不清栈：Ctrl+Z 要能回到"替换前" ✓
    return;
  }
  // PATCH002.73：**方向键按原生** —— 取消自绘选区，并把光标落到对应端（←/↑ 起点、→/↓ 终点 ✓）
  if (
    selRange.value &&
    (e.key === "ArrowLeft" ||
      e.key === "ArrowRight" ||
      e.key === "ArrowUp" ||
      e.key === "ArrowDown")
  ) {
    const r = selRange.value;
    const target = e.key === "ArrowLeft" || e.key === "ArrowUp" ? r.from : r.to;
    clearSel();
    e.preventDefault();
    focusBlock(blocks.value[target.index]?._key ?? "", target.offset);
    return;
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
  // 临时诊断（.72 排查）：粘贴走了哪个分支 + 自绘选区是否还在（`selRange` 空 ⇒ 普通粘贴）
  {
    const r = selRange.value;
    const a = selAnchor.value;
    const f = selFocus.value;
    undoLog(
      `paste:enter sel=${r ? `${r.from.index}:${r.from.offset}->${r.to.index}:${r.to.offset}` : "null"}` +
        ` anchor=${a ? `${a.index}:${a.offset}` : "null"} focus=${f ? `${f.index}:${f.offset}` : "null"}` +
        ` native=${el.selectionStart}-${el.selectionEnd} lines=${(e.clipboardData?.getData("text/plain") ?? "").split("\n").length}`,
    );
  }
  // PATCH002.70（A）：**自绘选区存在时，粘贴 = 整体替换选区**。
  // 根因：Ctrl+A 是板级自绘选区（`preventDefault` 掉原生全选），而粘贴只看**焦点文本域的
  // 原生选区** ⇒ 两套状态互不相认：粘贴落在原生光标处，单行文本甚至放行给浏览器原生粘贴、
  // 替换那块里残留的原生区间 ✗。修法：先**纯本地**删除选区并取回起点（落库交给下方统一
  // `scheduleSave`，避免与 `normalizeAndFlush` 的即时落库抢序 ✗），再按正常分叉在起点插入。
  const range = selRange.value;
  if (range) {
    e.preventDefault();
    // PATCH002.72：**一次粘贴 = 一次撤销**。唯一一次快照取在**粘贴前**（= Ctrl+A 选中的那整篇），
    // 删选区 + 后续插入一律不再入栈 ⇒ 第一次 Ctrl+Z 就回到"粘贴前"，不会先落到"空" ✗
    const todoId = props.todo?.id ?? null;
    clearKeepUndo("paste"); // 粘贴开始：豁免一律失效 ✓
    pasteInFlight += 1; // C5：窗口内所有保存入口跳过（防"删完未插"的空中间态落库）
    let at: SelPoint | null = null;
    try {
      pushUndo();
      at = removeSelectedRange(false);
      const html0 = e.clipboardData?.getData("text/html") ?? "";
      const plain0 = e.clipboardData?.getData("text/plain") ?? "";
      if (!at) return;
      // PATCH002.72 C6：图 / HTML 分支让 Rust 按"库里的块集"重建列表 ⇒ 本地删除必须先落库，
      // 否则服务端返回"未删的旧块 + 插入"（实证 len=6 ⇒ 旧内容跑到粘贴内容之后 ✗）
      if (html0.includes("<img") || clipboardHasImage(e)) await saveBlocks(true);
      if (html0.includes("<img")) {
        await pasteHtmlAt(at.index, at.offset, at.offset, html0, plain0, false);
        return;
      }
      if (clipboardHasImage(e)) {
        await pasteImageAt(at.index, at.offset, at.offset, plain0, false);
        return;
      }
      if (plain0.length > 0) pasteTextLines(at.index, at.offset, at.offset, plain0, false);
    } finally {
      pasteInFlight -= 1;
      // 临时诊断（.72 排查）：替换分支收尾时的模型规模与去向
      undoLog(
        `paste:end inFlight=${pasteInFlight} len=${blocks.value.length} sameTodo=${props.todo?.id === todoId}`,
      );
      // 窗口关闭后统一落**最终态**：板已切走/已关 ⇒ 按快照写回原条目（沿 `.47` 口径）；
      // 仍在同一条目 ⇒ 交给防抖（`pasteTextLines` 在窗口内的 scheduleSave 被跳过了 ✗）
      if (todoId != null) {
        if (props.todo?.id === todoId) scheduleSave();
        else void saveBlocksFor(todoId, blocks.value);
      }
      // PATCH002.78：替换分支收尾把光标放到**插入起点**（Ctrl+A ⇒ 第一行行首；三条插入分支统一）
      if (at && props.todo?.id === todoId) focusInsertionStart(at);
    }
    return;
  }
  const block = blocks.value[index];
  if (!block) return;
  const start = el.selectionStart ?? 0;
  const end = el.selectionEnd ?? 0;
  // PATCH002.25：剪贴板带 text/html 且含 <img> → 走多段粘贴（唯一能带**多图 + 段落结构**
  // 的载体；应用内复制经 PATCH002.24 内联 data: 图，此处免联网直取）
  const html = e.clipboardData?.getData("text/html") ?? "";
  if (html.includes("<img")) {
    e.preventDefault();
    await pasteHtmlAt(index, start, end, html, e.clipboardData?.getData("text/plain") ?? "");
    return;
  }
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

/** 粘贴前置就绪（PATCH002.46）：目标块必须**已在库中**（id > 0）——"幻影块"（空笔记首块由
 * `normalizeBlocks` 补出 `id=0`，且基准比对让它永不落库）与"跨条目未载完"都会让 Rust
 * 定位不到行、静默失败。返回是否就绪；未就绪由调用方中止并落一条可读错误 */
async function ensureBlocksReady(): Promise<boolean> {
  const todo = props.todo;
  if (todo == null) return false;
  if (savedTodoId !== todo.id) {
    await loadBlocks(todo.id); // 切源/开板未载完 → 先载
    if (savedTodoId !== todo.id) return false;
  }
  if (blocks.value.some((b) => b.id <= 0)) {
    // 临时诊断（.72 排查）：哪个保存入口
    undoLog(`save:caller=ensureReady inFlight=${pasteInFlight} len=${blocks.value.length}`);
    await saveBlocks(true); // 本地独有的块必须先拿到 id
  }
  return true;
}

/** 插图：先过就绪闸（把 live 文本交上去 + 保证目标块已落库）→ Rust 落盘/复用 + 切分插入 →
 * **外科式打补丁**（PATCH002.14：按 id 映射回既有块、保 `_key`，DOM 不重挂、原生
 * undo 栈不丢——整体替换会重建全部 textarea 的 DOM） */
async function pasteImageAt(
  index: number,
  start: number,
  end: number,
  fallbackText: string,
  push = true,
): Promise<void> {
  const todo = props.todo;
  const block = blocks.value[index];
  if (todo == null || !block) return;
  const key = block._key; // await 期间数组可能被整体替换 → 之后一律按 _key 重解析
  if (!(await ensureBlocksReady())) {
    console.error("粘贴中止：详情块未就绪（目标块尚未落库）");
    return;
  }
  const target = blocks.value.find((b) => b._key === key) ?? blocks.value[index];
  if (!target || target.id <= 0) {
    console.error("粘贴中止：目标块定位失败", { key });
    return;
  }
  const before = target.text.slice(0, Math.min(start, target.text.length));
  const after = target.text.slice(Math.min(end, target.text.length));
  try {
    const outcome = await invoke<NotePasteOutcome>("note_paste_image", {
      todoId: todo.id,
      blockId: target.id,
      before,
      after,
    });
    if (props.todo?.id !== todo.id) return;
    if (outcome.status === "image") {
      if (push) pushUndo(); // 结构性操作入栈（仅真插图入栈；文本回落由 pasteTextLines 按同一 push 决定）
      blocks.value = normalizeBlocks(reconcileLocal(outcome.blocks));
      savedSignature = signature(blocks.value);
      brokenKeys.value = [];
      void nextTick(() => autosizeAll());
      emit("changed");
    } else {
      // 剪贴板无图（或非图片文件）→ 回落文本粘贴（含多行拆块），不吞粘贴
      // （PATCH002.72：`push` 透传——本函数在回落分支**未**入栈，由 pasteTextLines 按同一开关决定）
      const at = blocks.value.findIndex((b) => b._key === key);
      pasteTextLines(at >= 0 ? at : index, start, end, fallbackText, push);
    }
  } catch (err) {
    console.error("插入图片失败", err);
  }
}

/** 含图 HTML 粘贴（PATCH002.25）：先过就绪闸（块 id 就位）→ Rust 多段插入（`data:` 图直解、
 * 其他 scheme 降级文本链接 / 不支持的图降级短占位）→ **整体替换本地块**（沿 `pasteImageAt`
 * 的外科式打补丁口径）；片段全空 → 回落纯文本剪贴板（PATCH002.46，此前这条退路是断的） */
async function pasteHtmlAt(
  index: number,
  start: number,
  end: number,
  html: string,
  fallbackText: string,
  push = true,
): Promise<void> {
  const todo = props.todo;
  const block = blocks.value[index];
  if (todo == null || !block) return;
  const key = block._key;
  if (!(await ensureBlocksReady())) {
    console.error("粘贴中止：详情块未就绪（目标块尚未落库）");
    return;
  }
  const target = blocks.value.find((b) => b._key === key) ?? blocks.value[index];
  if (!target || target.id <= 0) {
    console.error("粘贴中止：目标块定位失败", { key });
    return;
  }
  const before = target.text.slice(0, Math.min(start, target.text.length));
  const after = target.text.slice(Math.min(end, target.text.length));
  try {
    const outcome = await invoke<NotePasteOutcome>("note_paste_html", {
      todoId: todo.id,
      blockId: target.id,
      before,
      after,
      html,
    });
    if (props.todo?.id !== todo.id) return;
    if (outcome.status !== "image") {
      // 片段全空（剪贴板 HTML 里没有可入块的内容）→ 回落纯文本，不吞这次粘贴
      // （PATCH002.72：`push` 透传——回落分支未入栈，由 pasteTextLines 按同一开关决定）
      const at = blocks.value.findIndex((b) => b._key === key);
      pasteTextLines(at >= 0 ? at : index, start, end, fallbackText, push);
      return;
    }
    if (push) pushUndo();
    blocks.value = normalizeBlocks(reconcileLocal(outcome.blocks));
    savedSignature = signature(blocks.value);
    brokenKeys.value = [];
    void nextTick(() => autosizeAll());
    emit("changed");
  } catch (err) {
    console.error("粘贴图文失败", err);
  }
}

/** 多行文本粘贴：按行拆成多块（保持"一段一行一块"不变量）；单行不走此处（原生粘贴） */
function pasteTextLines(
  index: number,
  start: number,
  end: number,
  text: string,
  push = true,
): void {
  const block = blocks.value[index];
  if (!block) return;
  if (push) pushUndo();
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

/** 粘贴替换收尾：光标落**插入起点**（PATCH002.78：Ctrl+A → 粘贴 ⇒ 第一行行首）。
 * 起点落在图块上时，顺延到其后第一个文本块块首 */
function focusInsertionStart(at: SelPoint): void {
  let index = Math.min(Math.max(at.index, 0), blocks.value.length - 1);
  let offset = at.offset;
  while (index < blocks.value.length && blocks.value[index].kind !== "Text") {
    index += 1;
    offset = 0;
  }
  const target = blocks.value[index];
  if (!target || target.kind !== "Text") return;
  focusBlock(target._key, Math.min(offset, target.text.length));
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
function onBlockContainerClick(block: LocalBlock, e: MouseEvent): void {
  // PATCH002.77：Shift+点击是扩选；`shiftGesture` 兜住"Shift 先于鼠标松开、click 丢了 shiftKey"
  if (e.shiftKey || shiftGesture) return;
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
// 纯逻辑在 `composables/normalizeBlocks.ts`（PATCH002.75 抽出：可测 + 全空补块复用焦点块）；
// 组件侧只留"焦点 key 取值 + 块数变化诊断"。

/** 当前焦点所在文本块的 `_key`（无焦点 / 焦点不在块区 ⇒ undefined） */
function focusedBlockKey(): string | undefined {
  const el = document.activeElement;
  if (!el) return undefined;
  for (const [key, node] of textEls) if (node === el) return key;
  return undefined;
}

/** 空块归一化（组件侧包装）：带"当前焦点块 key"（PATCH002.75）+ 块数变化诊断 */
function normalizeBlocks(list: LocalBlock[]): LocalBlock[] {
  const out = normalizeBlankBlocks(list, { preferredKey: focusedBlockKey(), makeKey: localKey });
  // 临时诊断（抖动定位，测完即删）：归一化**改变块数**时记一笔（`keep` = 留下的是哪一块）
  if (out.length !== list.length) {
    const last = list[list.length - 1];
    void invoke("debug_log", {
      line: `normalize ${list.length}->${out.length} inLast=${last ? `${last.kind}#${last.id}` : "none"} keep=${out[0] ? `${out[0].kind}#${out[0].id}` : "none"}`,
    }).catch((err) => console.error("诊断上报失败", err));
  }
  return out;
}

/** **保存点**归一化 + 立即落库（PATCH002.8：Ctrl+Enter / 焦点离开块区 / 关板切源专用）。
 * 带 `todoId` 且**与当前 props 不符**（切源已发生）时走**快照路径**：只归一化这份快照再
 * 落库、不回写当前板，且**强制落库**（此时 `savedTodoId` 可能已被切源改写，PATCH002.47） */
async function normalizeAndFlush(todoId?: number, list?: LocalBlock[]): Promise<void> {
  clearTimeout(saveTimer);
  // 临时诊断（.72 排查）：哪个保存入口
  undoLog(
    `save:caller=normalizeAndFlush inFlight=${pasteInFlight} len=${(list ?? blocks.value).length}`,
  );
  const target = list ?? blocks.value;
  if (todoId != null && todoId !== props.todo?.id) {
    const snapshot = normalizeBlocks(target);
    await saveBlocksFor(todoId, snapshot, true);
    return;
  }
  const next = normalizeBlocks(blocks.value);
  if (next.length !== blocks.value.length) {
    blocks.value = next;
    // 临时诊断（.75 排查）：赋值**后一拍**看焦点是否仍在块区（保 `_key` ⇒ `focus=blk-N` ✓；
    // 若为 `focus=none` ⇒ 焦点仍被销毁 ✗）
    void nextTick(() =>
      undoLog(`normalize:apply len=${blocks.value.length} focus=${focusedBlockKey() ?? "none"}`),
    );
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

/** 组字开始（PATCH002.73 乙）：先把自绘选区**删掉**、光标落到选区起点 ⇒ 输入法提交的文字
 * 即为"**替换**" ✓（与直接打字一致）。**风险与定案**：部分输入法在组字开始时改焦点可能被打断
 * ⇒ 用户定案走"乙"；若实测某输入法异常，退回"甲"（不插手，提交=插入）并把本条登记为已知边界 */
function onBlocksCompositionStart(): void {
  if (!selRange.value) return;
  const at = removeSelectedRange();
  if (at) focusBlock(blocks.value[at.index]?._key ?? "", at.offset);
  armKeepUndo("ime"); // 组字期间的连续 input 都不许清栈 ✓
}

/** 组字结束（PATCH002.73）：**延迟一拍**再清豁免 —— 部分浏览器 `compositionend` 早于最后一次
 * `input` ⇒ 立即清会把刚压的快照清掉 ✗（回到"撤销成空白" ✓） */
function onBlocksCompositionEnd(): void {
  undoLog("undo:keep compositionend");
  window.setTimeout(() => {
    clearKeepUndo("compositionend");
    scheduleSave(); // 窗口关闭后把组字结果落库 ✓
  }, 0);
}

/** 焦点离开块区（PATCH002.8 保存点）：归一化 + 立即落库 */
function onBlocksFocusOut(e: FocusEvent): void {
  const container = scrollEl.value;
  const next = e.relatedTarget as Node | null;
  if (container && next && container.contains(next)) return; // 仍在块区内
  // PATCH002.72 C5：粘贴"先删后插"期间，被删文本域的失焦会伪造一次 focusout ⇒
  // 此刻归一化落库 = 把"删完、还没插"的空中间态写进库（实证：`sig=36` 的空块被保存 ✗）
  if (pasteInFlight > 0 || editInFlight > 0) return;
  // 临时诊断（抖动定位，测完即删）：焦点离开块区 → 会走保存点归一化
  dragDiag("focusout:normalizeAndFlush");
  void normalizeAndFlush();
}

// —— 结构性撤销（PATCH002.14 用户定案"乙"：只收结构性操作；打字走浏览器原生）——
// 栈非空 → Ctrl+Z 截走并还原快照；栈空 → 放行原生（打字撤销）；**打字即清空本栈**
// （"按删除 undo 删除、打字 undo 打字"：打字后 Ctrl+Z 交回原生，不回退到旧结构）。

/** 结构性操作快照栈（上限 30；浅拷贝块对象——_key 一并保留，还原时不重挂 DOM）。
 * PATCH002.78：**单源条目** = 块快照 + 操作前光标（同栈进出，避免平行数组失步） */
interface UndoEntry {
  blocks: LocalBlock[];
  /** 操作前活动文本块与光标（不在文本块 → null：撤销不抢标题框等输入面焦点） */
  caret: { key: string; offset: number } | null;
}
const undoStack: Array<UndoEntry> = [];
const UNDO_LIMIT = 30;
/** **粘贴进行中**计数（PATCH002.72 C5）：`.70` 的"替换选区"是**先删后插**——删完那一瞬
 * 被删的文本域失焦 ⇒ 浏览器抛 `focusout` ⇒ 保存点会把"删完、还没插"的**空中间态**落库
 * （实证：`sig=36` 的空块被保存）。窗口内**所有保存入口一律跳过**，最终态由粘贴收尾统一写。 */
let pasteInFlight = 0;
/** **一次性保留撤销栈**（PATCH002.73）：打字默认 `dropUndo()`（清结构性栈），但"替换选区"刚压了快照
 * ⇒ 必须让它活过**这一下输入**，否则 Ctrl+Z 只能落到原生（回退成空白 ✗）。只允许一个 ✓ */
let keepUndoOnce = false;
/** 输入法组字期间保留（`compositionstart` → `compositionend` 之间会连续触发多次 `input` ✓） */
let keepUndoComposing = false;
/** **结构性编辑窗口**（PATCH002.73）：替换/组字都是"**先删后写**"——删的那一瞬被删文本域失焦会
 * 伪造一次 `focusout` ⇒ 若此刻归一化落库/清豁免，就会把空中间态写进库、并把撤销快照清掉 ✗
 * （实证：`undo:keep clear reason=focusout` 紧接 `pre-save len=1`）。窗口内**保存入口与豁免清理一律跳过**。 */
let editInFlight = 0;
/** 置一次豁免（`char` = 普通字符，下一拍兜底自清；`ime` = 组字，活到 `compositionend`） */
function armKeepUndo(reason: "char" | "ime"): void {
  keepUndoOnce = true;
  if (reason === "ime") keepUndoComposing = true;
  editInFlight += 1; // 打开结构性编辑窗口（删→写之间不许落库、不许清豁免 ✓）
  undoLog(`undo:keep set reason=${reason} editInFlight=${editInFlight}`);
  if (reason === "char") {
    // 兜底：万一那一下没产生 input（键被别处拦了 / 焦点丢了），下一拍自清 ✓
    window.setTimeout(() => clearKeepUndo("timeout"), 0);
  } else {
    // 兜底：一旦判"输入法起手"却没真的进入组字（键被吞 / 输入法直接上屏），5s 后自清，
    // 免得结构性编辑窗口一直挂着把保存全挡住 ✗
    window.setTimeout(() => clearKeepUndo("ime-timeout"), 5000);
  }
}
/** 清豁免（带原因 ⇒ 万一泄漏，日志能定位是哪条路径清的/没清 ✓） */
function clearKeepUndo(reason: string): void {
  const had = keepUndoOnce || keepUndoComposing;
  keepUndoOnce = false;
  keepUndoComposing = false;
  if (editInFlight > 0) editInFlight -= 1; // 关闭窗口
  if (had) undoLog(`undo:keep clear reason=${reason} editInFlight=${editInFlight}`);
}
/** 撤销代际（PATCH002.44）：删块动效是"**延迟收场**"，收场前复核本代际——用户已撤销则放弃
 * 收场（否则 180ms 后的摘除会覆盖用户的回退 = "动效期间按 Ctrl+Z 无效"） */
let undoSeq = 0;

/** 结构性操作**前**入栈（拆块/合并/删块/插图/多行粘贴/选区删除；附操作前光标） */
function pushUndo(): void {
  const active = document.activeElement as HTMLTextAreaElement | null;
  let caret: UndoEntry["caret"] = null;
  if (active?.classList.contains("note-block-text")) {
    const host = active.closest<HTMLElement>(".note-block");
    const index = Number(host?.dataset.rowId) - 1;
    const block = blocks.value[index];
    if (block?.kind === "Text") caret = { key: block._key, offset: active.selectionStart ?? 0 };
  }
  undoStack.push({ blocks: blocks.value.map((b) => ({ ...b })), caret });
  if (undoStack.length > UNDO_LIMIT) undoStack.shift();
  // 临时诊断（.72 排查）：这次压进去的是**哪一代**
  undoLog(`undo:push depth=${undoStack.length} ids=${idsOf(blocks.value)}`);
}

/** 打字即清空结构性栈（原生接管；见段首语义） */
function dropUndo(): void {
  if (undoStack.length > 0) undoStack.length = 0;
}

/** 撤销一次结构性操作（Ctrl+Z；栈空由调用方放行原生） */
function undoStructural(): void {
  // PATCH002.73（A）：**组字在飞时必须重建文本域** —— 输入法持有该 textarea 的 value ⇒ 程序写入
  // 的旧文本会被忽略（实证：Ctrl+Z 后模型已恢复 3 块、屏幕仍停在组字结果 "a" ✗）。
  // 换新 `_key` ⇒ Vue 重建元素 ⇒ 摆脱输入法 ✓；**普通撤销不重建**（保 `_key` ⇒ DOM 不重挂、焦点不丢 ✓）
  const wasComposing = keepUndoComposing;
  clearKeepUndo("undo"); // 撤销时豁免必须失效（防旧快照跨操作存活 ✓）
  const entry = undoStack.pop();
  if (!entry) return;
  const snapshot = entry.blocks;
  const caret = entry.caret;
  // 临时诊断（.72 排查）：恢复**前**的当前代 vs 恢复**后**的 id 序列（跨代即 bug ✗）
  const beforeIds = idsOf(blocks.value);
  undoSeq += 1; // PATCH002.44：撤销代际 +1（在飞的删块动效收场时据此放弃）
  clearSel();
  selectedKey.value = "";
  blocks.value = wasComposing
    ? snapshot.map((b) => ({ ...b, _key: localKey() })) // 组字在飞 ⇒ 全量重建（脱开输入法持有的元素 ✓）
    : snapshot.map((b) => ({ ...b }));
  undoLog(`undo:pop depth=${undoStack.length} before=${beforeIds} restored=${idsOf(blocks.value)}`);
  void nextTick(() => {
    autosizeAll();
    // PATCH002.72：焦点若停在**已被 Vue 移除**的文本域上（快照换掉了该块），之后打的字只进
    // 那个游离 DOM、不进 blocks ⇒ signature 不变 ⇒ 保存被静默跳过（"写了字、退板还是空"）
    const active = document.activeElement as HTMLElement | null;
    if (active?.classList.contains("note-block-text") && !active.isConnected) active.blur();
    // PATCH002.78：撤销后把光标放回**操作前**的位置（Ctrl+A → 粘贴 ⇒ 第一行行首）。
    // 组字在飞时 key 已重建 ⇒ 跳过（避免找不回元素）
    if (caret && !wasComposing) {
      const target = blocks.value.find((b) => b._key === caret.key);
      if (target) focusBlock(target._key, Math.min(caret.offset, target.text.length));
    }
  });
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
/** 已进入跨块（覆盖层接管指针事件；单块内一律交原生） */
const crossSelecting = ref(false);
/** 跨块拖选的**源块 key**（PATCH002.45：跨块期间给源块挂 `sel-crossing` 压制其原生选区） */
const selCrossKey = ref("");
/** 指针所在块下标（PATCH002.45 优化：覆盖层只挂"源块 / 选区内 / 指针块"）。
 * **必须先由元素命中定它、再等一帧做精确 caret 命中**——否则该块的覆盖层本帧还没渲染，
 * 命中必失败 → 退化成"整块选择"（首版顺序写反导致过该缺陷，见 y.problems #16） */
const selHoverIndex = ref(-1);

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
  crossSelecting.value = false;
  selCrossKey.value = ""; // 选区清空才撤源块标记（松手不撤：原生选区仍在，须持续压制）
  selHoverIndex.value = -1;
}

/** 全选所有块（Ctrl+A 板级拦截；仅一块时改走原生全选）。
 * PATCH002.78：① 无块焦点时补焦点（否则 Enter / 粘贴没有落点——事件只挂在 textarea 上）；
 * ② 单块笔记不能"吃掉原生全选还什么都不选"⇒ 确定性全选（文本块原生选区 / 图块选中态） */
function selectAllBlocks(): void {
  const active = document.activeElement as HTMLElement | null;
  const activeInBlock = !!active?.closest(".note-block-text");
  // 标题等输入面：不抢焦点（沿既有行为；Ctrl+A 仍选中所有块）
  const activeInField =
    !activeInBlock && !!active?.closest("input, textarea, [contenteditable='true']");
  const last = blocks.value.length - 1;
  if (last <= 0) {
    const only = blocks.value[0];
    if (!only || activeInField) return;
    if (only.kind === "Text") {
      const el = textEls.get(only._key);
      if (el) {
        el.focus({ preventScroll: true });
        el.setSelectionRange(0, el.value.length);
        dragDiag(`ctrlA:single len=${el.value.length}`);
      }
    } else {
      selectedKey.value = only._key; // 单图块：走图块选中态（Ctrl+C 可复制）
      dragDiag("ctrlA:single image");
    }
    return;
  }
  selectedKey.value = ""; // 与图块单击选中态互斥
  selAnchor.value = { index: 0, offset: 0 };
  selFocus.value = { index: last, offset: blocks.value[last].text.length };
  // PATCH002.78：无块焦点 ⇒ 补一个（focusBlock 自带 preventScroll + 定光标，不拉视口）
  if (!activeInBlock && !activeInField) {
    const firstText = blocks.value.find((b) => b.kind === "Text");
    if (firstText) {
      focusBlock(firstText._key, 0);
      dragDiag(`ctrlA:focus idx=${blocks.value.indexOf(firstText)}`);
    }
  }
}

/** 复制入参（镜像 Rust `NoteCopyBlock`，PATCH002.24）：文本块原文 + 图块 id */
interface NoteCopyBlockInput {
  kind: "Text" | "Image";
  text: string;
  block_id: number;
}

/** 收集待复制块（PATCH002.24）：跨块选区优先（三段区间 = 起点块尾段 / 中间整块 /
 * 终点块首段），否则图块单击选中态单块；两者皆无 → 空数组（调用方直接返回） */
function copyPayload(): NoteCopyBlockInput[] {
  const r = selRange.value;
  if (r) {
    const out: NoteCopyBlockInput[] = [];
    for (let i = r.from.index; i <= r.to.index; i += 1) {
      const block = blocks.value[i];
      const seg = blockSelRange(i);
      if (!block || !seg) continue;
      out.push({
        kind: block.kind,
        text: block.kind === "Text" ? block.text.slice(seg.start, seg.end) : "",
        block_id: block.id,
      });
    }
    return out;
  }
  const picked = blocks.value.find((b) => b._key === selectedKey.value);
  return picked && picked.kind === "Image"
    ? [{ kind: "Image", text: "", block_id: picked.id }]
    : [];
}

/** 复制选区 / 图块到系统剪贴板（PATCH002.24）：走 Rust **一次写入**——纯文本 +
 * CF_HTML（应用内图内联 base64 data URL，超上限降级占位）+ 首图 PNG/CF_DIB 三格式；
 * 失败落 console，不吞 */
async function copySelection(): Promise<void> {
  const payload = copyPayload();
  if (payload.length === 0) return;
  try {
    await invoke("note_copy_selection", { blocks: payload });
  } catch (err) {
    console.error("复制选区失败", err);
  }
}

/** 删除选区（三段拼接：起点块留前段 + 接终点块后段，中间块整体删除；起点块对象复用 →
 * 保 `_key` 不重挂）。**纯本地**：只改 `blocks` + 清选区，**不落库、不重算高度**，
 * 供"键位删除"与"粘贴替换选区"两个入口各取所需（PATCH002.70）。
 * @returns 选区**起点**（= 删除后的插入位置；`offset` 取前段长度）；null = 无可删 */
function removeSelectedRange(push = true): SelPoint | null {
  const r = selRange.value;
  if (!r) return null;
  const first = blocks.value[r.from.index];
  const last = blocks.value[r.to.index];
  if (!first || !last) return null;
  if (push) pushUndo();
  const head = first.kind === "Text" ? first.text.slice(0, r.from.offset) : "";
  const tail = last.kind === "Text" ? last.text.slice(r.to.offset) : "";
  first.kind = "Text";
  first.text = head + tail;
  first.image_path = null;
  blocks.value.splice(r.from.index + 1, r.to.index - r.from.index);
  const at: SelPoint = { index: r.from.index, offset: head.length };
  clearSel();
  return at;
}

/** 删除选区（键位入口：Backspace / Delete）：本地删除 + 重算高度 + 落库 */

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

/** 拖选焦点落点：**一次命中**——先 caret 命中（覆盖层文本节点 → 块 + 块内偏移）；
 * 失败（图块无覆盖层 / 指针落块内距 / 空块）才退回元素命中 + "越过块中线"取首/尾 */
function focusPointAt(x: number, y: number): SelPoint | null {
  const hit = offsetFromPoint(x, y);
  if (hit) return hit;
  const el = document.elementFromPoint(x, y)?.closest<HTMLElement>(".note-block");
  if (!el) return null;
  const index = Number(el.dataset.rowId) - 1;
  if (!Number.isInteger(index) || index < 0) return null;
  const block = blocks.value[index];
  const len = block?.kind === "Text" ? block.text.length : 0;
  const rect = el.getBoundingClientRect();
  return { index, offset: y < rect.top + rect.height / 2 ? 0 : len };
}

/** 拖选源块下标（文本块 mousedown 记；-1 = 无） */
let selDragIndex = -1;

/** 拖选按下点坐标（方案 B：锚点优先按"按下点"精确命中，见 `anchorOffset`） */
let selDownX = -1;
let selDownY = -1;

/** PATCH002.77：本次 Shift+点击是否已在 mousedown 阶段扩选（随后的 click 不得再走单选） */
let shiftGesture = false;

/** 源块文本域（PATCH002.77）：优先当前焦点；焦点已丢（先点了块内距/空白）则回落最近聚焦块。
 * 实测（Chromium）：失焦后源 textarea 的原生选区仍在 ⇒ 兜底可用 */
function sourceTextEl(): HTMLTextAreaElement | null {
  const active = document.activeElement as HTMLTextAreaElement | null;
  if (active?.classList.contains("note-block-text")) return active;
  return lastTextKey ? (textEls.get(lastTextKey) ?? null) : null;
}

/** 把"块内原生选区 / 光标"提升为跨块锚点（PATCH002.77）：无锚点时才做；方向按
 * `selectionDirection`。**光标也提升**（collapsed）——否则"光标在第一块中间 + Shift+点下一块"
 * 会被原生截断在源块末尾；定位不到块 → 不提升（保持原生行为） */
function promoteNativeSelectionAnchor(): boolean {
  if (selAnchor.value) return true;
  const el = sourceTextEl();
  if (!el) return false;
  const start = el.selectionStart ?? 0;
  const end = el.selectionEnd ?? 0;
  const host = el.closest<HTMLElement>(".note-block");
  const index = Number(host?.dataset.rowId) - 1;
  if (!Number.isInteger(index) || index < 0) return false;
  selAnchor.value = { index, offset: el.selectionDirection === "backward" ? end : start };
  dragDiag(`shift:promote idx=${index} off=${selAnchor.value.offset}`);
  return true;
}

/** 文本块 mousedown：记拖源块与**按下点坐标** + 清互斥态（选区/图块选中）。
 * PATCH002.39（2026-10-20 口径修正）：**Shift+点击 = 原生语义扩选**——不重设锚点，端点取
 * **鼠标所在那一点**（文本块精确到**字符**、图块取**最近边界**），**不是整块选取**；
 * 无既有选区时退化为普通按下（等同单击，语义安全）。 */
function onTextMouseDown(index: number, e: MouseEvent): void {
  if (e.shiftKey && selAnchor.value) {
    extendSelectionByShiftClick(e);
    return;
  }
  selDownX = e.clientX;
  selDownY = e.clientY;
  selDragIndex = index;
  clearSel();
  selCrossKey.value = blocks.value[index]?._key ?? ""; // 源块标记（跨块期间压制其原生选区）
  if (selectedKey.value) selectedKey.value = "";
}

/** PATCH002.39 的 Shift+点击实现（文本块与图块共用）。
 *
 * **字符级命中必须借自绘覆盖层**：`document.caretPositionFromPoint` 落在**文本域**上只
 * 返回元素（不是文本节点）⇒ 拿不到字符偏移（`.45` 已实证）。故与拖选同款三段式：
 * ① 先用**元素命中**定"指针块"（此时覆盖层未挂，不会被自绘层挡住）→ ② **记 `selHoverIndex`
 * 并挂覆盖层**（`cross-selecting` 让文本域让出 caret 命中）→ ③ **下一拍**用
 * `offsetFromPoint` 精确采样，采完**立即收层**并把 `selHoverIndex` 复位。
 *
 * ⚠️ **② 里的 `selHoverIndex` 不可省**（2026-10-20 live 实证）：覆盖层按
 * `selCrossKey === key || i === selHoverIndex || blockSelRange(i) !== null` 逐块挂载
 * ——首击时后两条都不成立（`selFocus` 还没设 ⇒ `selRange` 为 null），漏了 `selHoverIndex`
 * 就**没有覆盖层** ⇒ 命中返回 null ⇒ 降级取"块末尾" ⇒ **首击看起来是整块选** ✗
 * （第二击之所以正常，是因为 `selRange` 已建立、覆盖层已挂 ✓）。
 *
 * 图块无字符：**点上半 ⇒ 止于图前**（端点落到上一块末尾）、**点下半 ⇒ 含该图**
 * （端点落到该图，索引在区内即整块入选）。 */
function extendSelectionByShiftClick(e: MouseEvent): void {
  const x = e.clientX;
  const y = e.clientY;
  const el = document.elementFromPoint(x, y)?.closest<HTMLElement>(".note-block");
  if (!el) return;
  const index = Number(el.dataset.rowId) - 1;
  const block = blocks.value[index];
  if (!block) return;
  const rect = el.getBoundingClientRect();
  const upperHalf = y < rect.top + rect.height / 2;

  // PATCH002.70（B）：**单块内 Shift 扩选交原生**（`.45` 口径）——不进入自绘管线。
  // PATCH002.77：跨块压制期间原生选区可能已被浏览器改写（`.45` 打点）⇒ 交原生之前先把源块
  // 收成**自定义锚点**，否则会从错的锚点扩 ✗
  if (selAnchor.value && selAnchor.value.index === index) {
    const source = textEls.get(block._key);
    if (source) source.setSelectionRange(selAnchor.value.offset, selAnchor.value.offset);
    return;
  }

  if (block.kind === "Image") {
    const prev = index > 0 ? blocks.value[index - 1] : null;
    applyShiftEndpoint(
      upperHalf && prev
        ? { index: index - 1, offset: prev.kind === "Text" ? prev.text.length : 0 }
        : { index, offset: 0 },
    );
    return;
  }

  markShiftSource(); // 源块原生选区压制（否则与自绘层叠成双层涂色 ✗）
  selHoverIndex.value = index; // 挂载条件之一：本拍就让**指针块**的覆盖层出现
  crossSelecting.value = true; // 文本域让出 caret 命中
  void nextTick(() => {
    const at = offsetFromPoint(x, y); // 字符级
    selFocus.value = at ?? { index, offset: upperHalf ? 0 : block.text.length };
    crossSelecting.value = false; // 采完收层：pointer-events 还给文本域
    selHoverIndex.value = -1; // Shift+点击无 mouseup 收尾流程 ⇒ 须自己复位
    // PATCH002.70（B）：把目标文本域的**原生选区收成光标** —— 浏览器在 mousedown 时仍会
    // 做一次原生 Shift 扩选、留下"原生区间"；随后 Ctrl+V 若走原生粘贴就会**替换**它 ✗
    const tel = textEls.get(block._key);
    if (tel) {
      const pos = Math.min(selFocus.value?.offset ?? 0, block.text.length);
      tel.setSelectionRange(pos, pos);
    }
  });
}

/** Shift+扩选的共同前置：源块原生选区压制 + 与"图块单击选中态"互斥 */
function markShiftSource(): void {
  const anchor = selAnchor.value;
  selCrossKey.value = anchor ? (blocks.value[anchor.index]?._key ?? "") : "";
  if (selectedKey.value) selectedKey.value = "";
}

/** 图块分支的落点（同步可定）：前置 + 置终点 */
function applyShiftEndpoint(point: SelPoint): void {
  markShiftSource();
  selFocus.value = point;
}

/** 图块点击（PATCH002.39）：Shift+点击 = **原生语义扩选**（按落点取边界，非整块）；
 * 普通点击 = 选中态（原语义） */
function onImageClick(block: LocalBlock, e: MouseEvent): void {
  // PATCH002.77：Shift 扩选已在 mousedown 完成（即使 Shift 在 mouseup 前松开）⇒ 不得再走单选
  if (shiftGesture) return;
  if (e.shiftKey && selAnchor.value) {
    extendSelectionByShiftClick(e);
    return;
  }
  selectedKey.value = block._key;
}

/** 锚点偏移（方案 B ③ 两级求法）：先按**按下点坐标**精确命中（覆盖层已挂载时可用）；
 * 失败回落**按方向取端**——原生选区两端恒按文档序排列、不随拖拽方向翻转：指针在源块
 * **上方**（向上拖）时按下点落在 `selectionEnd`，在下方（向下拖）时落在 `selectionStart` */
function anchorOffset(el: HTMLTextAreaElement, pointerY: number): number {
  const hit = offsetFromPoint(selDownX, selDownY);
  if (hit && hit.index === selDragIndex) return hit.offset;
  const rect = el.getBoundingClientRect();
  const upward = pointerY < rect.top;
  return (upward ? el.selectionEnd : el.selectionStart) ?? 0;
}

/** 文档级 mousemove：指针越出拖源块即进跨块模式——**方案 B「越块即交接」**：
 * 单块内仍走原生选区；越块那一刻起，选区归自绘层所有：
 * ① 先置 `crossSelecting`（覆盖层立刻挂载）→ ② `nextTick` 用同坐标补采样 `selFocus`
 * （消"首帧按块中线降级"的整片变暗）→ ③ 锚点两级求法（`anchorOffset`）→ ④ 收起原生选区 */
function onDocMouseMove(e: MouseEvent): void {
  if (selDragIndex < 0) return;
  const src = blocks.value[selDragIndex];
  if (!src || src.kind !== "Text") return;
  // ① 先用**元素命中**定"指针块"（不依赖覆盖层）→ 该块的覆盖层本帧起才会被渲染
  const hovered = document
    .elementFromPoint(e.clientX, e.clientY)
    ?.closest<HTMLElement>(".note-block");
  const hoverIndex = hovered ? Number(hovered.dataset.rowId) - 1 : -1;
  const hoverChanged =
    Number.isInteger(hoverIndex) && hoverIndex >= 0 && hoverIndex !== selHoverIndex.value;
  if (hoverChanged) selHoverIndex.value = hoverIndex;
  const point = focusPointAt(e.clientX, e.clientY);
  if (!point || point.index === selDragIndex) return;
  if (!selAnchor.value) {
    const x = e.clientX;
    const y = e.clientY;
    crossSelecting.value = true; // ① 覆盖层先挂载（③ 的精确命中依赖它）
    void nextTick(() => {
      if (selDragIndex < 0) return; // 已松手：不再补采样
      const el = textEls.get(src._key);
      if (!el) return;
      const offset = anchorOffset(el, y); // ③
      selAnchor.value = { index: selDragIndex, offset };
      const at = focusPointAt(x, y); // ② 此刻覆盖层已就位 → 精确
      if (at) selFocus.value = at;
    });
    return;
  }
  // ② 刚进入新块（覆盖层本帧才渲染）→ 等下一拍再精确采样，**不落粗值**（免一帧整块闪）
  if (hoverChanged) {
    const x = e.clientX;
    const y = e.clientY;
    void nextTick(() => {
      if (selDragIndex < 0) return;
      const at = focusPointAt(x, y);
      if (at) selFocus.value = at;
    });
    return;
  }
  selFocus.value = point;
}

/** 文档级 mouseup：结束拖源（选区保留，供 Ctrl+C / Delete） */
function onDocMouseUp(): void {
  selDragIndex = -1;
  crossSelecting.value = false;
  selHoverIndex.value = -1; // 松手后焦点块已在选区内，指针块不再需要覆盖层
}

/** 板内拦截原生 HTML5 拖拽（PATCH002.74）：从"仍在选中态的文字"起手时，浏览器会把它
 * 当成"拖动已选文本"——拖拽会话吞掉 mousemove ⇒ 自绘跨块圈选根本起不来（禁止光标）。
 * **只掐 DnD，不动选区机制**（`.45` 单块内原生选择照旧；图块调序走 pointer 事件不受影响）。 */
function onBoardDragStart(e: DragEvent): void {
  e.preventDefault();
  dragDiag("dnd:blocked"); // 临时诊断（随 PATCH002 整批收官清）
}

// —— 拖拽调序（**仅图块可拖**；机制复用 useDragReorder，落点提交走 note_reorder）——

/** 拖拽收场重拉（PATCH002.56：末块不变量由前端 `normalizeBlocks` 在保存点负责）。
 * PATCH002.57（2026-10-19 打点定性）：**收场前若在底部 → 收场后回到新底部**——拖拽期被拖行
 * 重挂出列表（内容变矮、`st` 停在"矮内容"的底），收场把图放回 + 归一化补末块 → 内容**合法**
 * 长高（实测 +155px = 图高 + 补块高），视口却停在旧底之上 → 观感"跳回原来那一行"。
 * **回退方式**：删掉下面 `wasAtBottom` 一行 + 末尾那个 if 块即可（其余一字不动）。 */
async function reloadAfterDrag(todoId: number, ghost: HTMLElement | null = null): Promise<void> {
  const el = scrollEl.value;
  // PATCH002.79 回归修正：贴底判据回到 `.69` 的**收场时**计算——按下前采集会让
  // "从中间拖到底部"松手后不跟底 ⇒ 内容往上跳（`.57` 老 bug 复现）；`.69` 原样不动
  const wasAtBottom = el ? el.scrollTop >= el.scrollHeight - el.clientHeight - 1 : false;
  // PATCH002.82：ghost 仍在占位 ⇒ 过渡期不钉底（RO 别按"新列表 + ghost"的虚高贴底）
  kit?.boardRead.setPinned(false);
  // 占位符在 loadBlocks 赋值新列表**前**同一任务撤掉（只 paint 新列表，无 715/534 中间帧）
  await loadBlocks(todoId, ghost);
  if (ghost?.isConnected) ghost.remove(); // 兜底（重拉失败 / 条目已切走）
  await nextTick(); // 等新列表渲染完再定滚动
  if (wasAtBottom && el) {
    el.scrollTop = el.scrollHeight;
  }
  kit?.boardRead.setPinned(wasAtBottom); // 过渡结束恢复套件贴底态
  dragDiag("drag:settled");
}

/** 临时诊断（抖动定位，测完即删）：滚动几何 + 列表末尾块（`last` 的 kind#id）——用于判断
 * "库里多出的块"从哪来、前端是否仍以图块结尾；经 `debug_log` 送 Rust stdout */
function dragDiag(tag: string): void {
  const el = scrollEl.value;
  const rows = el ? [...el.querySelectorAll<HTMLElement>(".note-block")] : [];
  const geo = el ? `${el.scrollTop.toFixed(1)}/${el.scrollHeight}/${el.clientHeight}` : "no-el";
  const last = blocks.value[blocks.value.length - 1];
  const lastKind = last ? `${last.kind}#${last.id}` : "none";
  void invoke("debug_log", {
    line: `${tag} st/sh/ch=${geo} domRows=${rows.length} arrLen=${blocks.value.length} last=${lastKind}`,
  }).catch((err) => console.error("诊断上报失败", err));
}

installNoteDragHooks({
  onDown: () => {
    clearSel(); // 互斥（PATCH002.17）：图块长按拖拽优先，起拖即清选区
    clearTimeout(saveTimer);
    // PATCH002.56 修正：起拖**强制**落库（`force` 绕开签名比对）——保证重排前库与本地
    // 的块 id 集合一致，否则 Rust 的 `validate_reorder_ids` 会拒掉整条重排（块弹回原位）
    void saveBlocks(true);
  },
  commit: (slots) => {
    const todo = props.todo;
    if (todo == null) return;
    dragDiag(`drag:commit slots=${slots.join(",")}`);
    // 临时诊断（退回定位，测完即删）：提交那一刻 DOM 里**每一行**的 class+rowId
    // —— 用于找出"DOM 比数组多出来的那一行"是什么（ghost 残留 / 重复行 / 别的）
    const domEl = scrollEl.value;
    const rows = domEl ? [...domEl.querySelectorAll<HTMLElement>(".note-block")] : [];
    void invoke("debug_log", {
      line: `commit-dom n=${rows.length} ${rows
        .map(
          (r) =>
            `${r.className.replace(/\bnote-block\b/, "").trim() || "-"}#${r.dataset.rowId ?? "?"}`,
        )
        .join(" ")}`,
    }).catch((err) => console.error("诊断上报失败", err));
    const bySlot = new Map(blocks.value.map((b, i) => [i + 1, b]));
    const ordered = slots.map((slot) => bySlot.get(slot)).filter((b): b is LocalBlock => b != null);
    if (ordered.length !== blocks.value.length) return;
    // PATCH002.82：**不本地改 blocks**——否则会先渲染“图片 + ghost”的一帧；
    // 顺序由重拉（toLocal 全量换 key）带回，落库照旧
    dragDiag(`commit:ordered ids=${ordered.map((b) => b.id).join(",")}`);
    void invoke("note_reorder", { todoId: todo.id, ids: ordered.map((b) => b.id) }).catch((err) =>
      console.error("详情块重排失败", err),
    );
  },
  rerender: (ctx) => {
    const todo = props.todo;
    // PATCH002.57 打点（测完即删）：进入重拉前的 DOM/数组对齐快照（候选 1 的判决点）
    dragDiag("drag:rerender-entry");
    // PATCH002.82：ghost 由重拉流程收尾（列表高度保持到新列表就位）
    if (todo) void reloadAfterDrag(todo.id, ctx?.ghost ?? null);
    else ctx?.ghost.remove();
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

/** 等图片就绪（PATCH002.51）：`decode()` 完成才开板 ⇒ 无"空板面/黑纱褪去"。
 * **800ms 安全阀**只为防 `decode()` 死锁（本地文件正常 <20ms；超时即放行，退回旧行为）；
 * 解码失败不阻断开板（交给 `@error` 走既有失效态） */
async function decodeImageSrc(src: string, timeoutMs = 800): Promise<void> {
  const img = new Image();
  img.src = src;
  const decoded = typeof img.decode === "function" ? img.decode() : Promise.resolve();
  await Promise.race([
    decoded.catch(() => undefined),
    new Promise<void>((resolve) => window.setTimeout(resolve, timeoutMs)),
  ]);
}

/** 揭纱推进距离（PATCH002.51）：按**视口高**给 `--veil-h`（px）—— 超高图若按图高百分比算，
 * 可见段会"一秒刷完"。取 `min(滚动容器 clientHeight, 图实际高) + 100px`（渐变带宽）：
 * 短图到图底即结束（不空跑），长图始终按可见高推进。**必须是纯 px**（注册属性不接受 %） */
function setVeilHeight(): void {
  void nextTick(() => {
    const el = scrollEl.value;
    if (!el) return;
    const box = el.querySelector<HTMLElement>(".bubble-img-wrap");
    const visible = el.clientHeight;
    const imgH = box?.clientHeight ?? 0;
    const base = imgH > 0 ? Math.min(visible, imgH) : visible;
    if (base > 0) el.style.setProperty("--veil-h", `${Math.round(base) + 100}px`);
  });
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
  boardRead: {
    sync: () => void;
    settle: () => void;
    destroy: () => void;
    setPinned: (v: boolean) => void;
  };
  glassBar: { sync: () => void; destroy: () => void };
}
let kit: DetailKit | null = null;

/** 挂/卸套件（幂等：关板即卸——FIX013.5 显式清理先例）。
 * PATCH002.10/.11：套件挂**内层滚动容器**（mask 只作用内容，不碰外壳的描边/落影）；
 * PATCH002.50 起**统一两态**（到底 100% / 其余默认 14px 软边，与清单/气泡/归档/白板同口径，
 * 不再有"跨底缘半截块溶解"） */

function syncKit(): void {
  kit?.boardRead.destroy();
  kit?.glassBar.destroy();
  kit = null;
  if (!isOpen.value || !scrollEl.value) return;
  kit = {
    boardRead: useBoardRead(scrollEl.value, {
      gate: () => isOpen.value,
      layout: true,
      onDiag: dragDiag,
    }),
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
  clearKeepUndo("close"); // 关板：豁免失效 ✓
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
      // 切源快照：**清空前**捕获旧条目的 id 与块列表（PATCH002.47——切源 flush 必须写回旧 id，
      // 否则"打字后立刻切条目"这段编辑永久丢失）
      const prev = { todoId: savedTodoId, list: blocks.value };
      void flushPending(prev); // 切源先 flush 旧条目未决保存（快照写回旧 id，FIX003.4）
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
      // 切源快照（同 todo watch，PATCH002.47）：todo→气泡切换时 `props.todo` 已为 null，
      // 必须按快照把未决编辑写回旧条目 id（否则这段编辑丢失）
      const prev = { todoId: savedTodoId, list: blocks.value };
      void flushPending(prev); // todo→bubble 切源同款先 flush（FIX003.4）
      if (b.kind === "Image") {
        // PL025 图片气泡：板面 = 落盘图片（按需取绝对路径 → asset 协议直读）
        bubbleText.value = "";
        bubbleImgSrc.value = "";
        imageError.value = "";
        // PATCH002.51（A+C）：**先取图 + 解码，再开板** —— 旧实现先开板 ⇒ 图就位前是一整块
        // 只有黑纱的空板面（用户所说"黑纱褪去"）。安全阀 800ms 见 decodeImageSrc
        void (async () => {
          try {
            const path = await invoke<string>("bubble_get_image_path", { id: b.id });
            if (props.bubble?.id !== b.id) return;
            const src = resolveImageSrc(path);
            await decodeImageSrc(src);
            if (props.bubble?.id !== b.id) return;
            bubbleImgSrc.value = src;
          } catch (err) {
            console.error("气泡图片取数失败", err);
            if (props.bubble?.id !== b.id) return;
            imageError.value = "文件已移动或删除，请删除气泡";
          }
          open();
          setVeilHeight(); // PATCH002.51：揭纱距离按视口高
        })();
        return; // 本分支自己开板（等图就位）
      }
      bubbleImgSrc.value = "";
      imageError.value = "";
      bubbleText.value = b.text; // 气泡全文只读展示
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

/** 立即保存未决草稿（关板/切源/卸载的 flush——保存不丢弃，FIX003.4）；`prev` = **切源前
 * 捕获的** {todoId, list} 快照：watch 里 `props.todo` 已是新条目，必须按快照写回旧 id
 * （否则守卫必假 → 旧条目的未决编辑丢失，PATCH002.47）。缺省按当前条目 */
async function flushPending(prev?: { todoId: number | null; list: LocalBlock[] }): Promise<void> {
  // PATCH002.72 C5：粘贴窗口内不落库（此时 `blocks` 可能是"删完未插"的中间态 ✗）；
  // 收尾的 `finally` 会把最终态按快照写回（板已切走时也能写，沿 `.47` 口径）
  if (pasteInFlight > 0 || editInFlight > 0) return;
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
  // 详情块未决保存（PL027）：有快照按快照落库；无快照按当前条目（须已载入本条目）
  const todoId = prev ? prev.todoId : (props.todo?.id ?? null);
  const list = prev ? prev.list : blocks.value;
  if (todoId != null && savedTodoId === todoId && signature(list) !== savedSignature) {
    undoLog(`save:caller=flush inFlight=${pasteInFlight} len=${list.length}`);
    jobs.push(normalizeAndFlush(todoId, list));
  }
  await Promise.all(jobs);
}

/** 视口尺寸变化后重算自动增高（PATCH002.27）：宽度一变文本就重排 → 不重算会把文字截掉
 * （textarea 已不自己滚，高度必须跟着内容走） */
function onViewportResize(): void {
  autosizeAll();
  autosizeBubble();
}

// 板内 mousedown：清图块选中 / 清选区（板内交互须即时）
// PATCH002.49（甲案）：**板外收板从 mousedown 挪到 click** —— 拖动把手（主标题）在板外，
// mousedown 口径会在拖拽启动前就把板收掉；click 与归档板 / 设置板一致（拖动不产生 click）
function onGlobalDown(e: MouseEvent): void {
  if (!isOpen.value) return;
  shiftGesture = false; // PATCH002.77：新手势开始，重置上一次 Shift 扩选标记
  const ov = overlay.value;
  const target = e.target as HTMLElement | null;
  if (!ov || !ov.contains(e.target as Node)) return; // 板外交给 onDocClick
  // PATCH002.39：Shift+点击是"**扩选**"——**不得清选区**（否则锚点在 click 之前就没了）。
  // 捕获阶段最先到这里 ⇒ 只此一处即可覆盖文本块与图块两种落点。
  if (e.shiftKey) {
    // PATCH002.77：块内原生选区从不产生锚点 ⇒ 先提升；否则浏览器原生 Shift+点击只会
    // 把源块选区拉到块尾（Chromium 实测），永远进不了下一块。只在块区内提升（标题/板头不动）
    const inScroll = !!scrollEl.value?.contains(e.target as Node);
    const onCtrl = target?.closest(".note-block-del");
    const promoted = !onCtrl && inScroll && promoteNativeSelectionAnchor();
    const inBlock = target?.closest(".note-block");
    const onText = target?.closest(".note-block-text");
    // 文本块交 onTextMouseDown（`.39` 既有路径）；图块 / 块内距 / 容器在这里直接扩选并
    // 压住原生（消焦点环、防止原生把源块选区拉到块尾）
    if (promoted && inBlock && !onText && !onCtrl) {
      extendSelectionByShiftClick(e);
      e.preventDefault();
      shiftGesture = true;
    }
    return;
  }
  if (selectedKey.value && !target?.closest(".note-block-image")) selectedKey.value = "";
  // 文本块 mousedown 由 onTextMouseDown 自处理（拖选锚点）；其余位置一律清选区
  if (!target?.closest(".note-block-text")) clearSel();
}

/** 板外点击收板（PATCH002.49 甲案）：`click` 而非 `mousedown`——拖动把手不产生 click，
 * 故拖窗口时板不被收掉。**行 / 气泡行上的点击例外**：那是"切换详情源"（行单击开板），
 * 由开板流程接管（否则 document 冒泡阶段晚到的 close 会把刚开的板关掉） */
function onDocClick(e: MouseEvent): void {
  if (!isOpen.value) return;
  const ov = overlay.value;
  const target = e.target as HTMLElement | null;
  if (ov && ov.contains(e.target as Node)) return;
  if (target?.closest(".todo-row") || target?.closest(".bubble-row")) return;
  close();
}
window.addEventListener("mousedown", onGlobalDown, true);
document.addEventListener("click", onDocClick);
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
  document.removeEventListener("click", onDocClick);
  window.removeEventListener("keydown", onWindowKeydown, true);
  window.removeEventListener("mousemove", onDocMouseMove, true);
  window.removeEventListener("mouseup", onDocMouseUp, true);
  window.removeEventListener("resize", onViewportResize);
});
</script>

<template>
  <div
    ref="overlay"
    class="detail-overlay"
    :class="{ open: isOpen, 'bubble-mode': isBubbleMode, 'cross-selecting': crossSelecting }"
    @dragstart="onBoardDragStart"
  >
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
          @compositionstart="onBlocksCompositionStart"
          @compositionend="onBlocksCompositionEnd"
        >
          <!-- bubble 模式：图片 / 全文只读（PL024/PL025 原样） -->
          <template v-if="isBubbleMode">
            <div v-if="bubbleImgSrc && !imageError && isOpen" class="bubble-img-wrap">
              <img
                class="detail-image"
                :src="bubbleImgSrc"
                alt="气泡图片"
                @error="onImageError"
                @dblclick="onImageDblClick"
              />
            </div>
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
                'sel-crossing': selCrossKey === block._key && (crossSelecting || selRange !== null),
              }"
              :data-row-id="i + 1"
              :tabindex="block.kind === 'Image' ? 0 : undefined"
              @click="onBlockContainerClick(block, $event)"
              @keydown="onBlockContainerKeydown(i, $event)"
            >
              <template v-if="block.kind === 'Image'">
                <img
                  v-if="!brokenKeys.includes(block._key)"
                  class="note-block-img"
                  :src="blockSrc(block)"
                  alt="详情图片"
                  draggable="false"
                  @click="onImageClick(block, $event)"
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
                :placeholder="i === 0 && noteEmpty ? '添加描述、清单、想法…' : ''"
                spellcheck="false"
                rows="1"
                @focus="onTextFocus(block._key)"
                @mousedown="onTextMouseDown(i, $event)"
                @input="onBlockInput"
                @keydown="onBlockKeydown(i, $event)"
                @paste="onBlockPaste(i, $event)"
              ></textarea>
              <!-- 跨块选区高亮层（PATCH002.16/.17）：文本透明只显底色；拖选期间接管指针事件。
                   PATCH002.21：渲染条件由 `selDragging || selRange` 收紧为 `crossSelecting || selRange`
                   —— 普通 mousedown 不再给所有文本块挂全文透明覆盖层（一挂一卸 = 全块重绘闪烁） -->
              <div
                v-if="
                  block.kind === 'Text' &&
                  (crossSelecting || selRange) &&
                  (selCrossKey === block._key || i === selHoverIndex || blockSelRange(i) !== null)
                "
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
