<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { BubbleItem, TodoItem } from "../../types";
import { useBoardRead } from "../composables/useBoardRead";
import { useGlassBar } from "../composables/useGlassBar";
import { syncVeils } from "../composables/useVeils";

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
      noteDraft.value = b.text; // 气泡全文只读展示
      open();
    } else if (!props.todo) {
      close();
    }
  },
);

// —— 标题改名（maxlength 12，input 实时同步回清单——debounce 300ms 合并 IPC）。
// bubble 模式只读：head 整个隐藏无 input，此 watch 天然不触发 ——

let renameTimer: number | undefined;
watch(titleDraft, (text) => {
  if (!props.todo || text === props.todo.text) return;
  clearTimeout(renameTimer);
  renameTimer = window.setTimeout(() => {
    void invoke("todo_rename", { id: props.todo!.id, text })
      .then(() => emit("changed"))
      .catch((err) => console.error("改名失败", err));
  }, 300);
});

// —— 笔记防抖 300ms 保存（与实验场白板同拍）；bubble 模式 readonly 不触发 ——

let noteTimer: number | undefined;
watch(noteDraft, (note) => {
  if (!props.todo || note === props.todo.note) return;
  clearTimeout(noteTimer);
  noteTimer = window.setTimeout(() => {
    void invoke("todo_set_note", { id: props.todo!.id, note })
      .then(() => emit("changed"))
      .catch((err) => console.error("笔记保存失败", err));
  }, 300);
});

/** 组件卸载前若有未落库草稿，立即保存（防抖兜底） */
function flushPending(): void {
  clearTimeout(renameTimer);
  clearTimeout(noteTimer);
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
  flushPending();
  window.removeEventListener("mousedown", onGlobalDown, true);
});
</script>

<template>
  <div ref="overlay" class="detail-overlay" :class="{ open: isOpen, 'bubble-mode': isBubbleMode }">
    <div class="board-glass detail-glass">
      <div v-if="!isBubbleMode" class="detail-head">
        <span class="detail-label">标题</span>
        <input v-model="titleDraft" class="detail-title" maxlength="12" placeholder="最多 12 字" />
      </div>
      <div class="note-shell" :class="{ 'bubble-shell': isBubbleMode }">
        <textarea
          ref="noteEl"
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
