<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { bindOverlayState, syncVeils } from "../composables/useVeils";

// ===== 设置板（design panels.js setSettings + theme.js 1:1 移植）：主题双开关
// （跟随系统 + 日夜切换）与气泡提醒数量步进（1~20，会话内有效）。与归档板同构：
// 齿轮原点缩放飞出、双出口收板（再点齿轮 / 点板外）、三板互斥（经 opened 上抛父级）、
// 开板保色（.open）。主题三态落 :root[data-theme]（glass.css 双块令牌接管），换档后
// 上抛 themeChanged 让标题粒子按新 accent 重建 =====

const emit = defineEmits<{
  opened: [];
  themeChanged: [];
  "update:maxBubbles": [value: number];
}>();

const props = defineProps<{
  /** 气泡提醒数量（父级持有，BubblesView 警告阈值同源） */
  maxBubbles: number;
}>();

const overlay = ref<HTMLElement | null>(null);
const isOpen = ref(false);

// 帘联动注册：设置板开 = 其余滑杆/三角吃帘
bindOverlayState("settings", () => isOpen.value);

// —— 开合编排（panels.js setSettings 同构） ——

/** 开合设置板：开板动态落位（板顶=页签顶-4）+ 齿轮中心揭示原点 */
async function toggle(): Promise<void> {
  const next = !isOpen.value;
  if (next) {
    emit("opened"); // 三板互斥：父级收其余两板
    isOpen.value = true;
    syncVeils();
    await nextTick();
    const board = document.getElementById("board");
    const tabsEl = document.querySelector(".tabs");
    if (board && tabsEl && overlay.value) {
      const br = board.getBoundingClientRect();
      const tr = tabsEl.getBoundingClientRect();
      const top = Math.max(0, tr.top - br.top - 4);
      overlay.value.style.top = `${top}px`;
      // 齿轮中心的板内坐标（板左缘 14px，齿轮中心卡内 x = 卡宽 - 落位10 - 半宽15）
      overlay.value.style.setProperty("--origin-x", `${Math.round(br.width - 20 - 14)}px`);
      overlay.value.style.setProperty("--origin-y", `${20 - top}px`);
    }
  } else {
    isOpen.value = false;
    syncVeils();
  }
}

/** 收板（互斥出口：归档/详情板开启时由父级调用）；已关早退 */
function close(): void {
  if (!isOpen.value) return;
  isOpen.value = false;
  syncVeils();
}

defineExpose({ toggle, close });

/** 点板空白收板；点板外任何位置收板（panels.js 同款 document 级） */
function onOverlayDown(e: MouseEvent): void {
  if ((e.target as HTMLElement).closest(".board-glass") === null) toggle();
}

function onDocClick(e: MouseEvent): void {
  if (!isOpen.value) return;
  const t = e.target as HTMLElement;
  if (!t.isConnected) return;
  if (t.closest(".settings-overlay") || t.closest(".settingsButton")) return;
  close();
}

// —— 主题三态（theme.js 1:1）：0 跟随系统 / 1 浅色 / 2 暗色；默认手动模式取系统当前深浅 ——

const systemDark = window.matchMedia("(prefers-color-scheme: dark)");
const themeIdx = ref<0 | 1 | 2>(systemDark.matches ? 2 : 1);
// 跟随模式下系统深浅变化的刷新触发器：matchMedia.matches 不是响应式源，
// computed 里读它不会自动重算——onSystemChange 里 bump 一次驱动档位显示跟随
const systemTick = ref(0);

const followChecked = computed(() => themeIdx.value === 0);
const daynightDisabled = computed(() => themeIdx.value === 0);
const daynightChecked = computed(() => {
  systemTick.value; // 响应式依赖登记（A9）
  return themeIdx.value === 0 ? systemDark.matches : themeIdx.value === 2;
});
const daynightDesc = computed(() =>
  themeIdx.value === 0 ? "跟随系统当前深浅自动切换" : "手动选择浅色或暗色",
);

/** 档位应用与界面同步：data-theme 落根元素（跟随 = 移除），换档后粒子按新 accent 重建 */
function applyTheme(idx: 0 | 1 | 2): void {
  if (themeIdx.value === idx) return;
  themeIdx.value = idx;
}

watch(
  themeIdx,
  (idx) => {
    if (idx === 0) {
      delete document.documentElement.dataset.theme;
    } else {
      document.documentElement.dataset.theme = idx === 2 ? "dark" : "light";
    }
    emit("themeChanged");
  },
  { immediate: true },
);

/** 跟随系统开关：关闭瞬间以系统当前深浅作为日夜档起始（design 定案） */
function onFollowChange(e: Event): void {
  const on = (e.target as HTMLInputElement).checked;
  applyTheme(on ? 0 : systemDark.matches ? 2 : 1);
}

function onDaynightChange(e: Event): void {
  applyTheme((e.target as HTMLInputElement).checked ? 2 : 1);
}

/** 跟随期间系统深浅实时变化 → 界面档位同步（design theme.js syncThemeControls 同款；
 * bump 触发器让读 matches 的 computed 重算，A9） */
function onSystemChange(): void {
  systemTick.value += 1;
  if (themeIdx.value === 0) emit("themeChanged");
}

onMounted(() => {
  document.addEventListener("click", onDocClick);
  systemDark.addEventListener("change", onSystemChange);
});

onUnmounted(() => {
  document.removeEventListener("click", onDocClick);
  systemDark.removeEventListener("change", onSystemChange);
});

// —— 气泡提醒数量步进（panels.js 同款）：范围 1~20 钳制，会话内有效（design 定案） ——

const BUBBLE_MAX_LIMIT = 20;

function step(delta: -1 | 1): void {
  const next = Math.min(BUBBLE_MAX_LIMIT, Math.max(1, props.maxBubbles + delta));
  if (next !== props.maxBubbles) emit("update:maxBubbles", next);
}
</script>

<template>
  <!-- 右上角设置按钮：开板保色（.open），与归档按钮同款几何表现 -->
  <button class="settingsButton" :class="{ open: isOpen }" aria-label="Settings" @click="toggle">
    <svg class="settings-btn" xmlns="http://www.w3.org/2000/svg" viewBox="0 -960 960 960">
      <path
        fill="#B5BAC1"
        d="m370-80-16-128q-13-5-24.5-12T307-235l-119 50L78-375l103-78q-1-7-1-13.5v-27q0-6.5 1-13.5L78-585l110-190 119 50q11-8 23-15t24-12l16-128h220l16 128q13 5 24.5 12t22.5 15l119-50 110 190-103 78q1 7 1 13.5v27q0 6.5-2 13.5l103 78-110 190-118-50q-11 8-23 15t-24 12L590-80H370Zm70-80h79l14-106q31-8 57.5-23.5T639-327l99 41 39-68-86-65q5-14 7-29.5t2-31.5q0-16-2-31.5t-7-29.5l86-65-39-68-99 42q-22-23-48.5-38.5T533-694l-13-106h-79l-14 106q-31 8-57.5 23.5T321-633l-99-41-39 68 86 64q-5 15-7 30t-2 32q0 16 2 31t7 30l-86 65 39 68 99-42q22 23 48.5 38.5T427-266l13 106Zm42-180q58 0 99-41t41-99q0-58-41-99t-99-41q-59 0-99.5 41T342-480q0 58 40.5 99t99.5 41Zm-2-140Z"
      ></path>
    </svg>
    <span class="tooltip">settings</span>
  </button>

  <div ref="overlay" class="settings-overlay" :class="{ open: isOpen }" @mousedown="onOverlayDown">
    <div class="board-glass">
      <p class="board-title">设置</p>
      <div class="setting-row inline">
        <div>
          <p class="setting-name">跟随系统</p>
          <p class="setting-desc">开启后主题随系统深浅自动切换</p>
        </div>
        <div class="setting-control">
          <label class="switch">
            <input type="checkbox" :checked="followChecked" @change="onFollowChange" />
            <span class="slider"></span>
          </label>
        </div>
      </div>
      <div class="setting-row inline">
        <div>
          <p class="setting-name">日夜切换</p>
          <p class="setting-desc">{{ daynightDesc }}</p>
        </div>
        <div class="setting-control">
          <label class="theme-switch" :class="{ 'is-disabled': daynightDisabled }">
            <input
              type="checkbox"
              class="theme-switch__checkbox"
              :checked="daynightChecked"
              :disabled="daynightDisabled"
              @change="onDaynightChange"
            />
            <div class="theme-switch__container">
              <div class="theme-switch__clouds"></div>
              <div class="theme-switch__stars-container">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 55" fill="none">
                  <path
                    fill-rule="evenodd"
                    clip-rule="evenodd"
                    d="M135.831 3.00688C135.055 3.85027 134.111 4.29946 133 4.35447C134.111 4.40947 135.055 4.85867 135.831 5.71123C136.607 6.55462 136.996 7.56303 136.996 8.72727C136.996 7.95722 137.172 7.25134 137.525 6.59129C137.886 5.93124 138.372 5.39954 138.98 5.00535C139.598 4.60199 140.268 4.39114 141 4.35447C139.88 4.2903 138.936 3.85027 138.16 3.00688C137.384 2.16348 136.996 1.16425 136.996 0C136.996 1.16425 136.607 2.16348 135.831 3.00688ZM31 23.3545C32.1114 23.2995 33.0551 22.8503 33.8313 22.0069C34.6075 21.1635 34.9956 20.1642 34.9956 19C34.9956 20.1642 35.3837 21.1635 36.1599 22.0069C36.9361 22.8503 37.8798 23.2903 39 23.3545C38.2679 23.3911 37.5976 23.602 36.9802 24.0053C36.3716 24.3995 35.8864 24.9312 35.5248 25.5913C35.172 26.2513 34.9956 26.9572 34.9956 27.7273C34.9956 26.563 34.6075 25.5546 33.8313 24.7112C33.0551 23.8587 32.1114 23.4095 31 23.3545ZM0 36.3545C1.11136 36.2995 2.05513 35.8503 2.83131 35.0069C3.6075 34.1635 3.99559 33.1642 3.99559 32C3.99559 33.1642 4.38368 34.1635 5.15987 35.0069C5.93605 35.8503 6.87982 36.2903 8 36.3545C7.26792 36.3911 6.59757 36.602 5.98015 37.0053C5.37155 37.3995 4.88644 37.9312 4.52481 38.5913C4.172 39.2513 3.99559 39.9572 3.99559 40.7273C3.99559 39.563 3.6075 38.5546 2.83131 37.7112C2.05513 36.8587 1.11136 36.4095 0 36.3545ZM56.8313 24.0069C56.0551 24.8503 55.1114 25.2995 54 25.3545C55.1114 25.4095 56.0551 25.8587 56.8313 26.7112C57.6075 27.5546 57.9956 28.563 57.9956 29.7273C57.9956 28.9572 58.172 28.2513 58.5248 27.5913C58.8864 26.9312 59.3716 26.3995 59.9802 26.0053C60.5976 25.602 61.2679 25.3911 62 25.3545C60.8798 25.2903 59.9361 24.8503 59.1599 24.0069C58.3837 23.1635 57.9956 22.1642 57.9956 21C57.9956 22.1642 57.6075 23.1635 56.8313 24.0069ZM81 25.3545C82.1114 25.2995 83.0551 24.8503 83.8313 24.0069C84.6075 23.1635 84.9956 22.1642 84.9956 21C84.9956 22.1642 85.3837 23.1635 86.1599 24.0069C86.9361 24.8503 87.8798 25.2903 89 25.3545C88.2679 25.3911 87.5976 25.602 86.9802 26.0053C86.3716 26.3995 85.8864 26.9312 85.5248 27.5913C85.172 28.2513 84.9956 28.9572 84.9956 29.7273C84.9956 28.563 84.6075 27.5546 83.8313 26.7112C83.0551 25.8587 82.1114 25.4095 81 25.3545ZM136 36.3545C137.111 36.2995 138.055 35.8503 138.831 35.0069C139.607 34.1635 139.996 33.1642 139.996 32C139.996 33.1642 140.384 34.1635 141.16 35.0069C141.936 35.8503 142.88 36.2903 144 36.3545C143.268 36.3911 142.598 36.602 141.98 37.0053C141.372 37.3995 140.886 37.9312 140.525 38.5913C140.172 39.2513 139.996 39.9572 139.996 40.7273C139.996 39.563 139.607 38.5546 138.831 37.7112C138.055 36.8587 137.111 36.4095 136 36.3545ZM101.831 49.0069C101.055 49.8503 100.111 50.2995 99 50.3545C100.111 50.4095 101.055 50.8587 101.831 51.7112C102.607 52.5546 102.996 53.563 102.996 54.7273C102.996 53.9572 103.172 53.2513 103.525 52.5913C103.886 51.9312 104.372 51.3995 104.98 51.0053C105.598 50.602 106.268 50.3911 107 50.3545C105.88 50.2903 104.936 49.8503 104.16 49.0069C103.384 48.1635 102.996 47.1642 102.996 46C102.996 47.1642 102.607 48.1635 101.831 49.0069Z"
                    fill="currentColor"
                  ></path>
                </svg>
              </div>
              <div class="theme-switch__circle-container">
                <div class="theme-switch__sun-moon-container">
                  <div class="theme-switch__moon">
                    <div class="theme-switch__spot"></div>
                    <div class="theme-switch__spot"></div>
                    <div class="theme-switch__spot"></div>
                  </div>
                </div>
              </div>
            </div>
          </label>
        </div>
      </div>
      <div class="setting-row inline">
        <div>
          <p class="setting-name">气泡提醒数量</p>
          <p class="setting-desc">气泡达到该数量时横幅提醒清理</p>
        </div>
        <div class="setting-control stepper">
          <button
            class="stepper-btn"
            aria-label="减少"
            :disabled="maxBubbles <= 1"
            @click="step(-1)"
          >
            −
          </button>
          <span class="stepper-value">{{ maxBubbles }}</span>
          <button
            class="stepper-btn"
            aria-label="增加"
            :disabled="maxBubbles >= BUBBLE_MAX_LIMIT"
            @click="step(1)"
          >
            +
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<!-- 设置按钮/浮板/行样式均已全局挂载（main.ts：archive.css 共享几何 + settings.css） -->
