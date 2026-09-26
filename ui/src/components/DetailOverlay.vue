<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { TodoItem } from "../../types";
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
}>();

const emit = defineEmits<{ changed: []; close: [] }>();

const overlay = ref<HTMLElement | null>(null);
const titleDraft = ref("");
const noteDraft = ref("");

const isOpen = ref(false);

// 板内整板阅读（gate：仅开板时亮三角）+ 玻璃滑杆（textarea 透明填壳同源滚动）
const noteEl = ref<HTMLTextAreaElement | null>(null);
let boardRead: { sync: () => void; settle: () => void } | null = null;
let glassBar: { sync: () => void } | null = null;

/** 开板：揭示动画几何注入（页签顶实测上扩 4px；origin = 左上归档图标方向兜底） */
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
    if (noteEl.value && !boardRead) {
      boardRead = useBoardRead(noteEl.value, {
        gate: () => isOpen.value,
        layout: true,
      });
      glassBar = useGlassBar(noteEl.value, { right: 4.75 });
    }
    window.setTimeout(() => {
      boardRead?.settle();
      glassBar?.sync();
    }, 450);
  });
}

/** 关板：摘 open 类（0.35s 缩回）+ 帘布复位 */
function close(): void {
  isOpen.value = false;
  syncVeils();
  emit("close");
}

// todo prop 变化驱动开合与草稿装载
watch(
  () => props.todo,
  (t) => {
    if (t) {
      titleDraft.value = t.text;
      noteDraft.value = t.note;
      open();
    } else {
      close();
    }
  },
);

// —— 标题改名（maxlength 12，input 实时同步回清单——debounce 300ms 合并 IPC） ——

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

// —— 笔记防抖 300ms 保存（与实验场白板同拍） ——

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

import { onBeforeUnmount } from "vue";
onBeforeUnmount(() => {
  flushPending();
  window.removeEventListener("mousedown", onGlobalDown, true);
});
</script>

<template>
  <div ref="overlay" class="detail-overlay" :class="{ open: isOpen }">
    <div class="board-glass detail-glass">
      <div class="detail-head">
        <span class="detail-label">标题</span>
        <input v-model="titleDraft" class="detail-title" maxlength="12" placeholder="最多 12 字" />
      </div>
      <div class="note-shell">
        <textarea
          ref="noteEl"
          v-model="noteDraft"
          class="detail-note board-read"
          placeholder="添加描述、清单、想法…"
          spellcheck="false"
        ></textarea>
      </div>
    </div>
  </div>
</template>

<style scoped>
@import "../styles/detail.css";
</style>
