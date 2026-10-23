<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
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
const RETRY_MS = 10_000; // PATCH002.66：加载/保存失败 10s 自动重试

const content = ref("");
const savedContent = ref(""); // 最近一次成功保存的内容（脏判定基准）
const error = ref("");
const loadFailed = ref(false); // 载入失败态：禁编辑防"空白覆盖旧草稿"（FIX004.19）
const boardEl = ref<HTMLTextAreaElement | null>(null);
let debounceTimer = 0;
let retryTimer = 0;
let glassBar: { sync: () => void; destroy: () => void } | null = null;
let boardRead: { sync: () => void; settle: () => void; destroy: () => void } | null = null;

/** 10s 自动重试（加载/保存共用；成功即清 timer；失败重新排） */
function scheduleRetry(kind: "load" | "save"): void {
  window.clearTimeout(retryTimer);
  retryTimer = window.setTimeout(() => {
    retryTimer = 0;
    if (kind === "load") void load();
    else void flush();
  }, RETRY_MS);
}

/** 拉取白板内容（首启空串）：失败置错误行 + 禁编辑（防空白覆盖旧草稿），10s 自动重试 */
async function load(): Promise<void> {
  try {
    const text = await invoke<string>("whiteboard_load");
    content.value = text;
    savedContent.value = text;
    loadFailed.value = false;
    error.value = "";
    window.clearTimeout(retryTimer);
    retryTimer = 0;
  } catch (err) {
    loadFailed.value = true;
    error.value = "白板加载失败";
    console.error("whiteboard_load 拉取失败", err);
    scheduleRetry("load");
  }
}

/** 立即保存脏数据；成功返回 true（供切页/关窗 flush 判定）。
 * PATCH002.66：失败短句“白板保存失败”，完整错误 console；10s 自动重试；内容不锁。 */
async function flush(): Promise<boolean> {
  window.clearTimeout(debounceTimer);
  // FIX015.27：保存值必须在 await 前捕获；请求飞行期间的新输入不能被标记为已保存。
  const text = content.value;
  if (text === savedContent.value) {
    return true;
  }
  try {
    await invoke("whiteboard_save", { content: text });
    savedContent.value = text;
    error.value = "";
    window.clearTimeout(retryTimer);
    retryTimer = 0;
    // 保存期间继续输入 → 保留脏态并重新排一次防抖，不能把新输入吞掉。
    if (content.value !== text) {
      window.clearTimeout(debounceTimer);
      debounceTimer = window.setTimeout(() => {
        void flush();
      }, DEBOUNCE_MS);
    }
    return true;
  } catch (err) {
    console.error("whiteboard_save 保存失败", err);
    error.value = "白板保存失败";
    scheduleRetry("save");
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

/** 板级保存手势（PATCH002.43）：Ctrl+S / Ctrl+Enter = **保存 + 退出输入**（白板不关板，
 * 与详情板 Ctrl+Enter 区分）。挂 textarea 而非 window——白板是 v-show 常驻挂载，window 级
 * 会在清单/气泡页也截走 Ctrl+S 并去保存看不见的白板；挂 textarea 天然"只在输入中生效"，
 * 正好等于"退出输入"语义。失败时 `flush()` 已写错误行（沿白板既有可见口径） */
function onBoardKeydown(e: KeyboardEvent): void {
  if (e.isComposing) return;
  const key = e.key.toLowerCase();
  if (!(e.ctrlKey || e.metaKey) || (key !== "s" && e.key !== "Enter")) return;
  e.preventDefault(); // Ctrl+S 免触发"保存网页"
  void flush().then(() => boardEl.value?.blur());
}

defineExpose({ flush });

// 内容变化重算 ▲▼ 三角显隐（bug4 修复）：textarea 内容清空/增删不产生 DOM 子节点
// 变化（value 非子节点），useBoardRead 的 MutationObserver 观察不到——scroll 事件
// 也不会因删除触发，三角残留删除前状态；数据层 watch 补触发源
watch(content, () => {
  boardRead?.sync();
});

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
  window.clearTimeout(retryTimer);
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
        @keydown="onBoardKeydown"
      ></textarea>
    </div>
    <p v-if="error" class="error">{{ error }}</p>
  </section>
</template>

<!-- 白板样式已全局挂载（styles/whiteboard.css 经 main.ts） -->
