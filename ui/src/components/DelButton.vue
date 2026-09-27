<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";

// 删除按钮组件（PL008.6）：uiverse smart-emu-83 改造 Vue 化 + 二态盖翻确认全套
// （V0.025–V0.028 定案形态：盖翻 0.5s + Delete 字渐隐 + 按压脉冲 + 200ms 执行窗口锁
// + 离开即回退无合盖动画）。状态语义对齐 design：
// confirming = del-open（盖翻锁定红底）；pressing = del-press（脉冲动画类）
const props = defineProps<{
  /** 确认态（del-open；单实例收口由父级管理——多行同开时父级只保留一个 true） */
  confirming: boolean;
}>();

const emit = defineEmits<{ press: []; confirm: []; leave: [] }>();

const root = ref<HTMLElement | null>(null);
const pressing = ref(false); // 脉冲动画类（Vue 响应式驱动——classList.add 外部加类会被
// 渲染 patch 的 class 重写抹掉，动画一帧未渲染即消失，2026-09-28 真窗口实测）
let pressTimer: number | undefined;
let confirmTimer: number | undefined;
let pendingConfirm = false;

/** 点击分派：首点 press、再点启动执行窗口（窗口内重复点击忽略——防双删） */
function onClick(): void {
  if (pendingConfirm) return; // 执行窗口锁：200ms 内再点忽略
  if (props.confirming) {
    pendingConfirm = true;
    pulse();
    confirmTimer = window.setTimeout(() => {
      pendingConfirm = false;
      emit("confirm");
    }, 200);
  } else {
    pulse();
    emit("press");
  }
}

/** 按压脉冲：pressing 置真挂 del-press 动画（0.18s 缩放），220ms 后自摘；
 * remove+回流由 Vue class 切换天然完成重播 */
function pulse(): void {
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  pressing.value = false;
  void (root.value as HTMLElement | null)?.offsetWidth; // 强制回流：连点重播
  pressing.value = true;
  window.clearTimeout(pressTimer);
  pressTimer = window.setTimeout(() => (pressing.value = false), 220);
}

/** 确认态鼠标离开即回退（design mouseout 委托等价：mouseleave 天然不含按钮内部
 * 子元素间移动——relatedTarget 豁免免掉）；执行窗锁期间红底保持不被离开打断。
 * 事件名用 leave（cancel 与 HTMLDialogElement 原生事件同名的键会触发 vue-tsc
 * defineEmits 重载误判，实测 TS2344） */
function onMouseLeave(): void {
  if (!props.confirming || pendingConfirm) return;
  emit("leave");
}

/** 父级收口接口：摘确认态（板开合/换页/单实例换目标时调用） */
function rollBack(): void {
  // confirming 由父级置 false；此处仅防御性清执行窗计时器
  if (confirmTimer !== undefined) {
    window.clearTimeout(confirmTimer);
    confirmTimer = undefined;
    pendingConfirm = false;
  }
}

defineExpose({ rollBack });

onBeforeUnmount(() => {
  if (confirmTimer !== undefined) window.clearTimeout(confirmTimer);
  window.clearTimeout(pressTimer);
});
</script>

<template>
  <button
    ref="root"
    class="del"
    :class="{ 'del-open': confirming, 'del-press': pressing }"
    aria-label="删除"
    @click.stop="onClick"
    @mouseleave="onMouseLeave"
  >
    <svg class="del-icon" viewBox="0 0 448 512">
      <path
        d="M32 128H416V448c0 35.3-28.7 64-64 64H96c-35.3 0-64-28.7-64-64V128zm96 64c-8.8 0-16 7.2-16 16V432c0 8.8 7.2 16 16 16s16-7.2 16-16V208c0-8.8-7.2-16-16-16zm96 0c-8.8 0-16 7.2-16 16V432c0 8.8 7.2 16 16 16s16-7.2 16-16V208c0-8.8-7.2-16-16-16zm96 0c-8.8 0-16 7.2-16 16V432c0 8.8 7.2 16 16 16s16-7.2 16-16V208c0-8.8-7.2-16-16-16z"
      ></path>
      <path
        class="del-lid"
        d="M135.2 17.7C140.6 6.8 151.7 0 163.8 0H284.2c12.1 0 23.2 6.8 28.6 17.7L320 32h96c17.7 0 32 14.3 32 32s-14.3 32-32 32H32C14.3 96 0 81.7 0 64S14.3 32 32 32h96l7.2-14.3z"
      ></path>
    </svg>
  </button>
</template>

<style scoped>
@import "../styles/del-button.css";
</style>
