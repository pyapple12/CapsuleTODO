<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { BubbleItem, BubbleSnapshot } from "../types";

// 气泡页：捕获剪贴板 → 气泡列表（点击复制回）→ 满 5 横幅提醒（remind 由 Rust 裁决，前端零业务）
// → 一键清空二态确认（首点变红"确认清空 N 条？"再点执行，3 秒未点复位——沿一期无弹窗基调）
const items = ref<BubbleItem[]>([]);
const remind = ref(false);
const error = ref("");
const copiedVisible = ref(false);
const confirmingClear = ref(false);
let copiedTimer = 0;
let confirmTimer = 0;

/** 拉取气泡快照 */
async function refresh(): Promise<void> {
  try {
    const snapshot = await invoke<BubbleSnapshot>("bubble_list");
    items.value = snapshot.items;
    remind.value = snapshot.remind;
  } catch (err) {
    console.error("bubble_list 拉取失败", err);
  }
}

/** 捕获剪贴板文本为新气泡 */
async function capture(): Promise<void> {
  try {
    await invoke("bubble_capture");
    error.value = "";
    await refresh();
  } catch (err) {
    error.value = `捕获失败：${String(err)}`;
  }
}

/** 点击气泡 = 复制内容回剪贴板，状态行提示 2 秒 */
async function copyBack(item: BubbleItem): Promise<void> {
  try {
    await invoke("bubble_copy", { id: item.id });
    copiedVisible.value = true;
    window.clearTimeout(copiedTimer);
    copiedTimer = window.setTimeout(() => {
      copiedVisible.value = false;
    }, 2000);
  } catch (err) {
    console.error("复制失败", err);
  }
}

/** 删除单条气泡（直接删不确认，沿一期定案） */
async function remove(item: BubbleItem): Promise<void> {
  try {
    await invoke("bubble_remove", { id: item.id });
    await refresh();
  } catch (err) {
    console.error("删除失败", err);
  }
}

/** 一键清空（二态确认：一次删多条破坏性高于单删） */
async function clearAll(): Promise<void> {
  if (!confirmingClear.value) {
    confirmingClear.value = true;
    window.clearTimeout(confirmTimer);
    confirmTimer = window.setTimeout(() => {
      confirmingClear.value = false;
    }, 3000);
    return;
  }
  window.clearTimeout(confirmTimer);
  confirmingClear.value = false;
  try {
    await invoke("bubble_clear");
    await refresh();
  } catch (err) {
    console.error("清空失败", err);
  }
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <section class="bubbles">
    <div class="actions">
      <button class="capture" @click="capture">⧉ 捕获剪贴板</button>
      <span v-if="copiedVisible" class="copied">已复制到剪贴板</span>
    </div>
    <p v-if="error" class="error">{{ error }}</p>
    <div v-if="remind" class="banner">
      <span>气泡已满 5 个，该清理了</span>
      <button class="clear" :class="{ confirming: confirmingClear }" @click="clearAll">
        {{ confirmingClear ? `确认清空 ${items.length} 条？` : "一键清空" }}
      </button>
    </div>
    <ul v-if="items.length > 0" class="group">
      <li v-for="item in items" :key="item.id" class="bubble-row" @click="copyBack(item)">
        <span class="text">{{ item.text }}</span>
        <button class="del" title="删除" @click.stop="remove(item)">×</button>
      </li>
    </ul>
    <p v-else class="empty">暂无气泡，点上方捕获剪贴板</p>
  </section>
</template>

<style scoped>
.bubbles {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 100%;
  min-height: 0;
  flex: 1;
}

.actions {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  gap: 8px;
}

.capture {
  flex: 1;
  padding: 8px 0;
  border: none;
  border-radius: 10px;
  background: var(--accent);
  color: #fff;
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}

.copied {
  flex-shrink: 0;
  font-size: 12px;
  opacity: 0.7;
}

.error {
  margin: 0;
  font-size: 12px;
  color: #e5484d;
}

/* 满 5 提醒横幅：暖色玻璃小件 */
.banner {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 10px;
  background: rgba(230, 170, 0, 0.15);
  font-size: 12.5px;
}

.clear {
  flex-shrink: 0;
  padding: 4px 10px;
  border: none;
  border-radius: 8px;
  background: rgba(229, 72, 77, 0.15);
  color: inherit;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.clear.confirming {
  background: #e5484d;
  color: #fff;
}

.group {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
  overflow-y: auto;
}

/* 气泡条：整行点击 = 复制回剪贴板 */
.bubble-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  border-radius: 10px;
  background: var(--panel-bg);
  box-shadow: var(--panel-stroke);
  cursor: pointer;
}

.text {
  flex: 1;
  font-size: 13px;
  line-height: 1.35;
  word-break: break-word;
  /* 气泡截断展示：最多两行，全文走复制回 */
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.del {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: inherit;
  font-size: 14px;
  line-height: 1;
  opacity: 0.25;
  cursor: pointer;
}

.bubble-row:hover .del {
  opacity: 0.7;
}

.del:hover {
  background: rgba(229, 72, 77, 0.15);
  color: #e5484d;
  opacity: 1;
}

.empty {
  margin: 24px 0;
  font-size: 13px;
  text-align: center;
  opacity: 0.55;
}
</style>
