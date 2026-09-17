<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

// 添加行：回车/按钮双触发；空文本禁用；失败可见反馈（成功即清空并清除错误，沿 FIX001.5 模式）
const emit = defineEmits<{ changed: [] }>();
const draft = ref("");
const error = ref("");

async function add(): Promise<void> {
  const text = draft.value.trim();
  if (!text) {
    return;
  }
  try {
    await invoke("todo_add", { text });
    draft.value = "";
    error.value = "";
    emit("changed");
  } catch (err) {
    error.value = `添加失败：${String(err)}`;
  }
}
</script>

<template>
  <div class="addbar">
    <div class="row">
      <input
        v-model="draft"
        class="input"
        type="text"
        placeholder="添加一条待办…"
        @keydown.enter="add"
      />
      <button class="btn" :disabled="draft.trim().length === 0" @click="add">＋</button>
    </div>
    <p v-if="error" class="error">{{ error }}</p>
  </div>
</template>

<style scoped>
.addbar {
  flex-shrink: 0;
  width: 100%;
}

.row {
  display: flex;
  gap: 6px;
}

/* 输入框：面板级玻璃小件 */
.input {
  flex: 1;
  min-width: 0;
  padding: 8px 10px;
  border: none;
  border-radius: 10px;
  background: var(--panel-bg);
  box-shadow: var(--panel-stroke);
  color: inherit;
  font: inherit;
  font-size: 13px;
  outline: none;
}

.input::placeholder {
  color: var(--ink-2);
}

.btn {
  flex-shrink: 0;
  width: 34px;
  border: none;
  border-radius: 10px;
  background: var(--accent);
  color: #fff;
  font-size: 16px;
  line-height: 1;
  cursor: pointer;
}

.btn:disabled {
  opacity: 0.35;
  cursor: default;
}

.error {
  margin: 6px 2px 0;
  font-size: 12px;
  color: #e5484d;
}
</style>
