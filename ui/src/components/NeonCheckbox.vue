<script setup lang="ts">
// 勾选框组件（PL008.5）：uiverse hot-dragonfly-56 改造 Vue 化。
// 沿实验场定案：pointer-events 关闭（整行点击驱动，组件只做视觉状态位）、
// input 仅作 CSS 状态锚点（checked prop 直驱）；流光 borderFlow1-4 随勾选启停。
const props = defineProps<{
  /** 勾选态（由父级行数据驱动） */
  checked: boolean;
}>();

const emit = defineEmits<{ toggle: [] }>();

/** 框被点击时通知父级（pointer-events 已关，此事件实际由行容器转发调用） */
function onClick(): void {
  emit("toggle");
}
void onClick; // 保留交互语义位：PL010 行接线时由行容器调用
</script>

<template>
  <div
    class="neon-checkbox"
    :class="{ checked: props.checked }"
    role="checkbox"
    :aria-checked="props.checked"
  >
    <input type="checkbox" tabindex="-1" :checked="props.checked" />
    <div class="neon-checkbox__frame">
      <div class="neon-checkbox__box">
        <div class="neon-checkbox__check-container">
          <svg class="neon-checkbox__check" viewBox="0 0 24 24">
            <path d="M5 12l5 5L20 7"></path>
          </svg>
        </div>
        <div class="neon-checkbox__glow"></div>
        <div class="neon-checkbox__borders">
          <span></span><span></span><span></span><span></span>
        </div>
        <div class="neon-checkbox__particles">
          <span></span><span></span><span></span><span></span><span></span><span></span><span></span
          ><span></span><span></span><span></span><span></span><span></span>
        </div>
        <div class="neon-checkbox__rings">
          <div class="ring"></div>
          <div class="ring"></div>
          <div class="ring"></div>
        </div>
        <div class="neon-checkbox__sparks">
          <span></span><span></span><span></span><span></span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@import "../styles/neon-checkbox.css";
</style>
