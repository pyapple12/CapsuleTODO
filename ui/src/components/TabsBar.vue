<script setup lang="ts">
// 页签组件（PL008.4）：uiverse heavy-dragonfly-92 改造 Vue 化——radio 滑块改
// modelValue 驱动；glider 位移 = 页签序号 × 100%（CSS 定宽 80px 前提，与 design 一致）
const props = defineProps<{
  /** 当前页签键（v-model） */
  modelValue: string;
  /** 页签定义：key/label/徽章计数（badgeCount 非 0 且给定才显示） */
  tabs: ReadonlyArray<{ key: string; label: string; badgeCount?: number }>;
}>();

const emit = defineEmits<{ "update:modelValue": [key: string] }>();

/** 页签序号 → glider translateX 百分比（design：radio-1/2/3 → 0/100%/200%） */
function gliderStyle(): { transform: string } {
  const idx = Math.max(
    0,
    props.tabs.findIndex((t) => t.key === props.modelValue),
  );
  return { transform: `translateX(${idx * 100}%)` };
}
</script>

<template>
  <nav class="tabs">
    <template v-for="tab in tabs" :key="tab.key">
      <input
        :id="`tab-radio-${tab.key}`"
        type="radio"
        name="tabs"
        :checked="modelValue === tab.key"
        @change="emit('update:modelValue', tab.key)"
      />
      <label :for="`tab-radio-${tab.key}`" class="tab">
        {{ tab.label
        }}<span v-if="tab.badgeCount != null && tab.badgeCount > 0" class="notification">{{
          tab.badgeCount >= 10 ? "9+" : tab.badgeCount
        }}</span>
      </label>
    </template>
    <span class="glider" :style="gliderStyle()"></span>
  </nav>
</template>

<style scoped>
/* 相对仓库根解析（vite 根 = 仓库根，组件相对路径会落到 ui/src/components/styles/ 之外） */
@import "../styles/tabs.css";
</style>
