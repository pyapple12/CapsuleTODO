<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

// 添加行（PL008.7 重构）：REC 按钮（average-swan-99 直引配方）+ plastic-parrot-88 输入框。
// 回车/REC 双触发；空文本禁用；失败可见反馈（成功即清空并清除错误，沿 FIX001.5 模式）
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
      <button class="btn-add" :disabled="draft.trim().length === 0" @click="add">
        <span class="btn-add-face"><span class="btn-add-text">REC</span></span>
      </button>
    </div>
    <p v-if="error" class="error">{{ error }}</p>
  </div>
</template>

<style scoped>
@import "../styles/addbar.css";
</style>
