<script setup lang="ts">
import { nextTick, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { TodoItem } from "../../types";
import DelButton from "./DelButton.vue";
import NeonCheckbox from "./NeonCheckbox.vue";
import { useBoardRead } from "../composables/useBoardRead";
import { useGlassBar } from "../composables/useGlassBar";
import { bindOverlayState, syncVeils } from "../composables/useVeils";

// ===== 归档板（PL011.2）：已完成条目管理（勾选退回 / 删除二态）。
// 板揭示 = origin 注入 scale 回弹（从归档图标飞出）；三角 hintHost = 板内玻璃
// （V0.028 方案 B：随板缩放生长）；滑杆锚板内（right 4.5，V0.016 定值）。
// 行为红线（V0.015 定案）：归档行点正文 no-op；归档态不显示 note；板内浮现动画不做
// （板开时清单被遮，入档必发生在板关）。

const props = defineProps<{
  /** 归档视图（父级经 todo_archive_list 拉取） */
  items: TodoItem[];
}>();

const emit = defineEmits<{ changed: [] }>();

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

// —— 归档按钮出入场编排（archive-fx.js 1:1，定时器替代 animationend） ——

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

/** 开合归档板：开板重拉数据 + 揭示动画 + 450ms 后 settle（揭示中几何中间态防御） */
async function toggle(): Promise<void> {
  const next = !isOpen.value;
  setArchiveVisible(next);
  if (next) {
    isOpen.value = true;
    syncVeils();
    await nextTick();
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

defineExpose({ toggle });

/** 勾选退回：确认态直接翻转（归档行删除线态，两拍确认不适用——退回是低危操作） */
async function restore(item: TodoItem): Promise<void> {
  try {
    await invoke("todo_toggle", { id: item.id });
    emit("changed");
  } catch (err) {
    console.error("退回失败", err);
  }
}

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

/** 点板空白收板（与再点按钮双出口；行内交互 stop 传播不误触） */
function onOverlayDown(e: MouseEvent): void {
  if ((e.target as HTMLElement).closest(".board-glass") === null) toggle();
}

onUnmounted(() => {
  window.clearTimeout(animTimer);
});
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
    <svg class="bin" viewBox="0 0 16 16" fill="#B5BAC1" stroke="#B5BAC1" stroke-width="1.2">
      <path
        d="M6.5 1h3a.5.5 0 0 1 .5.5V2h3a.5.5 0 0 1 0 1h-.5v9.5A2.5 2.5 0 0 1 10 15H6a2.5 2.5 0 0 1-2.5-2.5V3H3a.5.5 0 0 1 0-1h3v-.5a.5.5 0 0 1 .5-.5ZM4.5 3v9.5A1.5 1.5 0 0 0 6 14h4a1.5 1.5 0 0 0 1.5-1.5V3h-7Z"
      />
    </svg>
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

<style scoped>
@import "../styles/archive.css";
@import "../styles/todos.css";
</style>
