<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { BubbleItem, TodoItem } from "../../types";
import { UI_TEXT_MAX_LEN } from "../../types";
import { useBoardRead } from "../composables/useBoardRead";
import { useGlassBar } from "../composables/useGlassBar";
import { bindOverlayState, syncVeils } from "../composables/useVeils";

// ===== 详情板（PL010.5 todo 模式；bubble 模式 PL012 填充）：
// 揭示动画（origin 注入 + scale 回弹）/ 标题行内改名（≤12 字实时 todo_rename）/
// 笔记防抖 300ms todo_set_note / 板内整板阅读 + 玻璃滑杆 / 开合时 syncVeils 帘联动。
// 开板落位：top 经页签实测再上扩 4px（design 定案），origin 注入实现"从图标处飞出"。

const props = defineProps<{
  /** 开板数据源：当前详情条目（null = 关板） */
  todo: TodoItem | null;
  /** 气泡全文板数据源（PL012.3 bubble 模式；todo 优先级低——两源互斥由父级保证） */
  bubble: BubbleItem | null;
  /** 被点行中心视口坐标（A3：飞出原点 = 行中心——design positionDetailOverlay 同款） */
  anchor: { x: number; y: number } | null;
}>();

const emit = defineEmits<{ changed: []; close: [] }>();

/** 双模式（V0.020 ⑩ 定案）：bubble = 单层玻璃（标签头摘除 + note 透明直落板面 + readonly） */
const isBubbleMode = computed(() => props.bubble !== null);

const overlay = ref<HTMLElement | null>(null);
const titleDraft = ref("");
const noteDraft = ref("");

const isOpen = ref(false);

// 帘联动注册（FIX003.7）：详情开合进 useVeils 全局读取器——板开时主清单滑杆/三角
// 吃 .veiled 淡出、关板恢复（useVeils detail 特判按 textarea id=detail-note 匹配）
bindOverlayState("detail", () => isOpen.value);

// 板内整板阅读（gate：仅开板时亮三角）+ 玻璃滑杆（textarea 透明填壳同源滚动）
const noteEl = ref<HTMLTextAreaElement | null>(null);
let boardRead: { sync: () => void; settle: () => void } | null = null;
let glassBar: { sync: () => void } | null = null;

/** 开板：揭示动画几何注入（页签顶实测上扩 4px；飞出原点 = 被点行中心——A3，
 * design positionDetailOverlay 同款，origin 在 nextTick 后注入见 open 内注释） */
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
  // 首帧挂载板内组件（textarea 出现后才有滚动几何）+ 450ms 后 settle 重算
  void nextTick(() => {
    // 飞出原点注入放 nextTick：watch(todo) flush pre 时父级的 anchor prop 尚未
    // 传递到子组件（同一 tick 内先后赋值），渲染完成后读才是本次的行中心
    if (props.anchor) {
      const or = ov.getBoundingClientRect();
      ov.style.setProperty("--origin-x", `${Math.round(props.anchor.x - or.left)}px`);
      ov.style.setProperty("--origin-y", `${Math.round(props.anchor.y - or.top)}px`);
    }
    if (noteEl.value && !boardRead) {
      boardRead = useBoardRead(noteEl.value, {
        gate: () => isOpen.value,
        layout: true,
      });
      glassBar = useGlassBar(noteEl.value, { right: 4.75 });
    }
    // 滚动复位（A4 = design resetDetailScroll）：换内容必归零——上一次会话的
    // scrollTop 会残留（实测开板落在文末）；textarea 程序赋值不派发 scroll，
    // 补发合成事件让三角/到底抬带停在复位后状态
    const note = noteEl.value;
    if (note) {
      note.scrollTop = 0;
      note.dispatchEvent(new Event("scroll"));
    }
    window.setTimeout(() => {
      if (!isOpen.value) return; // 早关板防污染：settle 不把 at-bottom/带挂回已收的板
      boardRead?.settle();
      glassBar?.sync();
    }, 450);
  });
}

/** 关板：摘 open 类（0.35s 缩回）+ 帘布复位 + 清到底抬带残留（A4：类常驻后双模式
 * 都出溶解带，不清则下次开板继承上次滚动态——design setDetail(false) 同款） */
function close(): void {
  if (!isOpen.value) return; // 幂等守卫：已关再调不重复 flush/emit（FIX003.10⑧）
  void flushPending(); // 关板先 flush 未决保存（FIX003.4：保存不丢弃）
  isOpen.value = false;
  const note = noteEl.value;
  if (note) {
    note.classList.remove("at-bottom");
    note.style.removeProperty("--fade-btm");
  }
  syncVeils();
  emit("close");
}

// 数据源变化驱动开合与草稿装载（todo / bubble 双模式互斥）
watch(
  () => props.todo,
  (t) => {
    if (t) {
      void flushPending(); // 切源先 flush 旧条目未决保存（快照写回旧 id，FIX003.4）
      titleDraft.value = t.text;
      noteDraft.value = t.note;
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
      noteDraft.value = b.text; // 气泡全文只读展示
      open();
    } else if (!props.todo) {
      close();
    }
  },
);

// —— 标题改名（maxlength 12，input 实时同步回清单——debounce 300ms 合并 IPC）。
// bubble 模式只读：head 整个隐藏无 input，此 watch 天然不触发 ——
// FIX003.4：待保存快照 = { 触发时刻 id + 草稿 }，回调与 flush 只用快照、不回读
// props.todo——旧实现回调实时解引用 props.todo!.id，快关板 = null.id TypeError、
// 快切条目 = A 的草稿写进 B 的 id（静默数据污染）

/** 改名待保存快照（null = 无未决保存） */
let renamePending: { id: number; text: string } | null = null;
let renameTimer: number | undefined;
/** 保存失败可见反馈（FIX007.7，对齐 AddBar 错误行模式；成功/新输入即清） */
const saveError = ref("");
/** 最近一次成功落库的标题基准（FIX007.5 基准机制 + FIX008.2 带条目 id）：相等
 * 判定对基准比而非 props.todo——App 刷新不回写 detailTodo，props 是陈旧快照，
 * "改 A→B 再改回 A"对陈旧值恒相等 = 跳过保存 = UI/DB 静默分叉；id 锚定防跨
 * 条目残留——基准只对所属条目生效，晚到的 flush 回调不污染新会话 */
let lastSavedText: { id: number; text: string } | null = null;
watch(titleDraft, (text) => {
  const todo = props.todo;
  saveError.value = ""; // 新输入即清（FIX007.7）
  // FIX004.1：相等分支必须清未决快照——只 return 会残留上一变更的定时器，
  // 300ms 内改回原文时到点仍把中间草稿写库（UI 原文 / DB 草稿分叉）。
  // 基准取序：lastSaved（id 匹配的已落库值，FIX008.2）→ props（板打开时初值）
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
        // FIX008.2：基准只写当前板内条目——切条目后晚到的落库回调不污染新会话
        if (props.todo?.id === pending.id) {
          lastSavedText = { id: pending.id, text: pending.text };
        }
        emit("changed");
      })
      .catch((err) => {
        // FIX009.6：错误只属于当前板内条目——invoke 在飞期切条目后，旧条目的
        // 失败不挂新板（否则红字与操作对象错位，误导排查）；console.error 仍留底账
        if (props.todo?.id === pending.id) {
          saveError.value = `改名保存失败：${String(err)}`;
        }
        console.error("改名失败", err);
      });
  }, 300);
});

// —— 笔记防抖 300ms 保存（与实验场白板同拍）；bubble 模式 readonly 不触发 ——

/** 笔记待保存快照（null = 无未决保存） */
let notePending: { id: number; note: string } | null = null;
let noteTimer: number | undefined;
/** 最近一次成功落库的笔记基准（FIX007.5，同改名基准机制；FIX008.2 带条目 id） */
let lastSavedNote: { id: number; note: string } | null = null;
// 切换条目时重置基准与未决快照（FIX008.2）：基准属于上一条目，跨条目残留会误判
// 相等/误建 pending；未决草稿的写回已由 props.todo watcher 的 flushPending 先行
// 负责，此处作废安全——titleDraft/noteDraft 两个装载消费 watch 按创建序先于本
// watch 执行，装载新文本时基准 id 不匹配回落 props 初值，不再以新文本误建无果写
watch(
  () => props.todo?.id,
  () => {
    lastSavedText = null;
    lastSavedNote = null;
    clearTimeout(renameTimer);
    clearTimeout(noteTimer);
    renamePending = null;
    notePending = null;
    // FIX009.6 语义闭环：错误只属于当前板内条目——上一条目的失败红字不得
    // 残留到新条目（titleDraft 装载仅在值变化时触发清空，A/B 文本相同即漏）
    saveError.value = "";
  },
);
watch(noteDraft, (note) => {
  const todo = props.todo;
  saveError.value = ""; // 新输入即清（FIX007.7）
  // FIX004.1 同构：相等分支清未决快照（同"改回原文"残留）；基准 id 匹配才生效
  const savedBase =
    lastSavedNote != null && lastSavedNote.id === todo?.id
      ? lastSavedNote.note
      : (todo?.note ?? null);
  if (todo == null || note === savedBase) {
    clearTimeout(noteTimer);
    notePending = null;
    return;
  }
  clearTimeout(noteTimer);
  notePending = { id: todo.id, note };
  noteTimer = window.setTimeout(() => {
    const pending = notePending;
    notePending = null;
    if (pending == null) return;
    void invoke("todo_set_note", { id: pending.id, note: pending.note })
      .then(() => {
        // FIX008.2：基准只写当前板内条目（同改名侧）
        if (props.todo?.id === pending.id) {
          lastSavedNote = { id: pending.id, note: pending.note };
        }
        emit("changed");
      })
      .catch((err) => {
        // FIX009.6：同改名侧——错误只属于当前板内条目
        if (props.todo?.id === pending.id) {
          saveError.value = `笔记保存失败：${String(err)}`;
        }
        console.error("笔记保存失败", err);
      });
  }, 300);
});

/** 立即保存未决草稿（关板/切源/卸载的 flush——保存不丢弃，FIX003.4）；快照自带
 * id，切换数据源后旧条目的草稿仍写回旧 id */
async function flushPending(): Promise<void> {
  clearTimeout(renameTimer);
  clearTimeout(noteTimer);
  const rename = renamePending;
  const note = notePending;
  renamePending = null;
  notePending = null;
  const jobs: Array<Promise<void>> = [];
  if (rename != null) {
    jobs.push(
      invoke("todo_rename", { id: rename.id, text: rename.text })
        .then(() => {
          // FIX008.2：flush 晚于切条目时回调落在新会话——id 不匹配不写基准
          if (props.todo?.id === rename.id) {
            lastSavedText = { id: rename.id, text: rename.text };
          }
          emit("changed");
        })
        .catch((err) => {
          // FIX009.6：flush 失败归因随 id（本条目本体）——切源后旧条目的失败
          // 不挂新板（红字与操作对象错位 = 误导排查）；console.error 留底账
          if (props.todo?.id === rename.id) {
            saveError.value = `改名保存失败：${String(err)}`;
          }
          console.error("改名失败", err);
        }),
    );
  }
  if (note != null) {
    jobs.push(
      invoke("todo_set_note", { id: note.id, note: note.note })
        .then(() => {
          // FIX008.2：同 flush 改名侧，id 不匹配不写基准
          if (props.todo?.id === note.id) {
            lastSavedNote = { id: note.id, note: note.note };
          }
          emit("changed");
        })
        .catch((err) => {
          // FIX009.6：同 flush 改名侧，失败归因随 id
          if (props.todo?.id === note.id) {
            saveError.value = `笔记保存失败：${String(err)}`;
          }
          console.error("笔记保存失败", err);
        }),
    );
  }
  await Promise.all(jobs);
}
defineExpose({ close, flushPending });

// 板外收板（design 定案语义）：mousedown 落在 overlay 外即收板——capture 挂 window，
// 停止于 overlay 内部的交互不受影响；overlay 为 null（未挂载）时跳过
function onGlobalDown(e: MouseEvent): void {
  if (!isOpen.value) return;
  const ov = overlay.value;
  if (ov && !ov.contains(e.target as Node)) close();
}
window.addEventListener("mousedown", onGlobalDown, true);

onBeforeUnmount(() => {
  void flushPending();
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
      <div class="note-shell" :class="{ 'bubble-shell': isBubbleMode }">
        <textarea
          ref="noteEl"
          id="detail-note"
          v-model="noteDraft"
          class="detail-note board-read"
          :class="{ 'bubble-note': isBubbleMode }"
          :readonly="isBubbleMode"
          :placeholder="isBubbleMode ? '' : '添加描述、清单、想法…'"
          spellcheck="false"
        ></textarea>
      </div>
    </div>
  </div>
</template>

<style scoped>
@import "../styles/detail.css";
</style>
