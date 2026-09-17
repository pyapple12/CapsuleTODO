<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

// 白板页：单块文本草稿区，防抖自动保存（操作即落库）；
// flush 暴露给 App.vue 供切页与关窗前强制保存；保存失败错误行可见（沿 AddBar 模式）
const DEBOUNCE_MS = 800; // 防抖自动保存间隔（可调）

const content = ref("");
const savedContent = ref(""); // 最近一次成功保存的内容（脏判定基准）
const status = ref(""); // 空 = 无状态；"编辑中…" / "✓ 已自动保存"
const error = ref("");
let debounceTimer = 0;

/** 拉取白板内容（首启空串） */
async function load(): Promise<void> {
  try {
    const text = await invoke<string>("whiteboard_load");
    content.value = text;
    savedContent.value = text;
  } catch (err) {
    console.error("whiteboard_load 拉取失败", err);
  }
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
    status.value = "✓ 已自动保存";
    return true;
  } catch (err) {
    error.value = `保存失败：${String(err)}`;
    return false;
  }
}

/** 输入：脏则置状态并重置防抖计时 */
function onInput(): void {
  if (content.value === savedContent.value) {
    status.value = "";
    return;
  }
  status.value = "编辑中…";
  window.clearTimeout(debounceTimer);
  debounceTimer = window.setTimeout(() => {
    void flush();
  }, DEBOUNCE_MS);
}

defineExpose({ flush });

onMounted(() => {
  void load();
});

onUnmounted(() => {
  window.clearTimeout(debounceTimer);
});
</script>

<template>
  <section class="whiteboard">
    <textarea
      v-model="content"
      class="board"
      placeholder="随手记点什么…"
      spellcheck="false"
      @input="onInput"
    ></textarea>
    <p v-if="error" class="error">{{ error }}</p>
    <p v-else-if="status" class="status">{{ status }}</p>
  </section>
</template>

<style scoped>
.whiteboard {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
  min-height: 0;
  flex: 1;
}

/* 草稿区：面板级玻璃小件，占满剩余空间 */
.board {
  flex: 1;
  min-height: 0;
  padding: 10px 12px;
  border: none;
  border-radius: 10px;
  background: var(--panel-bg);
  box-shadow: var(--panel-stroke);
  color: inherit;
  font: inherit;
  font-size: 13px;
  line-height: 1.5;
  resize: none;
  outline: none;
  word-break: break-word;
}

.board::placeholder {
  color: var(--ink-2);
}

.status {
  margin: 0;
  font-size: 12px;
  opacity: 0.55;
}

.error {
  margin: 0;
  font-size: 12px;
  color: #e5484d;
}
</style>
