<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
// IPC DTO 镜像类型统一收敛在 types.ts（单一来源 = Rust serde 结构，防多处声明漂移）
import type { BubbleItem, BubbleSnapshot, TodoItem, TodoView } from "./types";
import AddBar from "./src/components/AddBar.vue";
import TabsBar from "./src/components/TabsBar.vue";
import TodoList from "./src/components/TodoList.vue";
import DetailOverlay from "./src/components/DetailOverlay.vue";
import ArchiveOverlay from "./src/components/ArchiveOverlay.vue";
import SettingsOverlay from "./src/components/SettingsOverlay.vue";
import BubblesView from "./src/components/BubblesView.vue";
import WhiteboardView from "./src/components/WhiteboardView.vue";
import { initTitleParticles } from "./src/composables/titleParticles";
import { useThresholdDrag } from "./src/composables/useThresholdDrag";
import { useDragReorder } from "./src/composables/useDragReorder";
import { rollbackAllDelConfirms } from "./src/composables/delConfirmBus";
import { syncGlassBars } from "./src/composables/useGlassBar";
import { syncHints } from "./src/composables/useBoardRead";

const titleEl = ref<HTMLHeadingElement | null>(null);
const topbarEl = ref<HTMLElement | null>(null);
// 粒子引擎句柄：设置板换主题（accent 变色）后 refresh 换色不重建（2026-09-30
// 资源定案：重建会重播开场汇聚动画 = 切主题资源峰值主因）
// （shallowRef：volar 对裸 let 的模板收窄会把回调内赋值判成 never，实测 TS2339）
const titleFX = shallowRef<{ refresh: () => void } | null>(null);

// 拖拽机制安装（PL013：document 级 mousedown/mousemove/mouseup + blur 收尾，
// App 顶层一次安装，机制对全部 DRAG_TARGETS 生效）
useDragReorder();

// PL011 管线：分态纱浓度由 .focused class 驱动——初值经 isFocused 查询兜底，
// 此后随 Rust 的 window-focus 事件翻转（窗口恒纯 alpha 透明，2026-09-28 定案）
const windowFocused = ref(false);
let unlistenFocus: UnlistenFn | undefined;
// 关窗 T 腿句柄（FIX004.19 T+R3 双腿）：白板组件引用供关窗前强制落库
let unlistenClose: UnlistenFn | undefined;
// 热键失焦刷新句柄（PL016.1）：徽章随 bubble-changed 事件实时 +1
let unlistenBubbleChanged: UnlistenFn | undefined;
// 托盘预览打勾同步句柄（PL018.6）：主窗随 todo-changed 刷新
let unlistenTodoChanged: UnlistenFn | undefined;
// 窗口偏好回显句柄（PL021）：托盘菜单切开关广播 prefs-changed → 设置板同步
let unlistenPrefsChanged: UnlistenFn | undefined;
const whiteboardRef = ref<InstanceType<typeof WhiteboardView> | null>(null);

// 二期三页签（PL004）：清单（一期功能）/ 气泡（临时剪贴板）/ 白板（临时草稿）
type TabKey = "todos" | "bubbles" | "whiteboard";
const activeTab = ref<TabKey>("todos");

// 页签徽章：气泡未读计数（PL008.4 接 mock/真实 bubble_list 长度；≥10 显示 9+）
const bubbleCount = ref(0);

/** 拉取气泡徽章计数（复用 BubbleSnapshot 契约类型，items 长度即条目数，与页内同源） */
async function refreshBadge(): Promise<void> {
  try {
    const snap = await invoke<BubbleSnapshot>("bubble_list");
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
  // 换页重算挂点（design glass-bar.js 换页监听同款）：白板页 v-show 常驻不卸载，
  // 切走时锚层滑杆/三角无人驱动——nextTick 等 display 生效后逐实例 sync，
  // 宿主 rect 归零即自动隐藏，切回白板页按新几何恢复
  void nextTick(() => {
    syncGlassBars();
    syncHints();
  });
}

// —— 详情板（PL010.5）：行单击 openDetail 上抛 → 置 detailTodo 开板 ——

const detailTodo = ref<TodoItem | null>(null);
const detailBubble = ref<BubbleItem | null>(null);
// 气泡全文板数据源（PL012.3）：与 detailTodo 互斥（一开一关）
// 飞出原点（A3）：被点行中心视口坐标，DetailOverlay 开板时换算板内 origin
const detailAnchor = ref<{ x: number; y: number } | null>(null);

// 三板互斥（design panels.js 定案）：任一板开启即收其余两板；开详情（行/气泡点击）同理
const archiveRef = ref<InstanceType<typeof ArchiveOverlay> | null>(null);
const settingsRef = ref<InstanceType<typeof SettingsOverlay> | null>(null);

function closeDetail(): void {
  detailTodo.value = null;
  detailBubble.value = null;
}

/** 行单击开详情板：以最新视图中的条目为数据源（防陈旧）+ 行中心作飞出原点 */
function onOpenDetail(item: TodoItem, anchor: { x: number; y: number }): void {
  archiveRef.value?.close();
  settingsRef.value?.close();
  detailAnchor.value = anchor;
  detailTodo.value = items.value.find((it) => it.id === item.id) ?? item;
}

/** 气泡行单击开全文板：置 detailBubble（bubble-mode 单层玻璃只读）+ 行中心原点 */
function onOpenBubble(item: BubbleItem, anchor: { x: number; y: number }): void {
  archiveRef.value?.close();
  settingsRef.value?.close();
  detailAnchor.value = anchor;
  detailBubble.value = item;
}

/** 勾选入档联动（A7 = design 勾选分支同款）：详情板正开着这条则随行收起 */
function onArchived(item: TodoItem): void {
  if (detailTodo.value?.id === item.id) closeDetail();
}

// —— 归档板（PL011）：数据源 + 变更重拉 —— 非清单页隐藏归档按钮（V0.015 定案：
// 归档仅清单页生效），组件经 ref expose 的 toggle 由按钮内部自管 ——

const archiveItems = ref<TodoItem[]>([]);

/** 拉取归档视图（板开着时才可见，惰性拉取零浪费） */
async function refreshArchive(): Promise<void> {
  try {
    archiveItems.value = await invoke<TodoItem[]>("todo_archive_list");
  } catch (err) {
    console.error("todo_archive_list 拉取失败", err);
  }
}

/** 清单变更统一出口：重拉清单/归档/徽章三源 + 三板互斥（开详情板时归档收）。
 * 拖拽期冻结防线挂组件层（TodoList/BubblesView 列表 watch deferDuringDrag 早退），
 * 此处不设守卫——收场 rerender 即重拉（forceRemount 已修 vnode 断链，时序安全） */
function onListChanged(): void {
  void refresh();
  void refreshArchive();
  void refreshBadge();
}

/** 新增待办收口：统一刷新 + 清单滚动回顶（新行置顶定案的配套——无论滚动在哪，
 * 新条目必在顶部，拉顶保证立即可见；平滑滚动避免硬跳） */
function onTodoAdded(): void {
  onListChanged();
  document.querySelector("#page-todos .group")?.scrollTo({ top: 0, behavior: "smooth" });
}

// —— 三板互斥出口（design panels.js：两板互斥 + 详情板三方互斥） ——

/** 归档板开启：收设置板 + 详情板；开合都收口未决删除确认（A2 = design setBoard 首行） */
function onArchiveOpened(): void {
  settingsRef.value?.close();
  closeDetail();
}

/** 设置板开启：收归档板 + 详情板 */
function onSettingsOpened(): void {
  archiveRef.value?.close();
  closeDetail();
}

// 气泡提醒数量（PL014.2 持久化）：启动自 config.json 加载，设置板步进落库
const maxBubbles = ref(5);
// 窗口偏好（PL017 持久化）：置顶与贴边吸附开关，设置板 toggle 落库
const alwaysOnTop = ref(true);
const snapToEdge = ref(true);

/** 拉取气泡提醒上限（启动时初始化；失败保持默认 5） */
async function refreshMaxBubbles(): Promise<void> {
  try {
    maxBubbles.value = await invoke<number>("settings_get_max_bubbles");
  } catch (err) {
    console.error("settings_get_max_bubbles 拉取失败", err);
  }
}

/** 拉取窗口偏好开关（PL017：置顶/贴边吸附；失败保持默认 true） */
async function refreshWindowPrefs(): Promise<void> {
  try {
    alwaysOnTop.value = await invoke<boolean>("settings_get_always_on_top");
    snapToEdge.value = await invoke<boolean>("settings_get_snap_to_edge");
  } catch (err) {
    console.error("窗口偏好拉取失败", err);
  }
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

// 切页编排（design tabs.js 同款）：回清单页重拉兜底 + 归档按钮出入场动画
// （离清单页塌缩+粒子迸裂、回清单页粒子汇聚+回弹弹出——仅此处编排，与板开合解耦）；
// 切页收口未决删除确认（A2 = design tabs.js rollbackDelConfirms）
watch(activeTab, (tab) => {
  rollbackAllDelConfirms();
  if (tab === "todos") {
    void refresh();
  }
  archiveRef.value?.setArchiveVisible(tab === "todos");
});

onMounted(async () => {
  unlistenFocus = await listen<boolean>("window-focus", (event) => {
    windowFocused.value = event.payload;
  });
  // 热键失焦刷新（PL016.1）：Rust 热键入库发 bubble-changed → 徽章实时 +1
  //（气泡页内刷新由 BubblesView 自身的同款监听负责）
  unlistenBubbleChanged = await listen("bubble-changed", () => {
    void refreshBadge();
  });
  // 托盘预览打勾同步（PL018.6）：tray-preview 窗勾选落库后广播 → 主窗刷新链
  unlistenTodoChanged = await listen("todo-changed", () => {
    onListChanged();
  });
  // 窗口偏好回显（PL021，补齐 PL018.3 三入口双向）：托盘菜单切开关落库后广播
  // → 设置板 ref 同步刷新（托盘侧勾选态由 TrayMenu.vue 自身监听同款事件负责）
  unlistenPrefsChanged = await listen<{ always_on_top: boolean; snap_to_edge: boolean }>(
    "prefs-changed",
    (event) => {
      alwaysOnTop.value = event.payload.always_on_top;
      snapToEdge.value = event.payload.snap_to_edge;
    },
  );
  getCurrentWindow()
    .isFocused()
    .then((focused) => {
      windowFocused.value = focused;
    })
    .catch((err) => console.error("isFocused 查询失败（纱态保持透明态默认）", err));
  // 关窗 T 腿（FIX004.19 T+R3 双腿，探针实测推翻 V0.1.1.1"关闭挂起"旧结论——健康
  // webview + 正确用法不挂，实测记录 .temp/close-probe/）：关窗请求先过白板 flush
  // （组件暴露的强制落库），完成即自动销毁（正常态秒关）；webview 卡死时 Rust R3 腿
  // 600ms 超时强关兜底（lib.rs）。注册失败落日志——纯浏览器冒烟环境无窗口事件，
  // 关窗链路仍由 R3 腿保证
  try {
    unlistenClose = await getCurrentWindow().onCloseRequested(async () => {
      await whiteboardRef.value?.flush();
    });
  } catch (err) {
    console.error("onCloseRequested 注册失败（关窗依赖 Rust 600ms 兜底腿）", err);
  }
  // 白板数据安全 = 800ms 防抖自动保存（组件常驻挂载，计时器切页不中断）；
  await refresh();
  await refreshBadge();
  await refreshArchive();
  await refreshMaxBubbles();
  await refreshWindowPrefs();
  // 标题粒子化（design text-particles.js 移植）：reduced-motion 下不初始化回退静态文字
  if (titleEl.value) titleFX.value = initTitleParticles(titleEl.value);
  // 标题阈值拖拽：单击不吞 click（浮板可点标题关闭），按住移动才拖窗
  if (topbarEl.value) useThresholdDrag(topbarEl.value);
});

onUnmounted(() => {
  unlistenFocus?.();
  unlistenClose?.();
  unlistenBubbleChanged?.();
  unlistenTodoChanged?.();
  unlistenPrefsChanged?.();
});
</script>

<template>
  <!-- PL008.3 骨架 Vue 化（design/index.html 对应）：id=board 保留为卡片层锚点——
       浮板/滑杆/三角/拖拽重挂全部以它为宿主（useVeils/useGlassBar/useBoardRead 依赖）；
       拖动收敛 topbar：交互区（页签/行/输入）不再依赖白名单排除，误触面归零 -->
  <main id="board" class="glass-card" :class="{ focused: windowFocused }">
    <header ref="topbarEl" class="topbar">
      <h1 ref="titleEl" class="title">
        CapsuleTODO<canvas class="title-canvas" aria-hidden="true"></canvas>
      </h1>
    </header>
    <nav class="tabs-slot">
      <TabsBar :model-value="activeTab" :tabs="tabDefs" @update:model-value="onTabChange" />
    </nav>
    <div v-if="activeTab === 'todos'" class="page" id="page-todos">
      <AddBar @added="onTodoAdded" />
      <TodoList
        :items="items"
        @changed="onListChanged"
        @open-detail="onOpenDetail"
        @archived="onArchived"
      />
    </div>
    <div v-if="activeTab === 'bubbles'" class="page" id="page-bubbles">
      <BubblesView :max-bubbles="maxBubbles" @open-bubble="onOpenBubble" @changed="refreshBadge" />
    </div>
    <!-- 白板页常驻挂载（v-show）：组件内草稿状态不因切页丢失 -->
    <div v-show="activeTab === 'whiteboard'" class="page" id="page-whiteboard">
      <WhiteboardView ref="whiteboardRef" />
    </div>
    <!-- 归档板（PL011）：按钮常驻清单页左上角（切页由 archive-fx 出入场动画编排，
         非清单页塌缩隐藏）；v-if 会销毁按钮出入场状态，故常驻挂载 -->
    <ArchiveOverlay
      ref="archiveRef"
      :items="archiveItems"
      @changed="onListChanged"
      @opened="onArchiveOpened"
    />
    <!-- 设置板（PL014 前置）：齿轮开合 + 主题双开关 + 气泡提醒步进；互斥经 opened 上抛 -->
    <SettingsOverlay
      ref="settingsRef"
      v-model:max-bubbles="maxBubbles"
      v-model:always-on-top="alwaysOnTop"
      v-model:snap-to-edge="snapToEdge"
      @opened="onSettingsOpened"
      @theme-changed="titleFX?.refresh()"
    />
    <!-- 详情板（PL010.5）：todo 非 null 即开；三板互斥由其内部 syncVeils 联动；
         anchor = 被点行中心（A3 飞出原点）。close 双清——只清 bubble 时同一条 todo
         再点击会因 watch 同引用不触发而开不了板（真窗口实测 2026-09-28） -->
    <DetailOverlay
      :todo="detailTodo"
      :bubble="detailBubble"
      :anchor="detailAnchor"
      @changed="onListChanged"
      @close="closeDetail"
    />
  </main>
</template>

<style>
/* —— 设计令牌：PL008.2 起收敛到 ui/src/styles/glass.css（design/glass.css 1:1 落位，
   单一来源设计文档化），本文件不再内联令牌—— */
@import "./src/styles/glass.css";
</style>

<style scoped>
/* 玻璃板（全窗单层）：恒纯 alpha 透明 + 30% 纱（浅白/暗黑双主题，保可读）+ 亮边
   + rim + 落影；聚焦态纱退 0%（透明度靠背后桌面透出）；8px 圆角对齐系统窗口圆角 */
.glass-card {
  position: relative;
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100vh;
  padding: 18px 18px 20px; /* design 定案：顶 18 = 标题距卡顶；左右 18、底 20 */
  box-sizing: border-box;
  border-radius: var(--radius-board);
  box-shadow: var(--glass-stroke), var(--rim-light), var(--shadow-candy);
  overflow: hidden;
  text-shadow: var(--text-shadow);
  user-select: none;
  color: var(--ink);
  font-family: var(--font-stack);
}

/* 纱体色层：叠在透明底之上、内容之下；分态浓度（PL011 用户定案）——
   失焦透明态 30% 纱保可读，聚焦态退 0%（恒纯 alpha 定案，无背板磨砂） */
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

/* 流内直接子件浮于纱层之上（显式列举——通配 `.glass-card > *` 会以 scoped 高特异性
   压掉浮层组件（归档/详情/按钮/滑杆）的 position:absolute + z-index，实测图标被
   顶进 flex 流后"完全看不见"——2026-09-27 用户截图定案修复） */
.topbar,
.tabs-slot,
.page {
  position: relative;
  z-index: 1;
}

.topbar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center; /* 标题居中（design 定案） */
  margin-bottom: 15.5px; /* 墨迹底 → 页签 18px 节奏（design base.css 同值） */
  cursor: default; /* 拖动区光标语义（data-tauri-drag-region 按下即窗口拖拽） */
}

.title {
  position: relative; /* 粒子画布定位基准 */
  margin: 0;
  font-size: 30px; /* 放大一倍（原 15px，design 用户定案） */
  line-height: 1; /* 盒高=字高：排版内衬清零 */
  font-weight: 700; /* 加粗（design 用户定案） */
  letter-spacing: 0.3px;
  opacity: 0.8;
  cursor: default;
}

/* 标题粒子化：画布接管视觉，原文字透明让位（reduced-motion 不挂此类回退静态文字） */
.title--particles {
  color: transparent;
  text-shadow: none; /* 杀掉继承的字形影：透明文字仍投静态残影（design 定案） */
}

.title-canvas {
  position: absolute;
  left: -30px; /* 画布外扩余量：粒子散开/辉光侧摆不被裁 */
  top: -20px; /* 与粒子引擎 MARGIN_TOP 联动 */
  pointer-events: none; /* 事件穿透，交互监听在 window 上 */
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
