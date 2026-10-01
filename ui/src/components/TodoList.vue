<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch, watchEffect } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { TodoItem, TodoView } from "../../types";
import DelButton from "./DelButton.vue";
import NeonCheckbox from "./NeonCheckbox.vue";
import { rowMaskDead, syncMaskDead } from "../composables/useMaskDead";
import { useEmptyState } from "../composables/useEmptyState";
import { useDelConfirmGroup } from "../composables/useDelConfirmGroup";
import { useScrollKit } from "../composables/useScrollKit";
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

const emit = defineEmits<{
  changed: [];
  openDetail: [item: TodoItem, anchor: { x: number; y: number }];
  /** 勾选入档成功（A7：详情板若正开着这条随行收起——design 勾选分支同款联动） */
  archived: [item: TodoItem];
}>();

const listEl = ref<HTMLElement | null>(null);

/** 未完成行（排序视图已由 Rust 裁决，此处不再排序只过滤） */
const active = computed(() => props.items.filter((it) => !it.done));

// —— DelButton 二态单实例（FIX005.24 收敛至 useDelConfirmGroup）：全列表同时
// 只允许一行处于确认态；再点执行 = 删除 + 变更上抛 ——

const { confirmingId, setDelRef, onPress, onConfirm, onCancel } = useDelConfirmGroup<TodoItem>(
  async (item) => {
    try {
      await invoke("todo_remove", { id: item.id });
      emit("changed");
    } catch (err) {
      console.error("删除失败", err);
    }
  },
);

// —— 勾选 / 点击语义 ——

/** 罩死判定的行元素定位（null 安全：行不在 DOM 即视为不罩死——塌缩离场中） */
function rowEl(id: number): HTMLElement {
  return (
    (document.querySelector(`#page-todos [data-row-id="${id}"]`) as HTMLElement | null) ??
    document.createElement("div")
  );
}

/** 勾选翻转（⑥ = design 勾选分支时序 1:1）。勾选视觉用 **DOM 直改**（input.checked
 * + 删除线类，= design 的 input.checked=true 立即置位）——不能走响应式乐观置位：
 * item.done=true 会瞬时触发 active computed 重算把行从列表剔除（比主拍快），勾选
 * 视觉没机会播 = "勾选瞬间坍缩"（真窗口实测 2026-09-28）。
 * 勾选视觉播 300ms 主拍，播完 emit changed（行离场）+ archived（详情联动） */
/** 逐行勾选主拍定时器（id → handle，FIX003.5）：单句柄版在 300ms 内连勾两行时
 * clearTimeout 掐死前行拍子——前行 archived 上抛丢失（A7 详情联动失效）、离场搭
 * 后行拍子。每行各自 300ms 到点互不取消；changed 由末拍收口一次（提前收口的
 * 全量重拉会把其他行在飞的主拍截断，"勾选瞬间坍缩"换入口复活） */
const toggleTimers = new Map<number, number>();
async function toggle(item: TodoItem): Promise<void> {
  if (rowMaskDead(rowEl(item.id))) return; // 罩死行勾选失效（V0.022 定案）
  const wasUndone = item.done === false; // 未完成 → 勾选入档方向
  try {
    await invoke("todo_toggle", { id: item.id });
    // 乐观视觉置位（DOM 直改，绕开响应式——见上注）
    const row = rowEl(item.id);
    const input = row.querySelector("input");
    if (input) input.checked = wasUndone;
    row.querySelector(".t-text")?.classList.toggle("is-done-text", wasUndone);
    // 同行再点重挂自家拍子（design row._moveTimer 同款）；跨行拍子互不取消
    clearTimeout(toggleTimers.get(item.id));
    toggleTimers.set(
      item.id,
      window.setTimeout(() => {
        toggleTimers.delete(item.id);
        if (wasUndone) emit("archived", item); // A7 联动随自家拍子逐行上抛
        if (toggleTimers.size === 0) emit("changed"); // 末拍收口
      }, 300), // 勾选动效主拍 300ms（用户定案）
    );
  } catch (err) {
    console.error("勾选失败", err);
  }
}

/** 退场钉高（⑤ = design collapseRow 第一步 1:1）：height 从 auto 收 0 不可过渡，
 * leave 前先把实测高度钉成内联起点，塌缩动画才有过渡区间——缺失即"行直接消失" */
function pinLeaveHeight(el: Element): void {
  (el as HTMLElement).style.height = `${(el as HTMLElement).offsetHeight}px`;
}

let clickTimer: number | undefined;
function onRowClick(item: TodoItem, e: MouseEvent): void {
  // 编辑态点击抑制（用户定案）：编辑中点任何位置都只是结束编辑，绝不开详情——
  // 点自己行 = 光标聚焦回输入框（换输入位置）；点其他行 = 提交后离开。
  // 判据含 pendingExit（已提交待切回）态：rename IPC 极快，blur 提交后主窗 refresh
  // 可在 click 事件前清掉 editingId（用户实测 bug 复发根因）
  if (editingId.value !== null || pendingExitId.value !== null) {
    const editingNow = editingId.value ?? pendingExitId.value;
    if (editingNow === item.id) {
      // FIX005.19：聚焦走 focusEditEnd 同款 DOM 直查——v-for 模板 ref 被 Vue
      // 收集为数组，editEl.value?.focus() 恒静默失效（本文件同款坑自记）
      focusEditEnd();
    } else if (editingId.value !== null) {
      // 仍有未提交编辑（pendingExit 态则 rename 已落库，无需二次提交）
      const editing = props.items.find((t) => t.id === editingId.value);
      if (editing) commitEdit(editing);
    }
    return;
  }
  // 勾选框坐标分流（A10 = design todos.js 行 click 分支 1:1）：点中勾选框范围 =
  // 勾选入档（不受拖拽落点抑制约束——design suppress 只拦开详情）；点正文 = 开详情
  const row = e.currentTarget as HTMLElement;
  const cb = row.querySelector(".neon-checkbox")?.getBoundingClientRect();
  const inBox =
    cb != null &&
    e.clientX >= cb.left &&
    e.clientX <= cb.right &&
    e.clientY >= cb.top &&
    e.clientY <= cb.bottom;
  if (inBox) {
    void toggle(item);
    return;
  }
  if (isSuppressed()) return; // 拖拽落点抑制（useDragReorder 350ms 窗口）
  if (rowMaskDead(rowEl(item.id))) return;
  clearTimeout(clickTimer);
  // 行中心视口坐标随行上抛（A3：详情板飞出原点 = 被点行中心）
  const anchor = (() => {
    const r = row.getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
  })();
  clickTimer = window.setTimeout(() => emit("openDetail", item, anchor), 180);
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
  startEditFocusFlow(); // 挂载聚焦 + 80ms 重申（光标行尾）
}

// 编辑框挂载即聚焦 + 光标行尾（PL018 光标不显示根治）：DOM 直查——v-for 内
// 模板 ref 被 Vue 收集为数组，`.focus()` 调用抛 TypeError 静默死亡（CDP 实测
// focus 从未被调用，"全选也从未出现过"同根因）；挂载后 80ms 重申兜底
//（CDP 实测挂载瞬间有一次性焦点重置回 BODY，重申在其后必然生效）
function focusEditEnd(): void {
  const inp = document.querySelector<HTMLInputElement>(".t-edit");
  if (!inp) return;
  inp.focus();
  const len = inp.value.length;
  inp.setSelectionRange(len, len);
}

function startEditFocusFlow(): void {
  void nextTick(focusEditEnd);
  window.setTimeout(focusEditEnd, 80);
}

/** 提交后待退出编辑的条目 id：editingId 保持到 refresh 数据到达（该条 text 变新值）
 * 才切回文本元素——消除"切回瞬间显示旧文本 → 刷新跳新文本"的闪帧（用户实测）；
 * 1.5s 超时兜底防 refresh 失败卡编辑态 */
const pendingExitId = ref<number | null>(null);

async function commitEdit(item: TodoItem): Promise<void> {
  const text = editDraft.value.trim();
  if (!text || text === item.text) {
    editingId.value = null;
    return;
  }
  try {
    await invoke("todo_rename", { id: item.id, text });
    emit("changed");
    pendingExitId.value = item.id; // 退出编辑延到数据到达（watch props.items 兜底超时）
    window.setTimeout(() => {
      if (pendingExitId.value === item.id) {
        pendingExitId.value = null;
        editingId.value = null;
      }
    }, 1500);
  } catch (err) {
    console.error("改名失败", err); // input 保留：失败可就地重试
  }
}

// 内容增删后罩死复核（滚动监听经 useGlassBar 钩子已通；此处补重渲染路径）。
// Vue 冻结防线（PL013 红线）：拖拽 engaged 期间外部 items 到达不重渲染——重挂 DOM
// 与虚拟 DOM 打架防线；落点收场 rerender 统一重拉
watchEffect(() => {
  syncMaskDead();
  if (deferDuringDrag()) return; // 冻结期跳过（拖拽收场 rerender 统一复核）
  void props.items.length; // 依赖收集：items 变化即复核
});

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
// 挂载时机（泄漏教训 2026-09-28）：此前用 rAF 链轮询等 .list 入 DOM——组件卸载后
// rAF 闭包仍存活，切页往返会给新 DOM 重复挂载滑杆/三角且无人清理（白板页黄三角
// 残留的根因）。改为 items 长度 watch + nextTick：挂载时机与数据到达对齐；
// composable 在异步上下文挂载时生命周期钩子失效（无组件实例），destroy 由本组件
// 持有，重挂/卸载时显式调用（FIX005.24 收敛至 useScrollKit）
// 整板阅读套件：滑杆 + 阅读三角挂卸显式管理
const scrollKit = useScrollKit(listEl, {
  boardRead: { skipDuringDrag: true },
  glassBar: { inset: true },
  mountedFlag: { key: "mounted", hostId: "todo-active" },
});
const mountScrollKit = scrollKit.mount;
const unmountScrollKit = scrollKit.unmount;

// items 到达/清空 → 渲染完成后挂载（覆盖首挂与重建两种时机）
watch(
  () => props.items.length,
  () => {
    void nextTick(mountScrollKit);
  },
);

// 行内编辑退出时机（用户实测闪帧修复）：refresh 数据到达且该条文本已更新为新值
// 时才退出编辑态——文本元素直接以新文本首现，无"旧文本→新文本"中间帧
watch(
  () => props.items,
  (list) => {
    if (pendingExitId.value === null) return;
    const it = list.find((t) => t.id === pendingExitId.value);
    if (it && it.text === editDraft.value.trim()) {
      pendingExitId.value = null;
      editingId.value = null;
    }
  },
  { deep: true },
);

// —— 空态显隐（FIX005.24 收敛至 useEmptyState，与归档/气泡同款共存渲染）——
const {
  showEmpty,
  onAfterLeave,
  dispose: disposeEmptyState,
} = useEmptyState(() => props.items.length);

// v-else 切换重建 ul 兜底（items 长度不变但 ul 换元素的场景）：观测 section.list
// 子树替换即重挂
let rebuildObserver: MutationObserver | null = null;

onMounted(() => {
  rebuildObserver = new MutationObserver(() => {
    const ul =
      (listEl.value as unknown as { $el?: HTMLElement })?.$el ??
      (listEl.value as HTMLElement | null);
    if (!ul) return;
    if (ul.dataset.mounted !== "1") {
      unmountScrollKit();
      mountScrollKit();
    }
  });
  rebuildObserver.observe(document.querySelector(".list") ?? document.body, {
    childList: true,
  });
  void nextTick(mountScrollKit);
});

onUnmounted(() => {
  clearTimeout(clickTimer);
  toggleTimers.forEach((handle) => clearTimeout(handle));
  toggleTimers.clear();
  rebuildObserver?.disconnect();
  unmountScrollKit();
  disposeEmptyState();
});
</script>

<template>
  <section class="list">
    <p v-if="showEmpty" class="empty">暂无待办，添加一条吧</p>
    <TransitionGroup
      ref="listEl"
      tag="ul"
      name="todo"
      id="todo-active"
      class="group"
      :key="listKey"
      :duration="320"
      @before-leave="pinLeaveHeight"
      @after-leave="onAfterLeave"
    >
      <li v-for="item in active" :key="item.id" class="todo-item">
        <div
          class="todo-row"
          :data-row-id="item.id"
          @click="onRowClick(item, $event)"
          @dblclick="onRowDblClick(item)"
        >
          <NeonCheckbox :checked="item.done" />
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
            @leave="onCancel(item)"
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
