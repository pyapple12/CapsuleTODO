<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { TodoItem } from "../../types";
import DelButton from "./DelButton.vue";
import NeonCheckbox from "./NeonCheckbox.vue";
import { useBoardRead } from "../composables/useBoardRead";
import { useGlassBar } from "../composables/useGlassBar";
import { bindOverlayState, syncVeils } from "../composables/useVeils";

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

let boardRead: { sync: () => void; settle: () => void } | null = null;
let glassBar: { sync: () => void } | null = null;

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

/** 开合归档板：开板重拉数据 + 动态落位 + 揭示动画 + 450ms 后 settle（揭示中几何中间态防御） */
async function toggle(): Promise<void> {
  const next = !isOpen.value;
  if (next) {
    emit("opened"); // 三板互斥：父级收其余两板（design setBoard 同款）
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
    // listEl ref 指向 TransitionGroup 组件实例——真实 UL 须经 $el 取（ref 非元素）
    const listDom = (listEl.value as unknown as { $el?: HTMLElement })?.$el ?? listEl.value;
    if (listDom && !boardRead) {
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
});

// —— 二态确认删除（与清单同款；单实例收口在板内行间） ——

const confirmingId = ref<number | null>(null);
const delRefs = new Map<number, InstanceType<typeof DelButton>>();

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

/** 勾选退回：确认态直接翻转（归档行删除线态，两拍确认不适用——退回是低危操作） */
async function restore(item: TodoItem): Promise<void> {
  try {
    await invoke("todo_toggle", { id: item.id });
    emit("changed");
  } catch (err) {
    console.error("退回失败", err);
  }
}
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
        class="board-list board-read"
        :duration="320"
      >
        <li v-for="item in items" :key="item.id" class="todo-item">
          <!-- 整行点击驱动勾选退回（design 语义：checkbox pointer-events 关闭，
               行 click 触发 toggle；删除钮 stop 自有语义） -->
          <div class="todo-row is-done" @click="restore(item)">
            <NeonCheckbox :checked="true" />
            <span class="t-text">{{ item.text }}</span>
            <DelButton
              :ref="(el) => setDelRef(item.id, el as InstanceType<typeof DelButton>)"
              :confirming="confirmingId === item.id"
              @press="onPress(item)"
              @confirm="onConfirm(item)"
            />
          </div>
        </li>
      </TransitionGroup>
    </div>
  </div>
</template>

<!-- 归档按钮/浮层/行样式均已全局挂载（main.ts） -->
