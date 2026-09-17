<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
// IPC DTO 镜像类型统一收敛在 types.ts（单一来源 = Rust serde 结构，防多处声明漂移）
import type { TodoItem } from "./types";
import AddBar from "./components/AddBar.vue";
import TodoList from "./components/TodoList.vue";

// PL011 管线：分态纱浓度由 .focused class 驱动——初值经 isFocused 查询兜底，
// 此后随 Rust 的 window-focus 事件翻转（Rust 侧 Focused 分支同步切 DWM 背板）
const windowFocused = ref(false);
let unlistenFocus: UnlistenFn | undefined;

// 清单数据源：挂载拉取 + 动作后重拉（排序视图由 Rust 侧裁决，前端无轮询——无计时需求）
const items = ref<TodoItem[]>([]);

/** 拉取清单排序视图 */
async function refresh(): Promise<void> {
  try {
    items.value = await invoke<TodoItem[]>("todo_list");
  } catch (err) {
    console.error("todo_list 拉取失败", err);
  }
}

onMounted(async () => {
  unlistenFocus = await listen<boolean>("window-focus", (event) => {
    windowFocused.value = event.payload;
  });
  getCurrentWindow()
    .isFocused()
    .then((focused) => {
      windowFocused.value = focused;
    })
    .catch((err) => console.error("isFocused 查询失败（纱态保持透明态默认）", err));
  await refresh();
});

onUnmounted(() => {
  unlistenFocus?.();
});

// 拖动方案（沿 Pulse PL010 定案）：交互元素白名单命中不抢，其余一律启动窗口拖拽
// （.todo-row 整行点击 = 勾选，须入白名单；白名单随组件演进维护）
const DRAG_INTERACTIVE = "button, input, textarea, label, .todo-row";

/** 非交互区按下即启动窗口拖拽 */
function onWindowDown(e: MouseEvent): void {
  if (e.button !== 0) {
    return;
  }
  const target = e.target as HTMLElement | null;
  if (target?.closest(DRAG_INTERACTIVE)) {
    return;
  }
  void getCurrentWindow().startDragging();
}
</script>

<template>
  <main class="glass-card" :class="{ focused: windowFocused }" @mousedown="onWindowDown">
    <header class="topbar">
      <h1 class="title">CapsuleTODO</h1>
    </header>
    <AddBar @changed="refresh" />
    <TodoList :items="items" @changed="refresh" />
  </main>
</template>

<style>
/* —— 设计令牌（全局唯一来源）：沿系列玻璃配方（对照 CapsulePulse PL010 令牌表移植）。
   真实玻璃材质：高透薄纱体色 + 亮边 + 顶缘 rim + 落影；磨砂由 DWM Acrylic 背板承担（焦点联动）—— */
:root {
  --accent: #7c3aed;
  --ink: #1d1d1f;
  --ink-2: color-mix(in srgb, #1d1d1f 55%, transparent);
  --font-stack: "SF Pro Display", "Segoe UI Variable Display", "Segoe UI", sans-serif;
  --glass-bg: rgba(255, 255, 255, 0.3);
  --glass-stroke: inset 0 0 0 1.5px rgba(255, 255, 255, 0.78);
  --panel-bg: rgba(255, 255, 255, 0.38);
  --panel-stroke: inset 0 0 0 1px rgba(255, 255, 255, 0.65);
  --text-shadow: none;
  --glass-highlight:
    inset 0 1px rgba(255, 255, 255, 0.35), inset 0 0 0 0.5px rgba(255, 255, 255, 0.16);
  --rim-light: inset 0 1.5px 0 rgba(255, 255, 255, 0.9);
  --shadow-candy: 0 16px 40px rgba(80, 60, 120, 0.25);
}

@media (prefers-color-scheme: dark) {
  :root {
    /* 暗夜衍生：同一配方的低透深纱版（浅字 + 暗投影保对比），磨砂仍由 DWM Acrylic 背板承担 */
    --accent: #c0b0fd;
    --ink: #f5f5f7;
    --ink-2: color-mix(in srgb, #f5f5f7 55%, transparent);
    --glass-bg: rgba(0, 0, 0, 0.3);
    --glass-stroke: inset 0 0 0 1.5px rgba(255, 255, 255, 0.28);
    --panel-bg: rgba(255, 255, 255, 0.07);
    --panel-stroke: inset 0 0 0 1px rgba(255, 255, 255, 0.14);
    --text-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
    --glass-highlight:
      inset 0 1px rgba(255, 255, 255, 0.12), inset 0 0 0 0.5px rgba(255, 255, 255, 0.1);
    --rim-light: inset 0 1.5px 0 rgba(255, 255, 255, 0.32);
    --shadow-candy: 0 16px 40px rgba(0, 0, 0, 0.5);
  }
}
</style>

<style scoped>
/* 玻璃板（全窗单层 / PL011 焦点联动材质）：聚焦 = DWM Acrylic 背板真磨砂（0% 纱），
   失焦 = 纯 alpha 透明 + 30% 纱（浅白/暗黑双主题，保可读）+ 亮边 + rim + 落影；
   8px 圆角对齐系统窗口圆角 */
.glass-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: 100%;
  height: 100vh;
  padding: 20px 18px;
  box-sizing: border-box;
  border-radius: 8px;
  box-shadow: var(--glass-stroke), var(--rim-light), var(--shadow-candy);
  overflow: hidden;
  text-shadow: var(--text-shadow);
  user-select: none;
  color: var(--ink);
  font-family: var(--font-stack);
}

/* 纱体色层：叠在 DWM 背板/透明底之上、内容之下；分态浓度（PL011 用户定案）——
   失焦透明态 30% 纱保可读，聚焦磨砂态退 0%（磨砂已足够） */
.glass-card::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: 1;
  border-radius: inherit;
  background: var(--glass-bg);
  pointer-events: none;
}

.glass-card.focused::before {
  background: transparent;
}

/* 直接子件一律浮于体色层之上 */
.glass-card > * {
  position: relative;
  z-index: 1;
}

.topbar {
  flex-shrink: 0;
}

.title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  letter-spacing: 0.3px;
  opacity: 0.8;
  cursor: default;
}
</style>
