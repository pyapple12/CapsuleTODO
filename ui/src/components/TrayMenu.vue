<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PrefsView } from "../../types";

// ===== 托盘菜单（PL021）：独立 tray-menu 窗的根组件——自绘菜单替代原生
// Win32 菜单（深色模式下 Win11 圆角边距被 muda 涂实 = 上下空条）。四项动作
// invoke tray_menu_action 统一分发（Rust 收窗 + 执行）；勾选态事实源 =
// settings_get 命令 + prefs-changed 广播回显

/** 菜单项动作（与 Rust tray_menu_action 分发全集一致） */
type MenuAction = "show" | "snap" | "top" | "exit";

/** 贴边吸附勾选态（事实源 = config.json，经命令读出） */
const snapOn = ref(true);
/** 窗口置顶勾选态 */
const topOn = ref(true);
let unlistenPrefs: UnlistenFn | undefined;

/** 点菜单项：动作分发（收窗在 Rust 侧统一执行） */
async function onAction(action: MenuAction): Promise<void> {
  try {
    await invoke("tray_menu_action", { action });
  } catch (err) {
    console.error("菜单动作执行失败", err);
  }
}

/** 拉取两开关当前值（打开菜单瞬间的事实源快照） */
async function refresh(): Promise<void> {
  try {
    snapOn.value = await invoke<boolean>("settings_get_snap_to_edge");
    topOn.value = await invoke<boolean>("settings_get_always_on_top");
  } catch (err) {
    console.error("菜单勾选态拉取失败", err);
  }
}

onMounted(async () => {
  // 高度链修正（预览窗同款）：html/body/#app 无显式高度时 height:100% 塌缩，
  // 露出 webview 原生暗背景——菜单窗实例 JS 直改（只影响本窗）
  document.documentElement.style.height = "100%";
  document.documentElement.style.background = "transparent";
  document.body.style.height = "100%";
  document.body.style.background = "transparent";
  const root = document.getElementById("app");
  if (root) {
    root.style.height = "100%";
    root.style.background = "transparent";
  }
  await refresh();
  // 开关事实源变化（任意入口切换）→ 勾选态回显（FIX007.6 补 catch：注册失败落
  // 日志不中断，常驻窗无自愈机会）
  unlistenPrefs = await listen<PrefsView>("prefs-changed", (event) => {
    snapOn.value = event.payload.snap_to_edge;
    topOn.value = event.payload.always_on_top;
  }).catch((err) => {
    console.error("prefs-changed 监听注册失败", err);
    return undefined;
  });
});

onUnmounted(() => {
  unlistenPrefs?.();
});
</script>

<template>
  <div class="tray-menu">
    <button class="tm-item" type="button" @click="onAction('show')">
      <span class="tm-check" aria-hidden="true"></span>
      <span class="tm-text">聚焦主窗</span>
    </button>
    <button class="tm-item" type="button" @click="onAction('snap')">
      <span class="tm-check" aria-hidden="true">
        <svg v-if="snapOn" viewBox="0 0 12 12"><polyline points="2.5,6.5 5,9 9.5,3" /></svg>
      </span>
      <span class="tm-text">贴边吸附</span>
    </button>
    <button class="tm-item" type="button" @click="onAction('top')">
      <span class="tm-check" aria-hidden="true">
        <svg v-if="topOn" viewBox="0 0 12 12"><polyline points="2.5,6.5 5,9 9.5,3" /></svg>
      </span>
      <span class="tm-text">窗口置顶</span>
    </button>
    <button class="tm-item" type="button" @click="onAction('exit')">
      <span class="tm-check" aria-hidden="true"></span>
      <span class="tm-text">退出</span>
    </button>
  </div>
</template>

<style scoped>
/* 深底圆角菜单（预览窗同款视觉语言）：纯深底 + 细描边 + DWM 系统圆角，
紧凑行高（146px 窗高 = 4 × 34 行 + 上下 5px 内距） */
.tray-menu {
  box-sizing: border-box;
  width: 100%;
  height: 100%;
  padding: 5px 0;
  border-radius: 8px;
  background: rgba(24, 26, 34, 0.96);
  border: 1px solid rgba(255, 255, 255, 0.18);
  color: #fff;
  font-size: 12px;
  user-select: none;
  display: flex;
  flex-direction: column;
}

/* 菜单项：勾选列定宽对齐（原生菜单同款列结构），UA 按钮内距清零后重设 */
.tm-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  height: 34px;
  padding: 0 12px 0 10px;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.tm-item:hover {
  background: rgba(255, 255, 255, 0.08);
}

.tm-check {
  flex: none;
  width: 14px;
  height: 14px;
  display: grid;
  place-items: center;
}

.tm-check svg {
  width: 11px;
  height: 11px;
}

.tm-check polyline {
  fill: none;
  stroke: #fff;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.tm-text {
  flex: 1;
  white-space: nowrap;
}
</style>
