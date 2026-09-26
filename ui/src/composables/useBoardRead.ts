// ===== 整板阅读组合式（PL009.2 自 design/assets/js/glass-bar.js makeBoardRead 1:1）：
// 上下溶解遮罩 + ▲▼ 边缘三角 + scrollend 收尾（半截行 --fade-btm 渐隐）+ 到底抬带
// （at-bottom 100ms）+ 隐区位移（maskShift）。算法与数值零改动（顶 6/底 14 软边、
// 三角 26px、padB 8、迟滞判定 ±2 全沿实测定案） =====
import { onUnmounted } from "vue";

/** 整板阅读实例（注册表条目：useVeils 帘联动按 el id 匹配） */
export interface BoardReadEntry {
  el: HTMLElement;
  hints: [HTMLElement, HTMLElement];
  sync: () => void;
  settle: () => void;
}

export interface BoardReadOptions {
  /** 底缘判定内距（默认 8，与 CSS scroll-padding-bottom 同源） */
  padB?: number;
  /** 半截行判定用行选择器（默认 .todo-item；气泡列表 .bubble-row） */
  rowSel?: string;
  /** 门控（详情板 textarea：仅打开时亮三角）——返回 false 三角全隐 */
  gate?: () => boolean;
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
 * scroll 滚动实时显隐/到底抬带、scrollend 半截行收尾、内容重建自动 settle
 * @param el 滚动容器
 * @param opts 见 BoardReadOptions
 * @returns { sync, settle, hints } 手动同步/收尾入口与三角对
 */
export function useBoardRead(
  el: HTMLElement,
  opts: BoardReadOptions = {},
): { sync: () => void; settle: () => void; hints: [HTMLElement, HTMLElement] } {
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
  const padB = opts.padB ?? 8;
  const rowSel = opts.rowSel ?? ".todo-item";

  /** 边缘三角几何与显隐：容器可滚动且未到对应尽头才显示；hintHost 场景走布局盒 */
  const sync = (): void => {
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

  /** 吸附收尾：半截行顶推给 --fade-btm 渐隐；到底分支抬带 100%；空容器补 sync 防残留 */
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
    const bottomEdge = sr.bottom - padB;
    let fadeStart: number | null = null;
    for (const li of el.querySelectorAll(rowSel)) {
      const r = li.getBoundingClientRect();
      if (r.top < bottomEdge && r.bottom > bottomEdge + 0.5) {
        fadeStart = r.top - sr.top - 2; // 半截行顶略上 2px，连间隙一起隐去
        break;
      }
    }
    if (fadeStart !== null) el.style.setProperty("--fade-btm", `${Math.round(fadeStart)}px`);
    else el.style.removeProperty("--fade-btm");
    sync();
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
    sync();
  };
  const onScrollEnd = (): void => settle();
  const observer = new MutationObserver(onScrollEnd);
  observer.observe(el, { childList: true });
  el.addEventListener("scroll", onScroll);
  el.addEventListener("scrollend", onScrollEnd);

  sync();
  const inst: BoardReadEntry = { el, hints, sync, settle };
  boardReads.push(inst);

  onUnmounted(() => {
    el.removeEventListener("scroll", onScroll);
    el.removeEventListener("scrollend", onScrollEnd);
    observer.disconnect();
    const idx = boardReads.indexOf(inst);
    if (idx >= 0) boardReads.splice(idx, 1);
    up.remove();
    down.remove();
  });

  return { sync, settle, hints };
}

/** 全局别名：换页监听等挂点调用，逐实例重算三角显隐与几何 */
export function syncHints(): void {
  for (const r of boardReads) r.sync();
}
