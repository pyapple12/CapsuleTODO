// ===== 整板阅读组合式（PL009.2 自 design/assets/js/glass-bar.js makeBoardRead 1:1）：
// 上下溶解遮罩 + ▲▼ 边缘三角 + scrollend 收尾（**两态**：到底抬带 100% / 其余默认软边；
// PATCH002.50 起删掉"半截行整行溶解"）+ 隐区位移（maskShift）。数值沿实测定案（上下软边 14px、
// 三角 26px、迟滞判定 ±2）；顶部软边 PATCH002.12 参数化为 --fade-top，
// 贴顶（scrollTop ≤ 2）收到 0px、滚过阈值回落 14px（与 at-bottom 同规） =====
import { getCurrentInstance, onUnmounted } from "vue";

/** 整板阅读实例（注册表条目：useVeils 帘联动按 el id 匹配） */
export interface BoardReadEntry {
  el: HTMLElement;
  hints: [HTMLElement, HTMLElement];
  sync: () => void;
  settle: () => void;
  /** PATCH002.69：由调用方**直接声明**"收场前是否贴底"（拖拽/结构变更路径已知，比滚动事件可靠） */
  setPinned: (value: boolean) => void;
}

export interface BoardReadOptions {
  /** 门控（详情板 textarea：仅打开时亮三角）——返回 false 三角全隐 */
  gate?: () => boolean;
  /** 临时诊断出口（PATCH002.69 排查；不传即无副作用） */
  onDiag?: (line: string) => void;
  /** 走 offsetParent 布局盒（揭示缩放动画宿主内的容器：详情板/归档板） */
  layout?: boolean;
  /** 三角宿主（默认 #board；归档板传玻璃板——方案 B 三角随板缩放生长） */
  hintHost?: HTMLElement;
  /** 拖拽收尾互斥（清单/气泡：让位 transform 中重算会取错几何） */
  skipDuringDrag?: boolean;
  /** 隐区位移（气泡满仓警告：阈值 8.5 / 深度 6，用户定案） */
  maskShift?: { threshold: number; depth: number };
}

/** 全局注册表（useVeils 遍历挂帘；syncHints 逐实例重算） */
export const boardReads: BoardReadEntry[] = [];

/** 拖拽互斥判定注入（useDragReorder PL013 注册；design 为全局 dragCtx 直读） */
let dragEngagedProbe: () => boolean = () => false;

/** 注册拖拽在飞判定（PL013 useDragReorder 调用） */
export function registerDragProbe(fn: () => boolean): void {
  dragEngagedProbe = fn;
}

/**
 * 挂整板阅读：容器加 .board-read 类（溶解遮罩），创建 ▲▼ 三角对挂宿主层，
 * scroll 滚动实时显隐/到底抬带、scrollend 半截行收尾、内容重建自动 settle。
 * 在 nextTick/watch 等异步上下文调用时生命周期钩子失效（无组件实例），须由
 * 调用方持有 destroy 并在卸载/重挂前手动调用
 * @param el 滚动容器
 * @param opts 见 BoardReadOptions
 * @returns `BoardReadEntry`（sync / settle / hints / destroy / setPinned）
 */
export function useBoardRead(
  el: HTMLElement,
  opts: BoardReadOptions = {},
): {
  sync: () => void;
  settle: () => void;
  hints: [HTMLElement, HTMLElement];
  destroy: () => void;
  setPinned: (value: boolean) => void;
} {
  el.classList.add("board-read");
  const hintHost = opts.hintHost ?? (document.getElementById("board") as HTMLElement);
  const up = document.createElement("div");
  up.className = "edge-hint up";
  up.textContent = "▲";
  const down = document.createElement("div");
  down.className = "edge-hint down";
  down.textContent = "▼";
  hintHost.append(up, down);
  const hints: [HTMLElement, HTMLElement] = [up, down];

  /** 边缘三角几何与显隐：容器可滚动且未到对应尽头才显示；hintHost 场景走布局盒 */
  const sync = (): void => {
    // 顶部软边对称分支（PATCH002.12，与底部 at-bottom 同规）：贴顶时上方没有被滚出的
    // 内容，软边收到 0px（首行距顶 8px 本就不该淡）；滚过阈值回落到默认 14px
    if (el.scrollTop <= 2) {
      el.classList.add("at-top");
      el.style.setProperty("--fade-top", "0px");
    } else {
      el.classList.remove("at-top");
      el.style.removeProperty("--fade-top");
    }
    let left: number;
    let top: number;
    let width: number;
    let bottom: number;
    if (opts.layout || opts.hintHost) {
      // offsetParent 链累加到宿主 = 宿主局部坐标（揭示缩放动画不污染几何）
      let node: HTMLElement | null = el;
      let x = 0;
      let y = 0;
      while (node && node !== hintHost) {
        x += node.offsetLeft;
        y += node.offsetTop;
        node = node.offsetParent as HTMLElement | null;
      }
      left = x;
      top = y;
      width = el.offsetWidth;
      bottom = y + el.offsetHeight;
    } else {
      const sr = el.getBoundingClientRect();
      const hr = hintHost.getBoundingClientRect();
      left = sr.left - hr.left;
      top = sr.top - hr.top;
      width = sr.width;
      bottom = sr.bottom - hr.top;
    }
    const scrollable = el.scrollHeight > el.clientHeight + 1;
    const active = !opts.gate || opts.gate();
    const showUp = active && scrollable && el.scrollTop > 2;
    const showDown = active && scrollable && el.scrollTop < el.scrollHeight - el.clientHeight - 2;
    up.hidden = !showUp;
    down.hidden = !showDown;
    for (const hint of hints) {
      hint.style.left = `${left}px`;
      hint.style.width = `${width}px`;
    }
    up.style.top = `${top}px`;
    down.style.top = `${bottom - 26}px`;
  };

  /** 收尾：**只保留两态**（PATCH002.50 定案）——到底分支抬带 100%；其余回落默认软边。
   * 原"把 `--fade-btm` 推到跨底半截行行顶"的整行溶解已删除（观感差异过大，
   * 五容器统一为"上下软边"）；空容器补 sync 防三角残留 */
  const settle = (): void => {
    if (opts.skipDuringDrag && dragEngagedProbe()) return;
    const sr = el.getBoundingClientRect();
    if (!sr.height) {
      sync(); // 列表转空被隐藏时也要走显隐判定：三角保持清空前的状态会残留
      return;
    }
    // 到底分支：底部已是内容尽头，遮罩失去意义——带抬到 100% 全显
    if (el.scrollTop >= el.scrollHeight - el.clientHeight - 1) {
      el.classList.add("at-bottom");
      el.style.setProperty("--fade-btm", "100%");
      sync();
      return;
    }
    el.classList.remove("at-bottom");
    el.style.removeProperty("--fade-btm"); // 回落 `.board-read` 默认 14px 软边（与详情板/白板同口径）
    sync();
  };

  /** PATCH002.69：**贴底标志** —— 只在 `scroll` 时更新（内容增减不改 `scrollTop` 时不失真）。
   * 用它判"原本是否贴底"，不再用高度差值（高度在同一批 DOM 变更里"先矮后长"会判错 ✗） */
  let pinned = false;
  /** 一次性覆盖（PATCH002.69）：`setPinned` 后到"增长结算"之间，滚动事件**不得**改写贴底态
   * （实测：拖拽收场"先矮后长"时，夹取产生的 scroll 事件会把刚声明的 true 翻回 false ✗） */
  let pinOverride = false;
  let pinOverrideTimer: number | undefined;
  const syncPinned = (): void => {
    if (pinOverride) return; // 调用方已显式声明 ⇒ 本批变更加完前不接受滚动事件改写
    const next = el.scrollTop + el.clientHeight >= el.scrollHeight - 1;
    if (next !== pinned) {
      opts.onDiag?.(
        `pin:flip ${pinned ? 1 : 0}->${next ? 1 : 0} st=${el.scrollTop.toFixed(1)} sh=${el.scrollHeight} ch=${el.clientHeight}`,
      );
    }
    pinned = next;
  };

  const onScroll = (): void => {
    // 到底判定实时化：滚动途中贴底就地抬带；未到底清值回落默认带
    const dragging = opts.skipDuringDrag && dragEngagedProbe();
    if (el.scrollTop >= el.scrollHeight - el.clientHeight - 1 && !dragging) {
      el.classList.add("at-bottom");
      el.style.setProperty("--fade-btm", "100%");
    } else {
      el.classList.remove("at-bottom");
      if (!dragging) el.style.removeProperty("--fade-btm");
    }
    // 隐区位移：警告行在场且滚过阈值时渐显带下移，滚回复位
    if (opts.maskShift) {
      const shifted =
        el.classList.contains("has-warning") && el.scrollTop >= opts.maskShift.threshold;
      el.style.setProperty("--mask-shift", shifted ? `${opts.maskShift.depth}px` : "0px");
    }
    syncPinned(); // 滚动即刷新贴底标志（PATCH002.69）
    sync();
  };
  const onScrollEnd = (): void => settle();
  /**
   * 内容增长时**贴底补偿**（PATCH002.69）：容器自身尺寸不变，必须观察**子元素**。
   * 五容器的行/块都直接挂在滚动容器上（**没有内容层 wrapper**）⇒ 观察全部元素子节点，
   * 并在 `MutationObserver(childList)` 里**重绑**（行增删后新行也要被观察）。
   * `pinned` = **滚动时**维护的贴底标志（内容增益不改 `scrollTop` ⇒ 不能用高度差判，实测会判错 ✗）。
   * RO 回调在 **paint 之前**执行 ⇒ 贴底补偿**零可见帧**（治"拉回旧底再弹回"）。
   */
  const ro = new ResizeObserver(() => {
    const sh = el.scrollHeight;
    if (!el.clientHeight || !sh) return;
    // 只在**真的要补**时打点（否则每次尺寸变化都记一条，日志噪声大）
    if (pinned && el.scrollTop + el.clientHeight < sh - 1) {
      opts.onDiag?.(`ro:fix st=${el.scrollTop.toFixed(1)} sh=${sh} ch=${el.clientHeight}`);
      el.scrollTop = sh;
    }
    pinOverride = false; // 本批增长已结算 ⇒ 交回滚动判定
  });
  /** 重绑观察目标（**增量**：只 observe 新增、unobserve 移除——整体 disconnect 会丢同帧回调 ✗） */
  const observed = new Set<Element>();
  const observeRows = (): void => {
    const now = new Set<Element>(Array.from(el.children));
    for (const child of observed) {
      if (!now.has(child)) {
        ro.unobserve(child);
        observed.delete(child);
      }
    }
    for (const child of now) {
      if (!observed.has(child)) {
        ro.observe(child);
        observed.add(child);
      }
    }
  };
  observeRows();
  const observer = new MutationObserver(() => {
    observeRows();
    settle();
  });
  observer.observe(el, { childList: true });
  el.addEventListener("scroll", onScroll);
  el.addEventListener("scrollend", onScrollEnd);

  syncPinned();
  sync();
  /** PATCH002.69：供调用方**显式声明**"此刻是否贴底"——拖拽/结构变更路径（`.57` / `.44`）
   * 在内容变化前就知道答案，比滚动事件可靠（内容"先矮后长"时滚动时序会判错 ✗） */
  const setPinned = (value: boolean): void => {
    opts.onDiag?.(
      `pin:set ${value ? 1 : 0} (was ${pinned ? 1 : 0}) st=${el.scrollTop.toFixed(1)} sh=${el.scrollHeight}`,
    );
    pinned = value;
    pinOverride = true;
    if (pinOverrideTimer) window.clearTimeout(pinOverrideTimer);
    // 安全阀：增长一直没来（无 RO 命中）时最多挡 300ms，之后交回滚动判定
    pinOverrideTimer = window.setTimeout(() => {
      pinOverride = false;
    }, 300);
  };

  const inst: BoardReadEntry = { el, hints, sync, settle, setPinned };
  boardReads.push(inst);

  /** 显式清理（异步上下文挂载时生命周期钩子失效，调用方手动调） */
  const destroy = (): void => {
    el.removeEventListener("scroll", onScroll);
    el.removeEventListener("scrollend", onScrollEnd);
    if (pinOverrideTimer) window.clearTimeout(pinOverrideTimer);
    ro.disconnect();
    observer.disconnect();
    const idx = boardReads.indexOf(inst);
    if (idx >= 0) boardReads.splice(idx, 1);
    up.remove();
    down.remove();
  };

  // 仅当存在组件实例（setup 同步期调用）时注册自动清理；异步挂载由调用方管理
  if (getCurrentInstance()) {
    onUnmounted(destroy);
  }

  return { sync, settle, hints, destroy, setPinned };
}

/** 全局别名：换页监听等挂点调用，逐实例重算三角显隐与几何 */
export function syncHints(): void {
  for (const r of boardReads) r.sync();
}
