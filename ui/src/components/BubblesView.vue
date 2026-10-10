<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { BubbleCaptureOutcome, BubbleItem, BubbleSnapshot } from "../../types";
import DelButton from "./DelButton.vue";
import { rowMaskDead } from "../composables/useMaskDead";
import { rowElById, pinLeaveHeight } from "../composables/useListRow";
import { syncVeils } from "../composables/useVeils";
import { useEmptyState } from "../composables/useEmptyState";
import { useDelConfirmGroup } from "../composables/useDelConfirmGroup";
import { useScrollKit } from "../composables/useScrollKit";
import { installDragHooks, isSuppressed } from "../composables/useDragReorder";

// ===== 气泡页（PL012.2 换装实验场形态 V0.017–V0.020 + V0.028 对调语义）：
// 捕获钮（clipboard 双图标 + 占字 1s 反馈）/ 一键清空二态（宽度动画 + 悬停感知 2s
// 超时 + 旁路取消）/ 满仓警告红字（has-warning 两档偏移 + 隐区位移在 useBoardRead）/
// 行（两行截断 + DelButton 二态 + 单击开板双击复制 180ms 消歧）。数值全沿实测定案 =====

const emit = defineEmits<{
  openBubble: [item: BubbleItem, anchor: { x: number; y: number }];
  changed: [];
}>();

const props = defineProps<{
  /** 满仓警告阈值（设置板步进同源，父级持有；超过才警告——design 定案语义） */
  maxBubbles: number;
}>();

const items = ref<BubbleItem[]>([]);
const error = ref("");
/** 占字态（PL025.6 单态 + 判别）："" 空闲 / "capture" 已捕获 / "copy" 已复制 / "duplicate" 重复捕获 */
const feedback = ref<"" | "capture" | "copy" | "duplicate">("");
const confirmingClear = ref(false);

// 空态显隐（删末条动画定案 2026-09-30）：TransitionGroup 恒挂载（不再与空态
// v-if/v-else 互斥——删末条走分支整体卸载时 leave 无机会播 = 瞬间消失，与归档板
// 同根；清空集体退场 2026-09-28 的两段式 workaround 即此坑的批量版），空态文案
// 延至末条 leave 播完（after-leave）出现；清空走 DOM 直改无 leave，定时器兜底置位
//（FIX005.24 收敛至 useEmptyState）
const {
  showEmpty,
  onAfterLeave,
  dispose: disposeEmptyState,
} = useEmptyState(() => items.value.length);
let copiedTimer = 0;
let confirmTimer = 0;
let clearWidthTimer = 0;
let clickTimer: number | undefined;
let errorTimer = 0; // 错误行 1s 自动隐藏定时器（PL024.8a）
let unlistenBubbleChanged: UnlistenFn | undefined; // 热键失焦刷新监听句柄（PL016.1）
// 监听竞态防护标志（FIX006.4）：setup 层声明——生命周期钩子仅在 setup 同步上下文
// 注册才生效（嵌套注册被 Vue 忽略 = 防护恒失效教训）
let bubbleListenDisposed = false;

const clearBtn = ref<HTMLElement | null>(null);
const listEl = ref<HTMLElement | null>(null);

/** 满 maxBubbles 警告显隐：超过阈值（非达到）才警告——design bubbles.js 同款语义；
 * 阈值由设置板步进（已持久化，PL014.2 落库）即时生效。满额裁决收敛前端本地
 * （FIX004.23：Rust 侧 snapshot.remind 死值已删，契约只剩 items） */
const hasWarning = computed(() => items.value.length > props.maxBubbles);

/**
 * 错误文案截断（PL024.8a 单点收敛）：第二个全角冒号起丢弃——系统英文细节不进 UI，
 * Rust 文案与 console 底账保持完整（FIX009.6 可见文案/底账分离先例）
 */
function truncateError(msg: string): string {
  const first = msg.indexOf("：");
  if (first < 0) return msg;
  const second = msg.indexOf("：", first + 1);
  return second < 0 ? msg : msg.slice(0, second);
}

/** 展示错误行（PL024.8a）：置入文案 + 挂 1s 自动隐藏定时器（重复触发重新计时） */
function showError(msg: string): void {
  error.value = truncateError(msg);
  window.clearTimeout(errorTimer);
  errorTimer = window.setTimeout(() => {
    error.value = "";
  }, 1000);
}

/** 立即清除错误行（成功路径）：取消未决定时器并清文本（渐隐由 Transition 承担） */
function clearError(): void {
  window.clearTimeout(errorTimer);
  error.value = "";
}

/**
 * 拉取气泡快照（changed 上抛：父级同步页签徽章——捕获/删除/清空都走这里）。
 * FIX009.5：notify=false 供 bubble-changed 事件触发路径——App 自身的同款监听
 * 已管徽章，此处再 emit 会让 App.refreshBadge 对同一事件跑两次（冗余 IPC）；
 * 页内用户操作路径照旧 emit（App 无其他途径感知页内操作）
 */
async function refresh(notify: boolean = true): Promise<void> {
  try {
    const snapshot = await invoke<BubbleSnapshot>("bubble_list");
    items.value = snapshot.items;
    if (notify) emit("changed");
  } catch (err) {
    console.error("bubble_list 拉取失败", err);
  }
}

// 主窗挂点（design glass-bar.js 同款）：滑杆 inset + 整板阅读（skipDuringDrag + maskShift）。
// composable 在异步上下文挂载时生命周期钩子失效，destroy 由本组件持有，重挂/卸载
// 时显式调用（泄漏教训同 TodoList 2026-09-28；FIX005.24 收敛至 useScrollKit）
const scrollKit = useScrollKit(listEl, {
  boardRead: {
    rowSel: ".bubble-row",
    skipDuringDrag: true,
    maskShift: { threshold: 8.5, depth: 6 },
  },
  glassBar: { inset: true },
  mountedFlag: { key: "mounted", hostId: "bubble-list" },
});
const mountScrollKit = scrollKit.mount;
const unmountScrollKit = scrollKit.unmount;

let rebuildObserver: MutationObserver | null = null;

// —— 捕获占字反馈（V0.020 ⑤ 定案）：图标换 clipboard-check + 文字换 + 50% 紫 + 禁点 1s ——

const CLIPBOARD_ICON =
  '<svg class="cap-icon" viewBox="0 0 384 512"><path d="M280 64h40c35.3 0 64 28.7 64 64V448c0 35.3-28.7 64-64 64H64c-35.3 0-64-28.7-64-64V128C0 92.7 28.7 64 64 64h40 9.6C121 27.5 153.3 0 192 0s71 27.5 78.4 64H280zM64 112c-8.8 0-16 7.2-16 16V448c0 8.8 7.2 16 16 16H320c8.8 0 16-7.2 16-16V128c0-8.8-7.2-16-16-16H304v24c0 13.3-10.7 24-24 24H192 104c-13.3 0-24-10.7-24-24V112H64zm128-8a24 24 0 1 0 0-48 24 24 0 1 1 0 48z"></path></svg>';
const CLIPBOARD_CHECK_ICON =
  '<svg class="cap-icon" viewBox="0 0 384 512"><path d="M192 0c-41.8 0-77.4 26.7-90.5 64H64C28.7 64 0 92.7 0 128V448c0 35.3 28.7 64 64 64H320c35.3 0 64-28.7 64-64V128c0-35.3-28.7-64-64-64H282.5C269.4 26.7 233.8 0 192 0zm0 64a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM305 273L177 401c-9.4 9.4-24.6 9.4-33.9 0L79 337c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l47 47L271 239c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9z"></path></svg>';
const CLIPBOARD_X_ICON =
  '<svg class="cap-icon" viewBox="0 0 384 512"><path d="M192 0c-41.8 0-77.4 26.7-90.5 64H64C28.7 64 0 92.7 0 128V448c0 35.3 28.7 64 64 64H320c35.3 0 64-28.7 64-64V128c0-35.3-28.7-64-64-64H282.5C269.4 26.7 233.8 0 192 0zm0 64a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM143 239c9.4-9.4 24.6-9.4 33.9 0l15 15 15-15c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9l-15 15 15 15c9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0l-15-15-15 15c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9l15-15-15-15c-9.4-9.4-9.4-24.6 0-33.9z"></path></svg>';

/** 捕获剪贴板（Rust 真链路 bubble_capture）：成功播占字反馈；重复内容切
 * "重复捕获，无效！"占字态（PL015.5 去重，手动与热键两入口同规） */
async function capture(): Promise<void> {
  cancelClearConfirm();
  try {
    const outcome = await invoke<BubbleCaptureOutcome>("bubble_capture");
    clearError();
    feedback.value = outcome.status === "duplicate" ? "duplicate" : "capture";
    window.clearTimeout(copiedTimer);
    copiedTimer = window.setTimeout(() => {
      feedback.value = "";
    }, 1000);
    await refresh();
  } catch (err) {
    showError(`捕获失败：${String(err)}`);
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
  // 集体退场（⑥ design bubbles.js 同款两段式）：先 DOM 直改钉高+挂塌缩类播集体
  // 退场，300ms 收尾后才落库刷新——立即清数据会走 v-if/v-else 整体卸载（空态切换），
  // 逐行退场动画没有机会触发 = "直接消失"（真窗口实测 2026-09-28）
  const rows = [...document.querySelectorAll<HTMLElement>("#bubble-list .bubble-row")];
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (rows.length && !reduce) {
    rows.forEach((li) => {
      li.style.boxSizing = "border-box";
      li.style.height = `${li.offsetHeight}px`;
      li.classList.add("todo-leave-active"); // 复用清单塌缩三通道（height/transform/opacity）
      void li.offsetHeight; // 强制回流：锁定起步高度
      li.style.height = "0";
    });
    // 收尾句柄故意不存（不可被卸载清理）：见 onUnmounted 注记——用户已确认的清空必须完成
    window.setTimeout(async () => {
      try {
        await invoke("bubble_clear");
        await refresh();
      } catch (err) {
        // FIX007.7 失败可见（行已塌缩"假空"，不提示则用户以为删成功）
        showError(`清空失败：${String(err)}`);
        console.error("清空失败", err);
      }
    }, 300);
    return;
  }
  try {
    await invoke("bubble_clear");
    await refresh();
  } catch (err) {
    error.value = `清空失败：${String(err)}`;
    console.error("清空失败", err);
  }
}

/** 旁路取消（确认清空期间点捕获或行）：退回原状 */
function cancelClearConfirm(): void {
  resetClearButton();
}

// —— 行交互（V0.028 对调语义）：单击开板（180ms 延迟）、双击复制（占字反馈） ——

// —— DelButton 二态单实例（FIX005.24 收敛至 useDelConfirmGroup；对调语义 V0.028：
// 气泡删除也是两拍确认）——

const { confirmingId, setDelRef, onPress, onConfirm, onCancel } = useDelConfirmGroup<BubbleItem>(
  async (item) => {
    try {
      await invoke("bubble_remove", { id: item.id });
      await refresh();
    } catch (err) {
      console.error("删除失败", err);
    }
  },
);

// FIX011.9：删冗余 async（函数体无 await，开板经拍子回调 emit）
function onRowClick(item: BubbleItem, e: MouseEvent): void {
  if (isSuppressed()) return; // 拖拽落点抑制（useDragReorder 350ms 窗口）
  if (rowMaskDead(rowEl(item.id))) return; // 罩死行禁交互
  // FIX009.4：离场塌缩窗口点击防幽灵板（气泡全文板 readonly 无写入防御，死
  // 条目快照会被原样展示）——条目已删则静默忽略
  if (!items.value.some((it) => it.id === item.id)) return;
  cancelClearConfirm(); // 确认清空期间点气泡行：一键清空旁路退回（A5，todos.js:307 同款）
  clearTimeout(clickTimer);
  // 行中心视口坐标随行上抛（A3：全文板飞出原点 = 被点行中心）
  const row = (e.currentTarget as HTMLElement).closest(".bubble-row") as HTMLElement | null;
  const anchor = row
    ? (() => {
        const r = row.getBoundingClientRect();
        return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
      })()
    : { x: 0, y: 0 };
  clickTimer = window.setTimeout(() => {
    // FIX010.1：拍子到点复核（TodoList 同款）——武装期间气泡可被清空/删除
    if (!items.value.some((it) => it.id === item.id)) return;
    emit("openBubble", item, anchor);
  }, 180);
}

async function onRowDblClick(item: BubbleItem): Promise<void> {
  // FIX010.1：掐已武装拍子前置（同 TodoList）
  clearTimeout(clickTimer);
  // FIX009.4：离场塌缩窗口双击防幽灵复制——已删 id 的 bubble_copy 必然 NotFound
  if (!items.value.some((it) => it.id === item.id)) return;
  if (isSuppressed()) return; // 拖拽收场落点双击不当作复制（A6，detail.js:145 同款）
  if (rowMaskDead(rowEl(item.id))) return;
  try {
    await invoke("bubble_copy", { id: item.id });
    feedback.value = "copy"; // PL025.6：双击复制回显「已复制」（单态覆盖，无需同清）
    window.clearTimeout(copiedTimer);
    copiedTimer = window.setTimeout(() => {
      feedback.value = "";
    }, 1000);
  } catch (err) {
    console.error("复制失败", err);
  }
}

// FIX007.9 收敛：rowEl 定位与退场钉高走 useListRow 共享件（boxSizing 钉 border-box
// 已内聚——气泡行 9px 内距 content-box 膨胀 18px 教训 2026-09-28 一并带走）
function rowEl(id: number): HTMLElement {
  return rowElById("#page-bubbles", id);
}

// FIX010.13：原 defineExpose({ refresh, cancelClearConfirm }) 已删——App 不持本
// 组件模板 ref（数据自拉 + changed 事件上抛），零外部调用方

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

// 收口总线注册（A2）已随 useDelConfirmGroup 内聚（FIX005.24），本组件无显式注册

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
      if (ul.dataset.mounted !== "1") {
        unmountScrollKit();
        mountScrollKit();
      }
    });
    rebuildObserver.observe(host, { childList: true });
  }
  // 热键失焦刷新（PL016.1）：本组件 v-if 挂载 = 仅气泡页激活时存在——热键在
  // 别处入库时此监听把新泡拉进当前视图（无监听则要切页才见）。
  // FIX006.4 竞态防护：listen 注册异步返回——快速切页时卸载清理先于句柄到手，
  // 已卸载标志命中即当场注销（否则监听泄漏且死组件持续收事件）。
  // disposed 声明在 setup 层、置位在顶层 onUnmounted（FIX005.26 曾嵌套注册在
  // onMounted 回调内 = 生命周期钩子失效区，防护恒不生效，A006 P2-4 实证）
  listen("bubble-changed", () => {
    void refresh(false); // FIX009.5：事件路径不 emit——App 监听已同步徽章
  })
    .then((unlisten) => {
      if (bubbleListenDisposed) {
        unlisten();
      } else {
        unlistenBubbleChanged = unlisten;
      }
    })
    .catch((err) => console.error("bubble-changed 监听注册失败", err));
});

onUnmounted(() => {
  bubbleListenDisposed = true; // FIX006.4：后到的监听句柄当场注销（防泄漏）
  // clearFxTimer（清空集体退场收尾）豁免清理：用户已点"确认清空"，300ms 收尾
  // 落库必须完成——切页取消会让"确认"被吞（数据残留 + 退场动画已播 = 状态诡异）。
  // 收尾回调仅 invoke + refresh，无报错路径（IAB 实测 errs=0），卸载后执行静默无害
  [copiedTimer, confirmTimer, clearWidthTimer, clickTimer, errorTimer].forEach((t) =>
    window.clearTimeout(t),
  );
  unlistenBubbleChanged?.();
  rebuildObserver?.disconnect();
  unmountScrollKit();
  disposeEmptyState();
});
</script>

<template>
  <section class="bubbles">
    <div class="actions">
      <!-- 捕获钮：占字态单态 + 判别（PL025.6）——已捕获 / 已复制 / 重复捕获，无效！，
           均换图标 + 50% 紫禁点（文案定案 2026-09-28；复制判别 2026-10-10） -->
      <button class="capture" :disabled="feedback !== ''" @click="capture">
        <span v-if="feedback === 'duplicate'" class="cap-idle">
          <span class="cap-icon" v-html="CLIPBOARD_X_ICON"></span>重复捕获，无效！
        </span>
        <span v-else-if="feedback === 'copy'" class="cap-idle">
          <span class="cap-icon" v-html="CLIPBOARD_CHECK_ICON"></span>已复制
        </span>
        <span v-else-if="feedback === 'capture'" class="cap-idle">
          <span class="cap-icon" v-html="CLIPBOARD_CHECK_ICON"></span>已捕获
        </span>
        <span v-else class="cap-idle">
          <span class="cap-icon" v-html="CLIPBOARD_ICON"></span>捕获剪贴板
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
    <Transition name="err">
      <p v-if="error" class="error">{{ error }}</p>
    </Transition>
    <p v-if="showEmpty" class="empty">暂无气泡，点上方捕获剪贴板</p>
    <TransitionGroup
      ref="listEl"
      tag="ul"
      name="todo"
      id="bubble-list"
      class="group board-read glass-scroll"
      :class="{ 'has-warning': hasWarning }"
      :key="listKey"
      :duration="320"
      @before-leave="pinLeaveHeight"
      @after-leave="onAfterLeave"
    >
      <!-- board-read/glass-scroll 写进静态 class（FIX 目验③）：has-warning 动态切换
           会触发 Vue class patch 以 vdom 重写 class 属性，抹掉套件运行时 add 的类
           （原生滚动条回归 + 溶解遮罩消失）——静态+动态合并后 patch 两者恒在。
           挂载前已有类无碍：mountScrollKit 同帧 nextTick 执行，且隐藏原生滚动条
           本就是套件职责 -->
      <!-- 满仓警告：滚动容器内部首项，随内容滚动（V0.017 ⑧ 定案） -->
      <li v-if="hasWarning" key="__warn" class="full-warning">
        气泡已经超过{{ maxBubbles }}个啦！都溢出来啦！(*ﾉωﾉ) EEK
      </li>
      <li
        v-for="item in items"
        :key="item.id"
        class="bubble-row"
        :data-row-id="item.id"
        @click="onRowClick(item, $event)"
        @dblclick="onRowDblClick(item)"
      >
        <span class="b-text">{{ item.text }}</span>
        <DelButton
          :ref="(el) => setDelRef(item.id, el as InstanceType<typeof DelButton>)"
          :confirming="confirmingId === item.id"
          @press="onPress(item)"
          @confirm="onConfirm(item)"
          @leave="onCancel(item)"
        />
      </li>
    </TransitionGroup>
  </section>
</template>

<!-- 气泡样式已全局挂载（styles/bubbles.css 经 main.ts） -->
