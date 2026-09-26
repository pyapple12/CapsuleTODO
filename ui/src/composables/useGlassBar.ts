// ===== 玻璃滚动浮钮组合式（PL009.1 自 design/assets/js/glass-bar.js makeGlassBar 1:1）：
// 隐藏原生滑杆，4×16 透明玻璃浮钮随滚动比例移动、可拖拽。浮钮悬浮锚在宿主层（默认
// #board，不进滚动体内部——内贴会随内容滚走）；矩形每次同步按滚动体可视范围实测重算。
// 数值零改动（右侧偏移 7.75/4.5、浮钮 4×16、scrollable 判定 +1 全沿实测定案） =====
import { onUnmounted } from "vue";

/** 滑杆实例（注册表条目：useVeils 帘联动按 scroller id 匹配） */
export interface GlassBarEntry {
  scroller: HTMLElement;
  bar: HTMLElement;
  sync: () => void;
}

export interface GlassBarOptions {
  /** 浮钮锚层（默认 #board）；归档板传自家玻璃卡 */
  anchor?: HTMLElement;
  /** 轨道按滚动体上下内距内缩（列表类容器） */
  inset?: boolean;
  /** 距宿主右缘像素（默认 7.75；白板 7.75/归档 4.5） */
  right?: number;
}

/** 全局滑杆注册表（useVeils 遍历挂帘） */
export const glassBars: GlassBarEntry[] = [];

/**
 * 挂玻璃滑杆：滚动/输入/内容增删三挂点驱动 sync；浮钮可拖拽（textarea 程序赋值
 * scrollTop 不派发 scroll 的 Chromium 固有行为，拖拽路径补发合成事件）
 * @param scroller 滚动容器
 * @param opts anchor/inset/right
 * @returns { sync } 手动同步入口（内容重建后调用）
 */
export function useGlassBar(
  scroller: HTMLElement,
  opts: GlassBarOptions = {},
): { sync: () => void } {
  const anchor = opts.anchor ?? (document.getElementById("board") as HTMLElement);
  scroller.classList.add("glass-scroll");
  const bar = document.createElement("div");
  bar.className = "glass-bar";
  bar.hidden = true;
  const thumb = document.createElement("i");
  thumb.className = "glass-thumb";
  bar.appendChild(thumb);
  anchor.appendChild(bar);
  bar.dataset.for = scroller.id || scroller.className;

  const sync = (): void => {
    const sr = scroller.getBoundingClientRect();
    const hr = anchor.getBoundingClientRect();
    if (!sr.height) {
      bar.hidden = true; // 所在页隐藏：无几何，先隐藏
      return;
    }
    // 轨道纵向：列表（inset）按上下内距内缩 = 行可视范围；textarea 走满全高
    const cs = getComputedStyle(scroller);
    const padT = opts.inset ? parseFloat(cs.paddingTop) || 0 : 0;
    const padB = opts.inset ? parseFloat(cs.paddingBottom) || 0 : 0;
    bar.style.top = `${sr.top - hr.top + padT}px`;
    bar.style.height = `${sr.height - padT - padB}px`;
    bar.style.right = `${opts.right ?? 7.75}px`;
    const scrollable = scroller.scrollHeight > scroller.clientHeight + 1;
    bar.hidden = !scrollable;
    if (!scrollable) return;
    const trackH = sr.height - padT - padB - 16; // 轨道 = 行可视高 − 浮钮高
    const ratio = scroller.scrollTop / (scroller.scrollHeight - scroller.clientHeight || 1);
    thumb.style.top = `${Math.round(ratio * trackH)}px`;
  };

  const onScroll = (): void => {
    sync();
    syncMaskDeadHook();
  };
  scroller.addEventListener("scroll", onScroll);
  scroller.addEventListener("input", sync);
  const observer = new MutationObserver(onScroll);
  observer.observe(scroller, { childList: true });

  // 浮钮拖拽：按住上下平移即滚动（比例换算 1:1 design）
  thumb.addEventListener("pointerdown", (e: PointerEvent) => {
    const trackH = bar.getBoundingClientRect().height - 16;
    if (trackH <= 0) return;
    const startY = e.clientY;
    const startScroll = scroller.scrollTop;
    const range = scroller.scrollHeight - scroller.clientHeight;
    const move = (ev: PointerEvent): void => {
      scroller.scrollTop = startScroll + ((ev.clientY - startY) / trackH) * range;
      // textarea 程序赋值不派发 scroll（Chromium 固有），补发合成事件驱动同步
      scroller.dispatchEvent(new Event("scroll"));
    };
    const up = (): void => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    e.preventDefault();
  });

  sync();
  const entry: GlassBarEntry = { scroller, bar, sync };
  glassBars.push(entry);

  // Vue 生命周期卸载：清监听、断观测、摘浮钮（design 无此环节——页面即生命周期）
  onUnmounted(() => {
    scroller.removeEventListener("scroll", onScroll);
    scroller.removeEventListener("input", sync);
    observer.disconnect();
    const idx = glassBars.indexOf(entry);
    if (idx >= 0) glassBars.splice(idx, 1);
    bar.remove();
  });

  return { sync };
}

/** 罩死同步钩子（useMaskDead 注册；本文件先声明避免循环 import——design 同款松耦合） */
let maskDeadHook: (() => void) | null = null;

/** 注册罩死同步钩子（由 useMaskDead 模块调用一次） */
export function registerMaskDeadHook(fn: () => void): void {
  maskDeadHook = fn;
}

function syncMaskDeadHook(): void {
  maskDeadHook?.();
}
