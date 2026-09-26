<script setup lang="ts">
import { computed, onUnmounted, ref } from "vue";
import type { TodoItem, TodoView } from "../../types";
import DelButton from "./DelButton.vue";
import NeonCheckbox from "./NeonCheckbox.vue";
import { invoke } from "@tauri-apps/api/core";
import { rowMaskDead, syncMaskDead } from "../composables/useMaskDead";

// ===== 清单页（PL010.4 换装实验场形态）：行模板 1:1（NeonCheckbox + t-text +
// DelButton 二态 + age-alert 黄红提醒）+ TransitionGroup 出入场（0.3s 显式 duration——
// 遮挡 webview 不派发动画事件）+ 罩死禁交互 + DelButton 单实例收口 =====

const props = defineProps<{
  /** 清单排序视图（父级经 todo_list 拉取；含龄期档位） */
  items: TodoView[];
}>();

const emit = defineEmits<{ changed: []; openDetail: [item: TodoItem] }>();

/** 未完成行（排序视图已由 Rust 裁决，此处不再排序只过滤） */
const active = computed(() => props.items.filter((it) => !it.done));

// —— DelButton 二态单实例：全列表同时只允许一行处于确认态 ——

/** 当前确认中的条目 id（null = 无）；换目标时旧行自动回退 */
const confirmingId = ref<number | null>(null);

/** 行组件的 rollBack expose 句柄（id → 组件实例） */
const delRefs = new Map<number, InstanceType<typeof DelButton>>();

function setDelRef(id: number, el: InstanceType<typeof DelButton> | null): void {
  if (el) delRefs.set(id, el);
  else delRefs.delete(id);
}

/** 首点进确认态：单实例收口（旧确认行立即回退） */
function onPress(item: TodoItem): void {
  if (confirmingId.value != null && confirmingId.value !== item.id) {
    delRefs.get(confirmingId.value)?.rollBack();
  }
  confirmingId.value = item.id;
}

/** 再点执行：摘确认态 + 删除（DelButton 已延迟 200ms 播完脉冲） */
async function onConfirm(item: TodoItem): Promise<void> {
  confirmingId.value = null;
  try {
    await invoke("todo_remove", { id: item.id });
    emit("changed");
  } catch (err) {
    console.error("删除失败", err);
  }
}

// —— 勾选 / 点击语义 ——

/** 罩死判定的行元素定位（null 安全：行不在 DOM 即视为不罩死——塌缩离场中） */
function rowEl(id: number): HTMLElement {
  return (
    (document.querySelector(`#page-todos [data-row-id="${id}"]`) as HTMLElement | null) ??
    document.createElement("div")
  );
}

/** 勾选翻转：成功后通知父级重拉（300ms 流光主拍由 CSS 播放，不需 JS 等待——
 * 入档塌缩编排属 PL011 归档板接线） */
async function toggle(item: TodoItem): Promise<void> {
  if (rowMaskDead(rowEl(item.id))) return; // 罩死行勾选失效（V0.022 定案）
  try {
    await invoke("todo_toggle", { id: item.id });
    emit("changed");
  } catch (err) {
    console.error("勾选失败", err);
  }
}

let clickTimer: number | undefined;
function onRowClick(item: TodoItem): void {
  if (Date.now() < suppressUntil) return; // 拖拽落点抑制（PL013 注入窗口）
  if (rowMaskDead(rowEl(item.id))) return;
  clearTimeout(clickTimer);
  clickTimer = window.setTimeout(() => emit("openDetail", item), 180);
}

function onRowDblClick(item: TodoItem): void {
  clearTimeout(clickTimer); // 掐掉未决的开板定时器
  startInlineEdit(item);
}

/** 拖拽落点点击抑制时间戳（PL013 useDragReorder 写入；本组件只读） */
let suppressUntil = 0;

// —— 行内改标题（双击；≤12 字与详情板同规） ——

const editingId = ref<number | null>(null);
const editDraft = ref("");

function startInlineEdit(item: TodoItem): void {
  editingId.value = item.id;
  editDraft.value = item.text;
}

async function commitEdit(item: TodoItem): Promise<void> {
  const text = editDraft.value.trim();
  editingId.value = null;
  if (!text || text === item.text) return;
  try {
    await invoke("todo_rename", { id: item.id, text });
    emit("changed");
  } catch (err) {
    console.error("改名失败", err);
  }
}

// 内容增删后罩死复核（滚动监听经 useGlassBar 钩子已通；此处补重渲染路径）
const stopMask = computed(() => {
  syncMaskDead();
  return props.items.length;
});
void stopMask;

onUnmounted(() => {
  clearTimeout(clickTimer);
});
</script>

<template>
  <section class="list">
    <p v-if="items.length === 0" class="empty">暂无待办，添加一条吧</p>
    <TransitionGroup v-else tag="ul" name="todo" class="group" :duration="320">
      <li v-for="item in active" :key="item.id" class="todo-item" :class="{ 'mask-dead': false }">
        <div
          class="todo-row"
          :data-row-id="item.id"
          @click="onRowClick(item)"
          @dblclick="onRowDblClick(item)"
        >
          <NeonCheckbox :checked="item.done" @toggle="toggle(item)" />
          <input
            v-if="editingId === item.id"
            v-model="editDraft"
            class="t-edit"
            maxlength="12"
            @keydown.enter="commitEdit(item)"
            @blur="commitEdit(item)"
            @keydown.esc="editingId = null"
          />
          <span v-else class="t-text" :class="{ 'is-done-text': item.done }">{{ item.text }}</span>
          <DelButton
            :ref="(el) => setDelRef(item.id, el as InstanceType<typeof DelButton>)"
            :confirming="confirmingId === item.id"
            @press="onPress(item)"
            @confirm="onConfirm(item)"
          />
        </div>
        <p v-if="item.age_level === 'Yellow'" class="age-alert age-alert--yellow">
          已超过24小时了哦&nbsp;&nbsp;｜ω･) WATCHING
        </p>
        <p v-else-if="item.age_level === 'Red'" class="age-alert age-alert--red">
          2天都过去了哟&nbsp;&nbsp;( ﾟдﾟ) …… 大懒虫
        </p>
      </li>
    </TransitionGroup>
  </section>
</template>

<style scoped>
@import "../styles/todos.css";
</style>
