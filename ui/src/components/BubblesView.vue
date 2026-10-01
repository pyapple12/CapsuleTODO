<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { BubbleCaptureOutcome, BubbleItem, BubbleSnapshot } from "../../types";
import DelButton from "./DelButton.vue";
import { rowMaskDead } from "../composables/useMaskDead";
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
const copied = ref(false); // 占字态：捕获钮禁点 + 换 clipboard-check 图标文案
const duplicate = ref(false); // 重复占字态（PL015.5 去重）：换 ✕ 图标 + "重复捕获，无效！"
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
let unlistenBubbleChanged: UnlistenFn | undefined; // 热键失焦刷新监听句柄（PL016.1）

const clearBtn = ref<HTMLElement | null>(null);
const listEl = ref<HTMLElement | null>(null);

/** 满 maxBubbles 警告显隐：超过阈值（非达到）才警告——design bubbles.js 同款语义；
 * 阈值由设置板步进（已持久化，PL014.2 落库）即时生效。满额裁决收敛前端本地
 * （FIX004.23：Rust 侧 snapshot.remind 死值已删，契约只剩 items） */
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
    error.value = "";
    duplicate.value = outcome.status === "duplicate";
    copied.value = !duplicate.value;
    window.clearTimeout(copiedTimer);
    copiedTimer = window.setTimeout(() => {
      copied.value = false;
      duplicate.value = false;
    }, 1000);
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
        console.error("清空失败", err);
      }
    }, 300);
    return;
  }
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

// —— DelButton 二态单实例（对齐 design V0.026 三处二态推广：气泡删除也是两拍确认，
// 此前 :confirming 恒 false 属移植遗漏）——

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

async function onRowClick(item: BubbleItem, e: MouseEvent): Promise<void> {
  if (isSuppressed()) return; // 拖拽落点抑制（useDragReorder 350ms 窗口）
  if (rowMaskDead(rowEl(item.id))) return; // 罩死行禁交互
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
  clickTimer = window.setTimeout(() => emit("openBubble", item, anchor), 180);
}

async function onRowDblClick(item: BubbleItem): Promise<void> {
  if (isSuppressed()) return; // 拖拽收场落点双击不当作复制（A6，detail.js:145 同款）
  clearTimeout(clickTimer); // 掐掉未决的开板定时器：双击只复制不开板
  if (rowMaskDead(rowEl(item.id))) return;
  try {
    await invoke("bubble_copy", { id: item.id });
    copied.value = true;
    duplicate.value = false; // FIX004.3：复制占字复位须同清 duplicate，否则重复捕获后双击复制=钮永久卡死
    window.clearTimeout(copiedTimer);
    copiedTimer = window.setTimeout(() => {
      copied.value = false;
      duplicate.value = false;
    }, 1000);
  } catch (err) {
    console.error("复制失败", err);
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

/** 退场钉高（⑤ = design collapseRow 第一步，与清单同款）：单删/清空集体退场共用。
 * 必须同时锁 border-box——气泡行自带 9px 上下内距且默认 content-box，把 offsetHeight
 * 写进 content 高会瞬间膨胀 18px（单行"变两行再坍缩"，真窗口实测 2026-09-28） */
function pinLeaveHeight(el: Element): void {
  const h = el as HTMLElement;
  h.style.boxSizing = "border-box";
  h.style.height = `${h.offsetHeight}px`;
}

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
  // FIX005.26 竞态防护：listen 注册异步返回——快速切页时卸载清理先于句柄到手，
  // 已卸载标志命中即当场注销（否则监听泄漏且死组件持续收事件）
  let disposed = false;
  listen("bubble-changed", () => {
    void refresh();
  })
    .then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlistenBubbleChanged = unlisten;
      }
    })
    .catch((err) => console.error("bubble-changed 监听注册失败", err));
  onUnmounted(() => {
    disposed = true;
  });
});

onUnmounted(() => {
  // clearFxTimer（清空集体退场收尾）豁免清理：用户已点"确认清空"，300ms 收尾
  // 落库必须完成——切页取消会让"确认"被吞（数据残留 + 退场动画已播 = 状态诡异）。
  // 收尾回调仅 invoke + refresh，无报错路径（IAB 实测 errs=0），卸载后执行静默无害
  [copiedTimer, confirmTimer, clearWidthTimer, clickTimer].forEach((t) => window.clearTimeout(t));
  unlistenBubbleChanged?.();
  rebuildObserver?.disconnect();
  unmountScrollKit();
  disposeEmptyState();
});
</script>

<template>
  <section class="bubbles">
    <div class="actions">
      <!-- 捕获钮：占字态换 clipboard-check 图标 + "已捕获" + 50% 紫禁点（文案定案 2026-09-28）；
           重复占字态换 ✕ 图标 + "重复捕获，无效！"（PL015.5 去重定案 2026-09-30） -->
      <button class="capture" :disabled="copied || duplicate" @click="capture">
        <span v-if="duplicate" class="cap-idle">
          <span class="cap-icon" v-html="CLIPBOARD_X_ICON"></span>重复捕获，无效！
        </span>
        <span v-else-if="copied" class="cap-idle">
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
    <p v-if="error" class="error">{{ error }}</p>
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
