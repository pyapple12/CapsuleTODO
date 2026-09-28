<script setup lang="ts">
// 勾选框组件（PL008.5）：uiverse hot-dragonfly-56 改造 Vue 化。
// 沿实验场定案：pointer-events 关闭（整行点击驱动，行容器按点击坐标分流到勾选分支），
// input 仅作 CSS 状态锚点（checked prop 直驱）；流光 borderFlow1-4 随勾选启停。
// 纯视觉组件零事件出口（FIX003.10⑤：旧 emit("toggle")/onClick 死线已删——
// 无人调用且父级 @toggle 接的是永不触发的事件）
const props = defineProps<{
  /** 勾选态（由父级行数据驱动） */
  checked: boolean;
}>();
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
