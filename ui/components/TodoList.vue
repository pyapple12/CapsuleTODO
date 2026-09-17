<script setup lang="ts">
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { TodoItem } from "../types";

// 纯展示组件：数据源 = todo_list 排序视图（Rust 侧排序），前端只按 done 分组渲染；
// 已完成折叠区默认收起（PL002 定案），条目悬停显删除钮（直接删不确认——一期定案）
const props = defineProps<{ items: TodoItem[] }>();
const emit = defineEmits<{ changed: [] }>();

const active = computed(() => props.items.filter((it) => !it.done));
const finished = computed(() => props.items.filter((it) => it.done));
const doneOpen = ref(false);

/** 勾选翻转：成功后通知父级重拉排序视图；失败可见（console），不静默 */
async function toggle(item: TodoItem): Promise<void> {
  try {
    await invoke("todo_toggle", { id: item.id });
    emit("changed");
  } catch (err) {
    console.error("勾选失败", err);
  }
}

/** 删除条目：直接删不确认（一期定案 KISS） */
async function remove(item: TodoItem): Promise<void> {
  try {
    await invoke("todo_remove", { id: item.id });
    emit("changed");
  } catch (err) {
    console.error("删除失败", err);
  }
}
</script>

<template>
  <section class="list">
    <p v-if="items.length === 0" class="empty">暂无待办，添加一条吧</p>
    <ul v-else class="group">
      <li v-for="item in active" :key="item.id" class="todo-row" @click.stop="toggle(item)">
        <span class="check" aria-hidden="true"></span>
        <span class="text">{{ item.text }}</span>
        <button class="del" title="删除" @click.stop="remove(item)">×</button>
      </li>
    </ul>
    <div v-if="finished.length > 0" class="done-group">
      <button class="done-head" @click="doneOpen = !doneOpen">
        <span class="caret">{{ doneOpen ? "▾" : "▸" }}</span>
        已完成 {{ finished.length }}
      </button>
      <ul v-show="doneOpen" class="group">
        <li
          v-for="item in finished"
          :key="item.id"
          class="todo-row is-done"
          @click.stop="toggle(item)"
        >
          <span class="check checked" aria-hidden="true">✓</span>
          <span class="text">{{ item.text }}</span>
          <button class="del" title="删除" @click.stop="remove(item)">×</button>
        </li>
      </ul>
    </div>
  </section>
</template>

<style scoped>
.list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 100%;
  min-height: 0;
  flex: 1;
  overflow-y: auto;
}

.group {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.empty {
  margin: 24px 0;
  font-size: 13px;
  text-align: center;
  opacity: 0.55;
}

/* 清单行：面板级玻璃小件（panel 纱 + 细描边），整行点击 = 勾选 */
.todo-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  border-radius: 10px;
  background: var(--panel-bg);
  box-shadow: var(--panel-stroke);
  cursor: pointer;
}

/* 自绘复选框：方圆框，完成态填充 accent 并显对勾 */
.check {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border: 1.5px solid var(--ink-2);
  border-radius: 6px;
  font-size: 12px;
  line-height: 1;
  color: transparent;
}

.check.checked {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.text {
  flex: 1;
  font-size: 13.5px;
  line-height: 1.35;
  word-break: break-word;
}

/* 完成态定案：删除线 + 变淡 */
.is-done .text {
  text-decoration: line-through;
  opacity: 0.55;
}

/* 悬停删除钮：常驻低调，悬停行时提亮 */
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

.todo-row:hover .del {
  opacity: 0.7;
}

.del:hover {
  background: rgba(229, 72, 77, 0.15);
  color: #e5484d;
  opacity: 1;
}

/* 折叠区表头：文本按钮语义 */
.done-group {
  margin-top: 2px;
}

.done-head {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  opacity: 0.6;
  cursor: pointer;
}

.caret {
  font-size: 10px;
}
</style>
