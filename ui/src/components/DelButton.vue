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

const emit = defineEmits<{
  /** 首点：进入确认态（父级置 confirming=true） */
  press: [];
  /** 再点：执行窗口（200ms）结束后触发，父级执行删除 */
  confirm: [];
}>();

const root = ref<HTMLElement | null>(null);
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

/** 按压脉冲：del-press 类挂载 180ms 后自摘（remove+回流保证连点重播） */
function pulse(): void {
  const el = root.value;
  if (!el || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  el.classList.remove("del-press");
  void el.offsetWidth; // 强制回流：重入时动画从头重播
  el.classList.add("del-press");
  window.setTimeout(() => el.classList.remove("del-press"), 220);
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
});
</script>

<template>
  <button
    ref="root"
    class="del"
    :class="{ 'del-open': confirming }"
    aria-label="删除"
    @click.stop="onClick"
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
