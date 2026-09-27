<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { BubbleItem, BubbleSnapshot } from "../../types";
import DelButton from "./DelButton.vue";
import { rowMaskDead } from "../composables/useMaskDead";
import { syncVeils } from "../composables/useVeils";
import { useBoardRead } from "../composables/useBoardRead";
import { useGlassBar } from "../composables/useGlassBar";
import { installDragHooks, isSuppressed } from "../composables/useDragReorder";

// ===== 气泡页（PL012.2 换装实验场形态 V0.017–V0.020 + V0.028 对调语义）：
// 捕获钮（clipboard 双图标 + 占字 1s 反馈）/ 一键清空二态（宽度动画 + 悬停感知 2s
// 超时 + 旁路取消）/ 满仓警告红字（has-warning 两档偏移 + 隐区位移在 useBoardRead）/
// 行（两行截断 + DelButton 二态 + 单击开板双击复制 180ms 消歧）。数值全沿实测定案 =====

const emit = defineEmits<{ openBubble: [item: BubbleItem]; changed: [] }>();

const props = defineProps<{
  /** 满仓警告阈值（设置板步进同源，父级持有；超过才警告——design 定案语义） */
  maxBubbles: number;
}>();

const items = ref<BubbleItem[]>([]);
const error = ref("");
const copied = ref(false); // 占字态：捕获钮禁点 + 换 clipboard-check 图标文案
const confirmingClear = ref(false);
let copiedTimer = 0;
let confirmTimer = 0;
let clearWidthTimer = 0;
let clickTimer: number | undefined;

const clearBtn = ref<HTMLElement | null>(null);
const listEl = ref<HTMLElement | null>(null);

/** 满 maxBubbles 警告显隐：超过阈值（非达到）才警告——design bubbles.js 同款语义；
 * 阈值由设置板步进（会话内有效）。snapshot.remind（Rust 满 5 裁决）不再消费，
 * 持久化配置落位时（PL014）再议 */
const hasWarning = computed(() => items.value.length > props.maxBubbles);

/** 拉取气泡快照（changed 上抛：父级同步页签徽章——捕获/删除/清空都走这里） */
async function refresh(): Promise<void> {
  try {
    const snapshot = await invoke<BubbleSnapshot>("bubble_list");
    items.value = snapshot.items;
    emit("changed");
  } catch (err) {
    console.error("bubble_list 拉取失败", err);
  }
}

// 主窗挂点（design glass-bar.js 同款）：滑杆 inset + 整板阅读（skipDuringDrag + maskShift）。
// TransitionGroup v-else 切换：空→非空重建 ul 须重挂——沿 TodoList 的 observer 方案
let glassBar: { sync: () => void } | null = null;

/** 挂滑杆与整板阅读（ul 已在 DOM 时执行；幂等——只挂一次） */
function mountScrollKit(): void {
  const ul = (listEl.value as unknown as { $el?: HTMLElement })?.$el ?? listEl.value;
  if (!ul || glassBar) return;
  glassBar = useGlassBar(ul, { inset: true });
  useBoardRead(ul, {
    rowSel: ".bubble-row",
    skipDuringDrag: true,
    maskShift: { threshold: 8.5, depth: 6 },
  });
}

let rebuildObserver: MutationObserver | null = null;

// —— 捕获占字反馈（V0.020 ⑤ 定案）：图标换 clipboard-check + 文字换 + 50% 紫 + 禁点 1s ——

const CLIPBOARD_ICON =
  '<svg class="cap-icon" viewBox="0 0 384 512"><path d="M280 64h40c35.3 0 64 28.7 64 64V448c0 35.3-28.7 64-64 64H64c-35.3 0-64-28.7-64-64V128C0 92.7 28.7 64 64 64h40 9.6C121 27.5 153.3 0 192 0s71 27.5 78.4 64H280zM64 112c-8.8 0-16 7.2-16 16V448c0 8.8 7.2 16 16 16H320c8.8 0 16-7.2 16-16V128c0-8.8-7.2-16-16-16H304v24c0 13.3-10.7 24-24 24H192 104c-13.3 0-24-10.7-24-24V112H64zm128-8a24 24 0 1 0 0-48 24 24 0 1 1 0 48z"></path></svg>';
const CLIPBOARD_CHECK_ICON =
  '<svg class="cap-icon" viewBox="0 0 384 512"><path d="M192 0c-41.8 0-77.4 26.7-90.5 64H64C28.7 64 0 92.7 0 128V448c0 35.3 28.7 64 64 64H320c35.3 0 64-28.7 64-64V128c0-35.3-28.7-64-64-64H282.5C269.4 26.7 233.8 0 192 0zm0 64a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM305 273L177 401c-9.4 9.4-24.6 9.4-33.9 0L79 337c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l47 47L271 239c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9z"></path></svg>';

/** 捕获剪贴板（Rust 真链路 bubble_capture）：成功播占字反馈 */
async function capture(): Promise<void> {
  cancelClearConfirm();
  try {
    await invoke("bubble_capture");
    error.value = "";
    copied.value = true;
    window.clearTimeout(copiedTimer);
    copiedTimer = window.setTimeout(() => (copied.value = false), 1000);
    await refresh();
  } catch (err) {
    error.value = `捕获失败：${String(err)}`;
  }
}

// —— 一键清空二态（V0.020 ①③④ 定案）：宽度动画 + 悬停感知 2s 超时 + 旁路取消 ——

/** 宽度动画：width 无法从 auto 起过渡——量旧宽锁值回流再设新宽，330ms 后解锁 */
function animateClearWidth(from: number, to: number): void {
  const btn = clearBtn.value as HTMLElement | null;
  if (!btn) return;
  btn.style.width = `${from}px`;
  void (btn as HTMLElement).offsetWidth; // 强制回流：过渡从锁定的旧宽起算
  btn.style.width = `${to}px`;
  window.clearTimeout(clearWidthTimer);
  clearWidthTimer = window.setTimeout(() => (btn.style.width = ""), 330);
}

/** 复位（缩回路径共用：再点执行 / 2s 超时 / 旁路取消） */
function resetClearButton(): void {
  if (!confirmingClear.value) return;
  confirmingClear.value = false;
  window.clearTimeout(confirmTimer);
  const btn = clearBtn.value as HTMLElement | null;
  if (!btn) return;
  btn.classList.remove("confirming");
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    btn.textContent = "一键清空";
    return;
  }
  const from = (btn as HTMLElement).offsetWidth;
  btn.textContent = "一键清空";
  btn.style.width = "auto"; // 瞬时解锁量收起宽
  const to = (btn as HTMLElement).offsetWidth;
  animateClearWidth(from, to);
}

/** 确认态 2s 超时起算 */
function startClearTimeout(): void {
  window.clearTimeout(confirmTimer);
  confirmTimer = window.setTimeout(resetClearButton, 2000);
}

/** 悬停感知：确认态下鼠标在钮上暂停计时，离开再起算 */
function onClearEnter(): void {
  if (confirmingClear.value) window.clearTimeout(confirmTimer);
}

function onClearLeave(): void {
  if (confirmingClear.value) startClearTimeout();
}

/** 清空钮点击二态：首点展开确认（宽度过渡），再点执行集体退场 */
async function onClearClick(): Promise<void> {
  if (!confirmingClear.value) {
    confirmingClear.value = true;
    const btn = clearBtn.value as HTMLElement;
    btn.classList.add("confirming");
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      btn.textContent = `确认清空 ${items.value.length} 条？`;
    } else {
      const from = btn.offsetWidth;
      btn.textContent = `确认清空 ${items.value.length} 条？`;
      animateClearWidth(from, btn.offsetWidth);
    }
    if (!btn.matches(":hover")) startClearTimeout();
    return;
  }
  resetClearButton();
  try {
    await invoke("bubble_clear");
    await refresh();
  } catch (err) {
    console.error("清空失败", err);
  }
}

/** 旁路取消（确认清空期间点捕获或行）：退回原状 */
function cancelClearConfirm(): void {
  resetClearButton();
}

// —— 行交互（V0.028 对调语义）：单击开板（180ms 延迟）、双击复制（占字反馈） ——

async function onRowClick(item: BubbleItem): Promise<void> {
  if (isSuppressed()) return; // 拖拽落点抑制（useDragReorder 350ms 窗口）
  if (rowMaskDead(rowEl(item.id))) return; // 罩死行禁交互
  clearTimeout(clickTimer);
  clickTimer = window.setTimeout(() => emit("openBubble", item), 180);
}

async function onRowDblClick(item: BubbleItem): Promise<void> {
  clearTimeout(clickTimer); // 掐掉未决的开板定时器：双击只复制不开板
  if (rowMaskDead(rowEl(item.id))) return;
  try {
    await invoke("bubble_copy", { id: item.id });
    copied.value = true;
    window.clearTimeout(copiedTimer);
    copiedTimer = window.setTimeout(() => (copied.value = false), 1000);
  } catch (err) {
    console.error("复制失败", err);
  }
}

/** 删除单条（DelButton 二态确认） */
async function remove(item: BubbleItem): Promise<void> {
  try {
    await invoke("bubble_remove", { id: item.id });
    await refresh();
  } catch (err) {
    console.error("删除失败", err);
  }
}

/** 罩死判定的行元素定位（null 安全：行不在 DOM 即不罩死） */
function rowEl(id: number): HTMLElement {
  return (
    (document.querySelector(`#page-bubbles [data-row-id="${id}"]`) as HTMLElement | null) ??
    document.createElement("div")
  );
}

defineExpose({ refresh, cancelClearConfirm });

// 拖拽钩子安装（气泡路：cancelPendingClick 掐双击复制定时器；rerender 收场重拉）
const listKey = ref(0); // 强制重建计数：拖拽收场 vnode↔DOM 断链修复（同 TodoList）
installDragHooks({
  cancelPendingClick: () => clearTimeout(clickTimer),
  markSuppress: () => {},
  rerenderTodos: () => {}, // 清单路 rerender 由 TodoList 注册
  rerenderBubbles: () => void refresh(),
  forceRemount: () => {
    listKey.value += 1;
  },
});

onMounted(() => {
  void refresh();
  syncVeils();
  // v-else 切换重建 ul 补挂：观察 section.bubbles 子树（ul 换元素即重挂）；
  // refresh 后 nextTick 直接挂（items 到位 ul 已渲染——onMounted 时序比 TodoList 的
  // setup 同步段晚，此路径首挂即中）
  void nextTick(mountScrollKit);
  const host = document.querySelector(".bubbles");
  if (host) {
    rebuildObserver = new MutationObserver(() => {
      const ul = (listEl.value as unknown as { $el?: HTMLElement })?.$el ?? listEl.value;
      if (!ul) return;
      if (!glassBar) {
        mountScrollKit();
      } else if (ul.dataset.mounted !== "1") {
        glassBar = null;
        mountScrollKit();
      }
    });
    rebuildObserver.observe(host, { childList: true });
  }
});

onUnmounted(() => {
  [copiedTimer, confirmTimer, clearWidthTimer, clickTimer].forEach((t) => window.clearTimeout(t));
  rebuildObserver?.disconnect();
});
</script>

<template>
  <section class="bubbles">
    <div class="actions">
      <!-- 捕获钮：占字态换 clipboard-check 图标 + "已复制到剪贴板" + 50% 紫禁点 -->
      <button class="capture" :disabled="copied" @click="capture">
        <span v-if="!copied" class="cap-idle">
          <span class="cap-icon" v-html="CLIPBOARD_ICON"></span>捕获剪贴板
        </span>
        <span v-else class="cap-idle">
          <span class="cap-icon" v-html="CLIPBOARD_CHECK_ICON"></span>已复制到剪贴板
        </span>
      </button>
      <!-- 一键清空：红染玻璃卡；0 气泡灰染 disabled -->
      <button
        ref="clearBtn"
        class="clear"
        :class="{ confirming: confirmingClear }"
        :disabled="items.length === 0 && !confirmingClear"
        @click="onClearClick"
        @mouseenter="onClearEnter"
        @mouseleave="onClearLeave"
      >
        一键清空
      </button>
    </div>
    <p v-if="error" class="error">{{ error }}</p>
    <p v-if="items.length === 0" class="empty">暂无气泡，点上方捕获剪贴板</p>
    <TransitionGroup
      v-else
      ref="listEl"
      tag="ul"
      name="todo"
      id="bubble-list"
      class="group"
      :class="{ 'has-warning': hasWarning }"
      :key="listKey"
      :duration="320"
    >
      <!-- 满仓警告：滚动容器内部首项，随内容滚动（V0.017 ⑧ 定案） -->
      <li v-if="hasWarning" key="__warn" class="full-warning">
        气泡已经超过{{ maxBubbles }}个啦！都溢出来啦！(*ﾉωﾉ) EEK
      </li>
      <li
        v-for="item in items"
        :key="item.id"
        class="bubble-row"
        :data-row-id="item.id"
        @click="onRowClick(item)"
        @dblclick="onRowDblClick(item)"
      >
        <span class="b-text">{{ item.text }}</span>
        <DelButton :confirming="false" @press="remove(item)" @confirm="remove(item)" />
      </li>
    </TransitionGroup>
  </section>
</template>

<!-- 气泡样式已全局挂载（styles/bubbles.css 经 main.ts） -->
