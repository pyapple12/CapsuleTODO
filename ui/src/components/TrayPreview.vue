<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";

// ===== 托盘悬浮预览（PL018.4）：独立 tray-preview 窗的根组件——未完成待办前五
// 条最简展示（状态点 + 文本，点行切换）。显隐由 Rust 守候线程光标轮询判定，
// 前端只管列表与高度（不再上报悬停状态）

/** 预览条目（todo_list 返回 TodoView 的展示子集） */
interface PreviewTodo {
  id: number;
  text: string;
  done: boolean;
}

const todos = ref<PreviewTodo[]>([]);
let unlistenTodoChanged: UnlistenFn | undefined;
/** 勾选动画中的条目 id（用户定案：方形勾选框 + 画勾动画，播完行才消失落库） */
const animatingId = ref<number | null>(null);
const CHECK_ANIM_MS = 380;

/** 点行：播画勾动画，播完 toggle 落库（行随已完成过滤消失）；动画期防重复点击 */
function onRowClick(t: PreviewTodo): void {
  if (animatingId.value !== null) return;
  animatingId.value = t.id;
  window.setTimeout(() => {
    const id = animatingId.value;
    animatingId.value = null;
    if (id !== null) void toggle(id);
  }, CHECK_ANIM_MS);
}

/** 高度随内容实时缩放（用户定案恢复：DWM 圆角切掉闪帧直角后观感代价可接受）。
 * 走 Rust 命令（JS setSize 需 ACL allow-set-size 会静默拒绝） */
watch(
  todos,
  async () => {
    await nextTick();
    try {
      const listH = document.querySelector(".tp-list")?.getBoundingClientRect().height ?? 164;
      await invoke("tray_preview_resize", { height: Math.ceil(listH) + 18 });
    } catch (err) {
      console.error("预览窗高度调整失败", err);
    }
  },
  { deep: true },
);

/** 拉取未完成前五条（过滤已完成——用户定案不列已完成） */
async function refresh(): Promise<void> {
  try {
    const all = await invoke<PreviewTodo[]>("todo_list");
    todos.value = all.filter((t) => !t.done).slice(0, 5);
  } catch (err) {
    console.error("预览列表拉取失败", err);
  }
}

/** toggle 落库（动画播完后调用）→ 自刷新 → 广播 todo-changed 驱动主窗同步 */
async function toggle(id: number): Promise<void> {
  try {
    await invoke("todo_toggle", { id });
    await refresh();
    await emit("todo-changed", {});
  } catch (err) {
    console.error("预览打勾失败", err);
  }
}

onMounted(async () => {
  // 高度链修正（视觉二轮）：html/body/#app 无显式高度时 height:100% 塌缩为内容高，
  // 窗体剩余区域露出 webview 原生暗背景（用户截图"两层"）——预览窗实例 JS 直改
  //（只影响本窗，不动主窗布局）
  document.documentElement.style.height = "100%";
  document.documentElement.style.background = "transparent";
  document.body.style.height = "100%";
  document.body.style.background = "transparent";
  const root = document.getElementById("app");
  if (root) {
    root.style.height = "100%";
    root.style.background = "transparent";
  }
  await refresh();
  // 主窗改动（新增/勾选/归档）→ 预览列表同步
  unlistenTodoChanged = await listen("todo-changed", () => {
    void refresh();
  });
});

onUnmounted(() => {
  unlistenTodoChanged?.();
});
</script>

<template>
  <div class="tray-preview">
    <ul class="tp-list">
      <li v-for="t in todos" :key="t.id" class="tp-row" @click="onRowClick(t)">
        <span class="tp-box" :class="{ 'tp-box-checked': animatingId === t.id }">
          <svg v-if="animatingId === t.id" class="tp-check" viewBox="0 0 12 12" aria-hidden="true">
            <polyline points="2.5,6.5 5,9 9.5,3" />
          </svg>
        </span>
        <span class="tp-text">{{ t.text }}</span>
      </li>
      <li v-if="todos.length === 0" class="tp-empty">暂无待办</li>
    </ul>
  </div>
</template>

<style scoped>
/* 极简小窗（用户定案二轮：取消玻璃材质花哨不实用）：纯深底 + 细描边，
高度随内容（watch 后 Rust resize 窗体），无标题行 */
.tray-preview {
  box-sizing: border-box;
  width: 100%;
  height: 100%; /* 铺满当前窗（用户定案三轮：hover 期间窗高恒定，勾选后框内留白） */
  padding: 8px 10px;
  border-radius: 8px;
  background: rgba(24, 26, 34, 0.96);
  border: 1px solid rgba(255, 255, 255, 0.18);
  color: #fff;
  font-size: 12px;
  user-select: none;
}

.tp-title {
  margin: 0 0 6px;
  font-size: 11px;
  opacity: 0.6;
}

.tp-list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.tp-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 2px;
  border-radius: 6px;
  cursor: pointer;
}

.tp-row:hover {
  background: rgba(255, 255, 255, 0.08);
}

/* 方形勾选框（用户定案：圆点改方框）+ 画勾动画（播完行消失落库） */
.tp-box {
  flex: none;
  width: 13px;
  height: 13px;
  border-radius: 3px;
  border: 1.5px solid rgba(255, 255, 255, 0.65);
  display: grid;
  place-items: center;
  transition:
    background 0.15s ease,
    border-color 0.15s ease;
}

.tp-box-checked {
  background: var(--accent, #7c6cff);
  border-color: var(--accent, #7c6cff);
}

.tp-check {
  width: 11px;
  height: 11px;
}

.tp-check polyline {
  fill: none;
  stroke: #fff;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-dasharray: 16;
  stroke-dashoffset: 16;
  animation: tp-draw 0.26s ease-out forwards;
}

@keyframes tp-draw {
  to {
    stroke-dashoffset: 0;
  }
}

.tp-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tp-done {
  text-decoration: line-through;
  opacity: 0.45;
}

.tp-empty {
  padding: 8px 2px;
  opacity: 0.5;
}
</style>
