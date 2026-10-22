// ===== 玻璃滚动浮钮组合式（PL009.1 自 design/assets/js/glass-bar.js makeGlassBar 1:1）：
// 隐藏原生滑杆，4×16 透明玻璃浮钮随滚动比例移动、可拖拽。浮钮悬浮锚在宿主层（默认
// #board，不进滚动体内部——内贴会随内容滚走）；矩形每次同步按滚动体可视范围实测重算。
// 数值零改动（右侧偏移 7.75/4.5、浮钮 4×16、scrollable 判定 +1 全沿实测定案） =====
import { getCurrentInstance, onUnmounted } from "vue";

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

/** 全局滑杆重算（换页挂点调用，与 useBoardRead.syncHints 对称）：逐实例 sync——
 * 宿主隐藏（v-show 切走 rect 归零）时 sync 内建判定自动隐藏浮钮，切回按新几何恢复 */
export function syncGlassBars(): void {
  for (const g of glassBars) g.sync();
}

/**
 * 挂玻璃滑杆：滚动/输入/内容增删三挂点驱动 sync；浮钮可拖拽（textarea 程序赋值
 * scrollTop 不派发 scroll 的 Chromium 固有行为，拖拽路径补发合成事件）。
 * 在 nextTick/watch 等异步上下文调用时 Vue 生命周期钩子失效（无组件实例），此时
 * 须由调用方持有 destroy 并在卸载/重挂前手动调用
 * @param scroller 滚动容器
 * @param opts anchor/inset/right
 * @returns { sync, destroy } 手动同步入口与显式清理入口
 */
export function useGlassBar(
  scroller: HTMLElement,
  opts: GlassBarOptions = {},
): { sync: () => void; destroy: () => void } {
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
    // PATCH002.19：挂载 / 换页 / 开板收口等"内容已落定但还没滚过"那一拍也要复核罩死
    // （此前只在滚动/输入/子节点变更时跑 → 首屏与开板第一眼恒不生效）
    // FIX014.34：只复核**本滑杆所在列表**；详情/白板没有对应目标，自动 no-op。
    syncMaskDeadHook(scroller.id);
    const sr = scroller.getBoundingClientRect();
    const hr = anchor.getBoundingClientRect();
    if (!sr.height) {
      bar.hidden = true; // 所在页隐藏：无几何，先隐藏
      return;
    }
    // 轨道纵向：列表（inset）按上下内距内缩 = 行可视范围；textarea 走满全高。
    // 定位基准：绝对定位的包含块就是 anchor 的 **border box**（2026-10-18 实测坐标），
    // 故 `sr.top − hr.top` 可直接用、**不需要**再扣 anchor 的 padding
    const cs = getComputedStyle(scroller);
    const padT = opts.inset ? parseFloat(cs.paddingTop) || 0 : 0;
    const padB = opts.inset ? parseFloat(cs.paddingBottom) || 0 : 0;
    bar.style.top = `${sr.top - hr.top + padT}px`;
    bar.style.height = `${sr.height - padT - padB}px`;
    bar.style.right = `${opts.right ?? 7.75}px`;
    const scrollable = scroller.scrollHeight > scroller.clientHeight + 1;
    bar.hidden = !scrollable;
    if (!scrollable) return;
    const trackH = sr.height - padT - padB - 16; // 轨道 = 行可视高 − 浮钮高（常规）
    const range = scroller.scrollHeight - scroller.clientHeight;
    // 浮钮高度：常规 16px；**内容几乎不溢出**时按可视占比放大——否则 scrollTop 取整会让
    // 浮钮一次跳 trackH/range 像素（"一拖一大跳"）。门控 trackH/8：内容远长于视口的常规
    // 列表永远走 16px，观感与手感零变化；只有"刚好溢出几 px"的极端情况才比例化
    const thumbH =
      range < trackH / 8
        ? Math.min(
            trackH,
            Math.max(16, Math.round((trackH * scroller.clientHeight) / scroller.scrollHeight)),
          )
        : 16;
    thumb.style.height = `${thumbH}px`;
    const travel = Math.max(0, trackH + 16 - thumbH); // 行程 = 轨道高 − 浮钮高
    // ratio 夹取 [0,1]：scrollHeight/clientHeight 被浏览器取整为整数、scrollTop 是小数，
    // 小数几何的容器（归档板实测）到底时 scrollTop 会略大于整数 range → ratio 冲过 1 →
    // 浮钮被推出轨道底（实测越界 ~5px，且随小数浮动 2~5px）。清单/气泡 scrollTop ≤ range，
    // 夹取对它们零影响（ratio 本就 ≤ 1）
    const ratio = Math.min(1, Math.max(0, scroller.scrollTop / (range || 1)));
    thumb.style.top = `${Math.round(ratio * travel)}px`;
  };

  const onScroll = (): void => {
    sync(); // 内含罩死复核（PATCH002.19：收敛单点，不再单独调钩子）
  };
  scroller.addEventListener("scroll", onScroll);
  scroller.addEventListener("input", sync);
  const observer = new MutationObserver(onScroll);
  observer.observe(scroller, { childList: true });

  // 浮钮拖拽：按住上下平移即滚动（比例换算 1:1 design）
  thumb.addEventListener("pointerdown", (e: PointerEvent) => {
    const thumbH = thumb.getBoundingClientRect().height; // 比例化时浮钮更高 → 行程随之收窄
    const trackH = bar.getBoundingClientRect().height - thumbH;
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

  /** 显式清理（异步上下文挂载时 onUnmounted 拿不到组件实例，调用方手动调） */
  const destroy = (): void => {
    scroller.removeEventListener("scroll", onScroll);
    scroller.removeEventListener("input", sync);
    observer.disconnect();
    const idx = glassBars.indexOf(entry);
    if (idx >= 0) glassBars.splice(idx, 1);
    bar.remove();
  };

  // 仅当存在组件实例（setup 同步期调用）时注册自动清理；异步挂载由调用方管理
  if (getCurrentInstance()) {
    onUnmounted(destroy);
  }

  return { sync, destroy };
}

/** 罩死同步钩子（useMaskDead 注册；本文件先声明避免循环 import——design 同款松耦合） */
let maskDeadHook: ((listId: string) => void) | null = null;

/** 注册罩死同步钩子（由 useMaskDead 模块调用一次） */
export function registerMaskDeadHook(fn: (listId: string) => void): void {
  maskDeadHook = fn;
}

function syncMaskDeadHook(listId: string): void {
  maskDeadHook?.(listId);
}
