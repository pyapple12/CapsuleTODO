<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { TodoItem } from "../../types";
import DelButton from "./DelButton.vue";
import NeonCheckbox from "./NeonCheckbox.vue";
import { useBoardRead } from "../composables/useBoardRead";
import { useGlassBar } from "../composables/useGlassBar";
import { bindOverlayState, syncVeils } from "../composables/useVeils";
import { registerDelConfirms, rollbackAllDelConfirms } from "../composables/delConfirmBus";

// ===== 归档板（PL011.2）：已完成条目管理（勾选退回 / 删除二态）。
// 板揭示 = 动态落位（板顶=页签顶-4）+ origin 注入 scale 回弹（从归档图标飞出）；
// 三角 hintHost = 板内玻璃（V0.028 方案 B：随板缩放生长）；滑杆锚板内（right 4.5）。
// 职责切分（2026-09-27 修正，与 design 对齐）：按钮出入场（archive-fx）只在页签切换时
// 编排、由父级经 setArchiveVisible 驱动——此前误绑在开合上，收板触发按钮塌缩隐藏、
// 复开落空（第三次点击无按钮可点，用户实测）；开合只管 .open 类与动态落位。
// 行为红线（V0.015 定案）：归档行点正文 no-op；归档态不显示 note；板内浮现动画不做 =====

const props = defineProps<{
  /** 归档视图（父级经 todo_archive_list 拉取） */
  items: TodoItem[];
}>();

const emit = defineEmits<{ changed: []; opened: [] }>();

const overlay = ref<HTMLElement | null>(null);
const listEl = ref<HTMLElement | null>(null);
const isOpen = ref(false);
const btnLeaving = ref(false);
const btnEntering = ref(false);
const btnHidden = ref(false);

// —— 板内滚动套件（整板阅读 + 玻璃滑杆）：开板 listKey++ 重建 UL（重播勾选动画），
// 套件必须拆旧挂新随行——旧的一次性挂载守卫（!boardRead）让第二开板起绑在已换下
// 文档的旧 UL 上，三角/滑杆/整板阅读全失联（FIX003.3，TodoList 同款 destroy 链）
let boardRead: { sync: () => void; settle: () => void; destroy: () => void } | null = null;
let glassBar: { sync: () => void; destroy: () => void } | null = null;

/** 拆旧套件（监听/observer/三角/浮钮全清） */
function unmountScrollKit(): void {
  boardRead?.destroy();
  glassBar?.destroy();
  boardRead = null;
  glassBar = null;
}

/** 挂套件到当前 UL（listEl ref 指向 TransitionGroup 组件实例——真实 UL 须经 $el 取） */
function mountScrollKit(): void {
  const listDom = (listEl.value as unknown as { $el?: HTMLElement })?.$el ?? listEl.value;
  if (!listDom || boardRead) return;
  boardRead = useBoardRead(listDom, {
    hintHost: overlay.value?.querySelector(".board-glass") as HTMLElement,
    layout: true,
  });
  glassBar = useGlassBar(listDom, {
    anchor: overlay.value?.querySelector(".board-glass") as HTMLElement,
    inset: true,
    right: 4.5, // 几何缝心 6 左移 0.5（用户定案：微避描边亮线）
  });
}

// 帘联动注册：归档滑杆 veil = !boardOpen（板开则显、关板则隐——design 特例）
bindOverlayState("board", () => isOpen.value);

// —— 归档按钮出入场编排（archive-fx.js 1:1，定时器替代 animationend）——
// 仅页签切换时由父级调用：离清单页塌缩+粒子迸裂，回清单页粒子汇聚+回弹弹出

let animTimer: number | undefined;
const particleHost = () => document.getElementById("board") as HTMLElement | null;

/** 粒子迸裂/汇聚：18~28 颗随机 accent 圆点，从按钮中心定向飞行（播完自摘） */
function spawnParticles(mode: "in" | "out"): void {
  const host = particleHost();
  if (!host) return;
  const count = 18 + Math.floor(Math.random() * 11);
  for (let i = 0; i < count; i += 1) {
    const p = document.createElement("span");
    p.className = `archive-particle${mode === "in" ? " in" : ""}`;
    const angle = (Math.PI * 2 * i) / count + (Math.random() - 0.5) * 0.6;
    const dist = 10 + Math.random() * 8;
    p.style.left = "25px";
    p.style.top = "25px";
    p.style.setProperty("--dx", `${Math.cos(angle) * dist}px`);
    p.style.setProperty("--dy", `${Math.sin(angle) * dist}px`);
    p.addEventListener("animationend", () => p.remove());
    host.appendChild(p);
  }
}

/** 按钮显隐编排：离场 = 吸气塌缩 320ms → 隐藏 + 迸裂；入场 = 汇聚 240ms → 回弹弹出 */
function setArchiveVisible(visible: boolean): void {
  if (btnHidden.value === !visible) return; // 幂等守卫：防互切误喷粒子
  window.clearTimeout(animTimer);
  btnLeaving.value = false;
  btnEntering.value = false;
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    btnHidden.value = !visible;
    return;
  }
  if (visible) {
    spawnParticles("in");
    animTimer = window.setTimeout(() => {
      btnHidden.value = false;
      btnEntering.value = true; // 回弹弹出（entering 类）
    }, 240);
  } else {
    btnLeaving.value = true; // 吸气塌缩（leaving 类）
    animTimer = window.setTimeout(() => {
      btnHidden.value = true;
      btnLeaving.value = false;
      spawnParticles("out");
    }, 320);
  }
}

/** 开合归档板：开板重拉数据 + 动态落位 + 揭示动画 + 450ms 后 settle（揭示中几何中间态防御）。
 * 开合都先收口未决删除确认（A2 = design setBoard 首行：板开遮盖清单行、收板后板内行复见）。
 * 开板 listKey++ 强制列表重建（design renderBoard 每次开板 innerHTML 重建 → 勾选态的
 * 粒子迸发/环脉冲/对勾描画在新元素上重播——Vue 持久 vnode 不重建则开屏无动画，
 * 真窗口实测 2026-09-28） */
async function toggle(): Promise<void> {
  rollbackAllDelConfirms();
  const next = !isOpen.value;
  if (next) {
    emit("opened"); // 三板互斥：父级收其余两板
    listKey.value += 1;
    isOpen.value = true;
    syncVeils();
    await nextTick();
    // 动态落位（panels.js 同款）：板顶 = 页签顶再上扩 4px；揭示原点注入图标中心
    // 的板内坐标（板左缘 14px，图标中心卡内 x=20 → 板内 x=6，y=20-top）
    const board = document.getElementById("board");
    const tabsEl = document.querySelector(".tabs");
    if (board && tabsEl && overlay.value) {
      const br = board.getBoundingClientRect();
      const tr = tabsEl.getBoundingClientRect();
      const top = Math.max(0, tr.top - br.top - 4);
      overlay.value.style.top = `${top}px`;
      overlay.value.style.setProperty("--origin-x", "6px");
      overlay.value.style.setProperty("--origin-y", `${20 - top}px`);
    }
    // 开板重建 UL 后拆旧挂新：三角/滑杆/整板阅读绑定当次列表（FIX003.3）
    unmountScrollKit();
    mountScrollKit();
    // 揭示动画落定后重算滑杆/三角/收尾带（transform 中间态不取几何——V0.015 实测）
    window.setTimeout(() => {
      boardRead?.settle();
      glassBar?.sync();
    }, 450);
  } else {
    isOpen.value = false;
    syncVeils();
  }
}

/** 收板（互斥出口：详情/设置板开启时由父级调用）；已关早退 */
function close(): void {
  if (!isOpen.value) return;
  rollbackAllDelConfirms();
  isOpen.value = false;
  syncVeils();
}

defineExpose({ toggle, close, setArchiveVisible });

/** 点板空白收板（与再点按钮双出口；行内交互 stop 传播不误触） */
function onOverlayDown(e: MouseEvent): void {
  if ((e.target as HTMLElement).closest(".board-glass") === null) toggle();
}

// 点板外任何位置收板（panels.js 同款 document 级）；目标已脱离文档（重渲染摘除
// 原节点）不视为板外点击，防误收
function onDocClick(e: MouseEvent): void {
  if (!isOpen.value) return;
  const t = e.target as HTMLElement;
  if (!t.isConnected) return;
  if (t.closest(".archive-overlay") || t.closest(".deleteButton")) return;
  close();
}

onMounted(() => {
  document.addEventListener("click", onDocClick);
});

onUnmounted(() => {
  document.removeEventListener("click", onDocClick);
  window.clearTimeout(animTimer);
  unmountScrollKit();
  restoreTimers.forEach((handle) => window.clearTimeout(handle));
  restoreTimers.clear();
  unregisterRollback?.();
});

// —— 二态确认删除（与清单同款；单实例收口在板内行间） ——

const confirmingId = ref<number | null>(null);
const delRefs = new Map<number, InstanceType<typeof DelButton>>();
// 列表重建计数：开板时勾选态粒子/环/描画动画重播（design renderBoard 重建语义）
const listKey = ref(0);

// 收口总线注册（A2）：板开合/页签切换时批量摘未决确认（归档板自家行也在列——
// design rollbackDelConfirms 收全局 .del-open）
let unregisterRollback: (() => void) | null = null;
onMounted(() => {
  unregisterRollback = registerDelConfirms(() => {
    if (confirmingId.value != null) {
      delRefs.get(confirmingId.value)?.rollBack();
      confirmingId.value = null;
    }
  });
});

function setDelRef(id: number, el: InstanceType<typeof DelButton> | null): void {
  if (el) delRefs.set(id, el);
  else delRefs.delete(id);
}

function onPress(item: TodoItem): void {
  if (confirmingId.value != null && confirmingId.value !== item.id) {
    delRefs.get(confirmingId.value)?.rollBack();
  }
  confirmingId.value = item.id;
}

async function onConfirm(item: TodoItem): Promise<void> {
  confirmingId.value = null;
  try {
    await invoke("todo_remove", { id: item.id });
    emit("changed");
  } catch (err) {
    console.error("删除失败", err);
  }
}

/** 确认态鼠标离开即回退（A1：V0.025 定案，归档行并入同款） */
function onCancel(item: TodoItem): void {
  if (confirmingId.value === item.id) confirmingId.value = null;
}

/** 退场钉高（⑤ = design collapseRow 第一步 1:1，与清单同款）：height auto→0 不可
 * 过渡，leave 前钉实测高度作过渡起点，缺失即"行直接消失" */
function pinLeaveHeight(el: Element): void {
  (el as HTMLElement).style.height = `${(el as HTMLElement).offsetHeight}px`;
}

/** 勾选退回（⑥ = design 归档分支时序 1:1）：invoke 成功后**乐观置位**——勾选框
 * 熄灭 + 删除线摘除（design input.checked=false + 摘 is-done 的立即置位，退回视觉
 * 起播），300ms 主拍播完才塌缩离场。逐行状态 + 逐行拍子（FIX003.5）：单值
 * restoringId 在快速连点时被第二行抢写（首行退回动画闪回 = 挂假勾行），单句柄
 * 拍子被 clearTimeout 掐死（TodoList toggleTimer 同根）——改 Set + Map<id, handle> */
const restoringIds = ref(new Set<number>());
const restoreTimers = new Map<number, number>();
async function restore(item: TodoItem): Promise<void> {
  try {
    await invoke("todo_toggle", { id: item.id });
    restoringIds.value.add(item.id);
    clearTimeout(restoreTimers.get(item.id));
    restoreTimers.set(
      item.id,
      window.setTimeout(() => {
        restoreTimers.delete(item.id);
        // 末拍收口：提前拍的 changed 会全量重拉，截断其他行在飞的主拍
        if (restoreTimers.size === 0) emit("changed");
      }, 300), // 主拍 300ms（用户定案）
    );
  } catch (err) {
    console.error("退回失败", err);
  }
}

// restoringIds 的清空必须等 items 真正更新移除该行之后（flush post）：changed 后
// refresh 是异步的——同步清会让该行在离场前恢复勾选态（粒子动画重播 + 删除线闪回，
// 真窗口实测 2026-09-28）。逐行摘除（FIX003.5）：仅清已从视图消失的 id，交错刷新
// （如删除重拉）不抢走在飞主拍行的退回视觉
watch(
  () => props.items,
  () => {
    if (restoringIds.value.size === 0) return;
    const present = new Set(props.items.map((it) => it.id));
    for (const id of [...restoringIds.value]) {
      if (!present.has(id)) restoringIds.value.delete(id);
    }
  },
  { flush: "post" },
);
</script>

<template>
  <!-- 归档按钮：常驻清单页左上角（非清单页由父级 hidden 控制） -->
  <button
    class="deleteButton"
    :class="{ open: isOpen, leaving: btnLeaving, entering: btnEntering }"
    :hidden="btnHidden"
    aria-label="Archive"
    @click="toggle"
  >
    <svg class="bin" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" aria-hidden="true">
      <path
        fill="#B5BAC1"
        stroke="#B5BAC1"
        stroke-width="1"
        stroke-linejoin="round"
        d="M0 2a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1v7.5a2.5 2.5 0 0 1-2.5 2.5h-9A2.5 2.5 0 0 1 1 12.5V5a1 1 0 0 1-1-1zm2 3v7.5A1.5 1.5 0 0 0 3.5 14h9a1.5 1.5 0 0 0 1.5-1.5V5zm13-3H1v2h14zM5 7.5a.5.5 0 0 1 .5-.5h5a.5.5 0 0 1 0 1h-5a.5.5 0 0 1-.5-.5"
      ></path>
    </svg>
    <span class="tooltip">归档</span>
  </button>

  <div ref="overlay" class="archive-overlay" :class="{ open: isOpen }" @mousedown="onOverlayDown">
    <div class="board-glass">
      <p class="board-title title-plate">
        已完成 <span class="archive-count">{{ items.length }}</span>
      </p>
      <p v-if="items.length === 0" class="empty">暂无已完成，去清单勾一条吧</p>
      <TransitionGroup
        ref="listEl"
        v-else
        tag="ul"
        name="todo"
        id="archive-list"
        class="board-list board-read"
        :key="listKey"
        :duration="320"
        @before-leave="pinLeaveHeight"
      >
        <li v-for="item in items" :key="item.id" class="todo-item">
          <!-- 整行点击驱动勾选退回（design 语义：checkbox pointer-events 关闭，
               行 click 触发 toggle；删除钮 stop 自有语义） -->
          <div
            class="todo-row"
            :class="{ 'is-done': !restoringIds.has(item.id) }"
            @click="restore(item)"
          >
            <NeonCheckbox :checked="!restoringIds.has(item.id)" />
            <span class="t-text">{{ item.text }}</span>
            <DelButton
              :ref="(el) => setDelRef(item.id, el as InstanceType<typeof DelButton>)"
              :confirming="confirmingId === item.id"
              @press="onPress(item)"
              @confirm="onConfirm(item)"
              @leave="onCancel(item)"
            />
          </div>
        </li>
      </TransitionGroup>
    </div>
  </div>
</template>

<!-- 归档按钮/浮层/行样式均已全局挂载（main.ts） -->
