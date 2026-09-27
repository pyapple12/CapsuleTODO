<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref } from "vue";
import type { TodoItem, TodoView } from "../../types";
import DelButton from "./DelButton.vue";
import NeonCheckbox from "./NeonCheckbox.vue";
import { invoke } from "@tauri-apps/api/core";
import { rowMaskDead, syncMaskDead } from "../composables/useMaskDead";
import { useBoardRead } from "../composables/useBoardRead";
import { useGlassBar } from "../composables/useGlassBar";
import { deferDuringDrag, installDragHooks, isSuppressed } from "../composables/useDragReorder";

// ===== 清单页（PL010.4 换装实验场形态）：行模板 1:1（NeonCheckbox + t-text +
// DelButton 二态 + age-alert 黄红提醒）+ TransitionGroup 出入场（0.3s 显式 duration——
// 遮挡 webview 不派发动画事件）+ 罩死禁交互 + DelButton 单实例收口。
// PL013.3a 补挂点：ul 补 id=todo-active（useMaskDead/useVeils/drag 机制按 id 找容器）+
// 玻璃滑杆（inset）与整板阅读（skipDuringDrag）——design 主窗挂点补齐 =====

const props = defineProps<{
  /** 清单排序视图（父级经 todo_list 拉取；含龄期档位） */
  items: TodoView[];
}>();

const emit = defineEmits<{ changed: []; openDetail: [item: TodoItem] }>();

const listEl = ref<HTMLElement | null>(null);

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
  if (isSuppressed()) return; // 拖拽落点抑制（useDragReorder 350ms 窗口）
  if (rowMaskDead(rowEl(item.id))) return;
  clearTimeout(clickTimer);
  clickTimer = window.setTimeout(() => emit("openDetail", item), 180);
}

function onRowDblClick(item: TodoItem): void {
  clearTimeout(clickTimer); // 掐掉未决的开板定时器
  startInlineEdit(item);
}

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

// 内容增删后罩死复核（滚动监听经 useGlassBar 钩子已通；此处补重渲染路径）。
// Vue 冻结防线（PL013 红线）：拖拽 engaged 期间外部 items 到达不重渲染——重挂 DOM
// 与虚拟 DOM 打架防线；落点收场 rerender 统一重拉
const stopMask = computed(() => {
  syncMaskDead();
  if (deferDuringDrag()) return -1; // 冻结期哨兵值（不参与正常语义）
  return props.items.length;
});
void stopMask;

// 拖拽钩子安装（本组件只装一次——气泡组件重复调用幂等覆盖，钩子语义一致）
const listKey = ref(0); // 强制重建计数：拖拽收场 vnode↔DOM 断链修复（见 useDragReorder）
installDragHooks({
  cancelPendingClick: () => clearTimeout(clickTimer),
  markSuppress: () => {}, // suppress 时间戳在 useDragReorder 模块内自持，无需写回
  rerenderTodos: () => {
    emit("changed"); // 收场重拉（父级三源齐拉含本列表）
  },
  rerenderBubbles: () => {}, // 气泡路 rerender 由 BubblesView 覆盖注册
  forceRemount: () => {
    listKey.value += 1; // TransitionGroup 整列表重建（重挂节点被外部摘除，Vue 不感知）
  },
});

// 主窗挂点（design glass-bar.js 同款）：滑杆 inset + 整板阅读 skipDuringDrag。
// TransitionGroup 渲染后 ul 才存在——nextTick 后挂；items 空时 ul 不渲染，空→非空
// 重建的 ul 失去挂载？TransitionGroup v-else 分支切换会重建元素——用 watch 补挂
let glassBar: { sync: () => void } | null = null;

/** 挂滑杆与整板阅读（ul 已在 DOM 时执行；幂等——只挂一次）。boardRead 返回值
 * 挂注册表（useVeils/useMaskDead 遍历），本地不再持句柄 */
function mountScrollKit(): void {
  // TransitionGroup 的 template ref 指向组件实例——真实 UL 须经 $el 取（PL011 同教训）
  const ul =
    (listEl.value as unknown as { $el?: HTMLElement })?.$el ?? (listEl.value as HTMLElement | null);
  if (!ul || glassBar) return;
  ul.dataset.mounted = "1";
  glassBar = useGlassBar(ul, { inset: true });
  useBoardRead(ul, { skipDuringDrag: true });
}

void nextTick(mountScrollKit);
// 空→非空重建 ul 后补挂（glassBar 非空说明旧实例已挂——TransitionGroup v-else 切换
// 重建的元素失去监听，须重挂：先卸旧再挂新）
watchListRebuild();

/** 监听列表重建（v-else 切换使 ul 换元素）：卸旧实例重挂。setup 同步段 .list 尚未
 * 入 DOM、mock invoke 返回又早于 50ms 重试——observer 注册晚于首场渲染即死等（冷启动
 * 实测）。改为 rAF 链轮询：每帧查 host + 未挂则直挂，挂上后切换 observer 只管重建 */
function watchListRebuild(): void {
  const host = document.querySelector(".list");
  if (!host) {
    requestAnimationFrame(watchListRebuild); // 组件 DOM 未挂：下一帧再看（不设上限——挂载是必然事件）
    return;
  }
  rebuildObserver = new MutationObserver(() => {
    const ul =
      (listEl.value as unknown as { $el?: HTMLElement })?.$el ??
      (listEl.value as HTMLElement | null);
    if (!ul) return;
    if (!glassBar) {
      // 首挂早退（setup 时 items 未到、listEl 为 null）后的补挂口：items 渲染真实 ul
      // 后任何子树变化都会走到这里
      mountScrollKit();
    } else if (ul.dataset.mounted !== "1") {
      // ul 已换新元素：重挂
      glassBar = null;
      mountScrollKit();
    }
  });
  rebuildObserver.observe(host, { childList: true });
  // observer 就位前渲染可能已完成（最后一场变化没人接）：host 在手直接补一次挂
  mountScrollKit();
}

let rebuildObserver: MutationObserver | null = null;

onUnmounted(() => {
  clearTimeout(clickTimer);
  rebuildObserver?.disconnect();
});
</script>

<template>
  <section class="list">
    <p v-if="items.length === 0" class="empty">暂无待办，添加一条吧</p>
    <TransitionGroup
      v-else
      ref="listEl"
      tag="ul"
      name="todo"
      id="todo-active"
      class="group"
      :key="listKey"
      :duration="320"
    >
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

<!-- 行样式已全局挂载（styles/todos.css 经 main.ts）——归档行复用同款需全局作用域 -->
