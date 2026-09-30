<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useBoardRead } from "../composables/useBoardRead";
import { useGlassBar } from "../composables/useGlassBar";

// ===== 白板页（PL014.1 换装实验场形态）：board-shell 卡壳静止承担玻璃外观，
// textarea 透明填壳——mask 只淡文字墨迹；板内滑杆（right 7.75，居中于编辑区右缘
// ↔ 卡片描边缝隙）+ 整板阅读（textarea 退化：恒定软边带顶 6/底 14 + 到底抬带）。
// 防抖 800ms 自动保存（design whiteboard.js 同拍），flush 暴露给 App 供切页与
// 关窗前强制保存；保存失败错误行可见（严格报错策略）。自动保存属默认行为不做
// 展示（2026-09-28 目验定案：状态行"编辑中…/已自动保存"移除，APP 与 design 同步）

const DEBOUNCE_MS = 800; // 防抖自动保存间隔（design whiteboard.js 同值）

const content = ref("");
const savedContent = ref(""); // 最近一次成功保存的内容（脏判定基准）
const error = ref("");
const loadFailed = ref(false); // 载入失败态：禁编辑防"空白覆盖旧草稿"（FIX004.19）
const boardEl = ref<HTMLTextAreaElement | null>(null);
let debounceTimer = 0;
let glassBar: { sync: () => void; destroy: () => void } | null = null;
let boardRead: { sync: () => void; settle: () => void; destroy: () => void } | null = null;

/** 拉取白板内容（首启空串）：失败置错误行 + 禁编辑（防空白覆盖旧草稿），重试钮可再拉 */
async function load(): Promise<void> {
  try {
    const text = await invoke<string>("whiteboard_load");
    content.value = text;
    savedContent.value = text;
    loadFailed.value = false;
    error.value = "";
  } catch (err) {
    loadFailed.value = true;
    error.value = `白板加载失败：${String(err)}`;
    console.error("whiteboard_load 拉取失败", err);
  }
}

/** 重试载入（错误行入口）：成功解除禁编辑并恢复内容 */
function retryLoad(): void {
  void load();
}

/** 立即保存脏数据；成功返回 true（供切页/关窗 flush 判定） */
async function flush(): Promise<boolean> {
  window.clearTimeout(debounceTimer);
  if (content.value === savedContent.value) {
    return true;
  }
  try {
    await invoke("whiteboard_save", { content: content.value });
    savedContent.value = content.value;
    error.value = "";
    return true;
  } catch (err) {
    error.value = `保存失败：${String(err)}`;
    return false;
  }
}

/** 输入：脏则重置防抖计时 + 同步滑杆/溶解带（design syncDetailBar 同款） */
function onInput(): void {
  if (content.value === savedContent.value) {
    return;
  }
  window.clearTimeout(debounceTimer);
  debounceTimer = window.setTimeout(() => {
    void flush();
  }, DEBOUNCE_MS);
  glassBar?.sync();
}

defineExpose({ flush });

onMounted(() => {
  void load();
  // 板内挂点（design glass-bar.js：滑杆悬浮右缝 7.75 + 整板阅读恒定软边带）。
  // composable 在异步上下文挂载时生命周期钩子失效，destroy 由本组件持有
  void nextTick(() => {
    if (boardEl.value && !glassBar) {
      glassBar = useGlassBar(boardEl.value, { right: 7.75 });
      boardRead = useBoardRead(boardEl.value);
    }
  });
});

onUnmounted(() => {
  window.clearTimeout(debounceTimer);
  glassBar?.destroy();
  boardRead?.destroy();
  glassBar = null;
  boardRead = null;
});
</script>

<template>
  <section class="whiteboard">
    <div class="board-shell">
      <textarea
        ref="boardEl"
        v-model="content"
        id="wb-board"
        class="board"
        placeholder="随手记点什么…"
        spellcheck="false"
        :disabled="loadFailed"
        @input="onInput"
      ></textarea>
    </div>
    <p v-if="error" class="error">
      {{ error }}
      <button v-if="loadFailed" class="retry-btn" @click="retryLoad">重试</button>
    </p>
  </section>
</template>

<!-- 白板样式已全局挂载（styles/whiteboard.css 经 main.ts） -->
