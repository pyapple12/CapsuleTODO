<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { UI_TEXT_MAX_LEN } from "../../types";
import { shouldIgnoreImeEnter } from "../composables/imeEnter";
import { useAutoHideError } from "../composables/useAutoHideError";

// 添加行（PL008.7 重构）：REC 按钮（average-swan-99 直引配方）+ plastic-parrot-88 输入框。
// 回车/REC 双触发；空文本禁用；失败可见反馈（成功即清空并清除错误，沿 FIX001.5 模式）。
// PATCH002.66：错误短句固定“添加失败！”，10s 自隐；完整错误只落 console。
// added 单一事件（FIX004.21 收敛）：消费端 onTodoAdded 统一刷新 + 拉顶——
// 原 changed/added 双发使刷新链双跑（同一新增 6 个 invoke，其中 3 个纯重复）
const emit = defineEmits<{ added: [] }>();
const draft = ref("");
const busy = ref(false);
const { message: error, show: showError, hide: clearError } = useAutoHideError(10_000);

/** FIX014.30：中文输入法组字中的 Enter 只用于选词，不得提交待办。 */
function onAddEnter(e: KeyboardEvent): void {
  if (shouldIgnoreImeEnter(e)) return;
  void add();
}

async function add(): Promise<void> {
  const text = draft.value.trim();
  if (!text || busy.value) {
    return;
  }
  // FIX015.28：在飞锁 + 请求文本快照；请求期间的新输入不能被无条件清空。
  busy.value = true;
  try {
    await invoke("todo_add", { text });
    if (draft.value.trim() === text) {
      draft.value = "";
    }
    clearError();
    emit("added");
  } catch (err) {
    console.error("添加待办失败", err);
    showError("添加失败！");
  } finally {
    busy.value = false;
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
        :maxlength="UI_TEXT_MAX_LEN"
        placeholder="添加一条待办…"
        @keydown.enter="onAddEnter"
      />
      <button class="btn-add" :disabled="busy || draft.trim().length === 0" @click="add">
        <span class="btn-add-face"><span class="btn-add-text">REC</span></span>
      </button>
    </div>
    <p v-if="error" class="error">{{ error }}</p>
  </div>
</template>

<style scoped>
@import "../styles/addbar.css";
</style>
