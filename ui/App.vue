<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
// IPC DTO 镜像类型统一收敛在 types.ts（单一来源 = Rust serde 结构，防多处声明漂移）
import type { TodoItem, TodoView } from "./types";
import AddBar from "./src/components/AddBar.vue";
import TabsBar from "./src/components/TabsBar.vue";
import TodoList from "./src/components/TodoList.vue";
import DetailOverlay from "./src/components/DetailOverlay.vue";
import BubblesView from "./components/BubblesView.vue";
import WhiteboardView from "./components/WhiteboardView.vue";

// PL011 管线：分态纱浓度由 .focused class 驱动——初值经 isFocused 查询兜底，
// 此后随 Rust 的 window-focus 事件翻转（Rust 侧 Focused 分支同步切 DWM 背板）
const windowFocused = ref(false);
let unlistenFocus: UnlistenFn | undefined;

// 二期三页签（PL004）：清单（一期功能）/ 气泡（临时剪贴板）/ 白板（临时草稿）
type TabKey = "todos" | "bubbles" | "whiteboard";
const activeTab = ref<TabKey>("todos");

// 页签徽章：气泡未读计数（PL008.4 接 mock/真实 bubble_list 长度；≥10 显示 9+）
const bubbleCount = ref(0);

/** 拉取气泡徽章计数（snapshot.items 长度即条目数，与页内显示同源） */
async function refreshBadge(): Promise<void> {
  try {
    const snap = await invoke<{ items: unknown[] }>("bubble_list");
    bubbleCount.value = snap.items.length;
  } catch (err) {
    console.error("bubble_list 徽章计数拉取失败", err);
  }
}

/** 页签定义（TabsBar props）：徽章挂气泡页；whiteboard 键也要在列（glider 定位序号）。
 * computed 保持徽章计数响应式——静态数组写 badgeCount: bubbleCount.value 是一次性
 * 快照，ref 更新后不回写（实测徽章不显示的根因） */
const tabDefs = computed(() => [
  { key: "todos", label: "清单" },
  { key: "bubbles", label: "气泡", badgeCount: bubbleCount.value },
  { key: "whiteboard", label: "白板" },
]);

/** TabKey 兼容 TabsBar 字符串 key（v-model 回写收敛回三键类型） */
function onTabChange(key: string): void {
  activeTab.value = key as TabKey;
}

// —— 详情板（PL010.5）：行单击 openDetail 上抛 → 置 detailTodo 开板 ——

const detailTodo = ref<TodoItem | null>(null);

/** 行单击开详情板：以最新视图中的条目为数据源（防陈旧） */
function onOpenDetail(item: TodoItem): void {
  detailTodo.value = items.value.find((it) => it.id === item.id) ?? item;
}

/** 清单变更统一出口：重拉视图 + 保留详情板打开时的条目同步 */
function onListChanged(): void {
  void refresh();
}

// 清单数据源：挂载拉取 + 动作后重拉（排序视图由 Rust 侧裁决，前端无轮询——无计时需求）
const items = ref<TodoView[]>([]);

/** 拉取清单排序视图（todo_list 返回 TodoView：条目 + 龄期档位，PL010 起） */
async function refresh(): Promise<void> {
  try {
    items.value = await invoke<TodoView[]>("todo_list");
  } catch (err) {
    console.error("todo_list 拉取失败", err);
  }
}

// 切回清单页时重拉（动作都伴随刷新，此处兜底其他来源的变更）
watch(activeTab, (tab) => {
  if (tab === "todos") {
    void refresh();
  }
});

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
  // 白板数据安全 = 800ms 防抖自动保存（组件常驻挂载，计时器切页不中断）；
  // 不挂 JS onCloseRequested——实测该 API 会把关闭权移交 webview destroy 路径导致关闭挂起
  await refresh();
  await refreshBadge();
});

onUnmounted(() => {
  unlistenFocus?.();
});
</script>

<template>
  <!-- PL008.3 骨架 Vue 化（design/index.html 对应）：id=board 保留为卡片层锚点——
       浮板/滑杆/三角/拖拽重挂全部以它为宿主（useVeils/useGlassBar/useBoardRead 依赖）；
       拖动收敛 topbar：交互区（页签/行/输入）不再依赖白名单排除，误触面归零 -->
  <main id="board" class="glass-card" :class="{ focused: windowFocused }">
    <header class="topbar" data-tauri-drag-region>
      <h1 class="title" data-tauri-drag-region>CapsuleTODO</h1>
    </header>
    <nav class="tabs-slot">
      <TabsBar :model-value="activeTab" :tabs="tabDefs" @update:model-value="onTabChange" />
    </nav>
    <div v-if="activeTab === 'todos'" class="page" id="page-todos">
      <AddBar @changed="refresh" />
      <TodoList :items="items" @changed="onListChanged" @open-detail="onOpenDetail" />
    </div>
    <div v-if="activeTab === 'bubbles'" class="page" id="page-bubbles">
      <BubblesView />
    </div>
    <!-- 白板页常驻挂载（v-show）：组件内草稿状态不因切页丢失 -->
    <div v-show="activeTab === 'whiteboard'" class="page" id="page-whiteboard">
      <WhiteboardView />
    </div>
    <!-- 详情板（PL010.5）：todo 非 null 即开；三板互斥由其内部 syncVeils 联动 -->
    <DetailOverlay :todo="detailTodo" @changed="onListChanged" @close="detailTodo = null" />
  </main>
</template>

<style>
/* —— 设计令牌：PL008.2 起收敛到 ui/src/styles/glass.css（design/glass.css 1:1 落位，
   单一来源设计文档化），本文件不再内联令牌—— */
@import "./src/styles/glass.css";
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
  cursor: default; /* 拖动区光标语义（data-tauri-drag-region 按下即窗口拖拽） */
}

.title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  letter-spacing: 0.3px;
  opacity: 0.8;
  cursor: default;
}

/* 页签槽位（TabsBar 组件自带 .tabs 样式，此处只占位） */
.tabs-slot {
  flex-shrink: 0;
}

/* 页容器：占满页签以下空间，滚动交给页内列表 */
.page {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 100%;
  min-height: 0;
  flex: 1;
}

.placeholder {
  margin: 24px 0;
  font-size: 13px;
  text-align: center;
  opacity: 0.5;
}
</style>
