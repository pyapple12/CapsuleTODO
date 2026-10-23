// ===== 行内拖拽排序组合式（PL013.3 自 design/assets/js/drag-reorder.js 1:1）：
// 生产级重挂模型——长按 0.25s 进入（等待期移动超 6px 取消手势），起拖三步：等高
// 占位符留槽 → 行本体重挂卡片层随指针浮起 → 其余行平移让位、占位符滑向目标槽；
// 松手一次性提交（invoke reorder 落库）+ 收场重拉。边缘自动滚动（32px 带 10px/帧、
// moved 3px 门槛）与 blur 收尾同款。算法与数值零改动（六轮失败教训——红线）。
// design 差异映射：全局 dragCtx → 模块级闭包（组合式无全局作用域）；commit/rerender
// 由 DRAG_TARGETS 回调注入（Vue 侧走 invoke + 父级重拉）；Vue 冻结防线 = 注册表
// freezeProbe（engaged 期间列表 watch 早退，落点收场统一重拉）=====
import { onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { rowMaskDead } from "./useMaskDead";
import { HEIGHT_SWAP_K, noteSwapRule } from "./noteDragPolicy";
import { registerDragProbe } from "./useBoardRead";

/** 长按等待期允许的抖动幅度：超出即取消本次手势 */
const DRAG_THRESHOLD = 6;
/** 长按时长（用户定案：0.5s 缩减一半） */
const HOLD_MS = 250;
/** 与 .group 的 gap 对应：一个槽位 = 被拖行高 + 间隙 */
const ROW_GAP = 6;
/** 自动滚动边缘带宽度（用户定案按推荐值） */
/** 边缘自动滚动触发带（px）；target 可用 `autoScrollZone` 覆盖（PATCH002.79：详情块 12） */
const AUTO_SCROLL_ZONE = 32;

/** 被拖行中心钳制在卡片内时上下留的边距（px）——PATCH002.83：只在本模块用，策略读 ctx 边界 */
const DRAG_EDGE_MARGIN = 12;
/** 自动滚动全速：每帧像素数（约 600px/s） */
const AUTO_SCROLL_SPEED = 10;

/** FIX014.33：三个列表的真实离场类（`.leaving` 只用于归档删除按钮等局部元素）。
 * TransitionGroup 的 `name="todo"` 产出 `.todo-leave-active`，详情块用
 * `.note-block-leaving`；漏判会让塌缩中的行进入长按起拖。 */
function isLeavingRow(el: Element): boolean {
  return el.matches(".leaving, .todo-leave-active, .note-block-leaving");
}

/** FIX015.32：详情图块可拖区域——必须命中 `.note-block-image`，且不得命中删除按钮。
 * 抽成纯判定供 note `onDown` 与回归测试共用。 */
export function isNoteDragTarget(target: HTMLElement): boolean {
  if (target.closest(".note-block-del")) return false;
  return !!target.closest(".note-block-image");
}

/** 拖拽上下文（重挂模型的在飞状态体） */
export interface DragCtx {
  t: DragTarget;
  li: HTMLElement;
  row: HTMLElement;
  id: number;
  startX: number;
  startY: number;
  engaged: boolean;
  timer: number;
  ghost: HTMLElement;
  cardEl: HTMLElement;
  cardRect: DOMRect;
  selfTop: number;
  selfMid: number;
  selfHeight: number;
  listEl: HTMLElement;
  rows: { el: HTMLElement; top: number; mid: number; height: number }[];
  fromIdx: number;
  toIdx: number;
  scrollDelta: number;
  /** 落点提交的目标 id 序（PATCH002.89：数据回来必须命中此序才撤 ghost / 重建行） */
  expectedIds?: number[];
  /** 被拖块中心的可达范围（eff 空间：已含 scrollDelta；PATCH002.83 策略与引擎共用） */
  centerMin: number;
  centerMax: number;
  center: number;
  /** 真实指针 y（PATCH002.54：未钳制；`autoScrollProbe === "pointer"` 时自动滚动用它） */
  pointerY: number;
  moved: boolean;
  padTop: number;
  padBottom: number;
  raf: number;
}

/**
 * 目标行中点的可达钳制（PATCH002.87）：下拖时目标行若在 `centerMax` 之外（矮末行），
 * 先把中点钳到 `centerMax`，拖到底才能触发最后一行；上拖不钳中点，由顶部
 * `eff <= centerMin` 的“到顶即到首”覆盖。`ctx` 边界已含 `scrollDelta`（`.83` 口径）。
 * @param row 目标行快照（只读 `mid`）
 * @param ctx 拖拽上下文
 * @param downward 当前是否向下拖
 * @returns 可达空间中的目标中点
 */
export function reachableTargetMid(row: { mid: number }, ctx: DragCtx, downward: boolean): number {
  return downward ? Math.min(row.mid, ctx.centerMax) : row.mid;
}

/**
 * 清单 / 气泡默认换位规则（PATCH002.87）：中心对中心 + 高度差阈值 + 末行可达钳制。
 * - 等高行 `d=0`：逐帧退回原 `r.mid < eff`，手感不变；
 * - 高行对矮行：下拖用 `eff + d`（更早触发）、上拖用 `eff - d`（多拖 d 才越），
 *   与 `.80` 的方向语义一致；
 * - 顶部 `eff <= centerMin` 直接归 0（到顶即到首）；
 * - 下拖目标中点经 `reachableTargetMid` 钳到 `centerMax`（末行可达）；上拖不钳，顶部 `atTop` 归 0。
 * @param ctx 拖拽上下文
 * @param eff 被拖行中心的等效位置（已含 `scrollDelta`）
 * @returns 目标槽位号
 */
export function defaultSwapRule(ctx: DragCtx, eff: number): number {
  if (eff <= ctx.centerMin + 0.5) return 0;
  const downward = eff > ctx.selfMid;
  let to = 0;
  ctx.rows.forEach((row, index) => {
    if (index === ctx.fromIdx) return;
    const targetMid = reachableTargetMid(row, ctx, downward);
    const d = HEIGHT_SWAP_K * Math.max(0, ctx.selfHeight - row.height);
    const threshold = downward ? eff + d : eff - d;
    // 等高末行 d=0：中心恰好等于 centerMax 时用边界条件补一次，避免 `<` 吃掉最后一行
    const atBottomBoundary = downward && targetMid === ctx.centerMax && eff >= ctx.centerMax;
    if (targetMid < threshold || atBottomBoundary) to += 1;
  });
  return to;
}

/** 拖拽目标注册项（差异点全在此——机制本体零差异） */
export interface DragTarget {
  /** 目标名（清单 todos / 气泡 bubbles / 详情块 note——对应 invoke 命令前缀） */
  name: "todos" | "bubbles" | "note";
  /** 列表容器 id（scopeId：todo-active / bubble-list） */
  scopeId: string;
  /** 行选择器（命中与快照用） */
  rowSel: string;
  /** 外壳选择器（li；气泡行身兼外壳） */
  itemSel: string;
  /** 占位符类（todo-item todo-ghost / bubble-row bubble-ghost） */
  ghostClass: string;
  /** 按下钩子：取消各自待办定时器 + 排除自有语义控件；返回 false 不起拖 */
  onDown: (row: HTMLElement, e: MouseEvent) => boolean;
  /** 真正长按进入拖拽时调用（FIX014.34：详情图块的落库从 mousedown 移到这里） */
  onEngage?: (ctx: DragCtx) => void;
  /** 落点提交：toIdx = 目标槽位（invoke reorder 落库）；可返回 Promise 供收场等待 */
  commit: (ctx: DragCtx) => Promise<unknown> | void;
  /** 收场重绘（无论是否换位都重拉——行本体以（可能新）顺序收回列表） */
  rerender: (ctx?: DragCtx) => void;
  /** 保留占位符到收场重拉完成（PATCH002.82/89：清单/气泡/详情块共用——保高度、同任务交接） */
  keepGhost?: boolean;
  /** 自动滚动基准（PATCH002.54，可选）：缺省 `"center"` = 现行为（被钳制的行中心）；
   * `"pointer"` = 未钳制的真实指针 y（详情块用——容器内距与卡片内距不同，行中心够不到下边缘带） */
  autoScrollProbe?: "center" | "pointer";
  /** 边缘自动滚动触发带（px；缺省 32）——PATCH002.79：详情板矮，note 用 12 */
  autoScrollZone?: number;
  /** 换位判定（PATCH002.55，可选）：缺省 = 现"中心对中心"公式；详情块注入"按方向取缘"
   * （见 `noteDragPolicy.ts`）。等高行时两者数学等价 → 清单 / 气泡不受影响 */
  swapRule?: (ctx: DragCtx, eff: number) => number;
}

/** 组件层注入钩子（清单掐 detailOpenTimer / 气泡掐 bubbleCopyTimer；suppress 写回；
 * rerender 由收场重拉转发 hookBag） */
export interface DragHooks {
  /** 新按下取代上次点击的待办动作（清单开详情 / 气泡开板同款竞态） */
  cancelPendingClick: () => void;
  /** 拖拽结束后 350ms 内的点击不当作行点击（组件读 suppressUntil） */
  markSuppress: () => void;
  /** 收场重拉转发（hookBag 透传——todo/bubbles 两路；ctx 携带 ghost / expectedIds） */
  rerenderTodos?: (ctx?: DragCtx) => void;
  rerenderBubbles?: (ctx?: DragCtx) => void;
}

// —— 拖拽目标注册表（清单 + 气泡，V0.027 推广气泡；归档板行不注册——归档不排序）——
// onDown/commit/rerender 由组件层经 installDragTargets 注入（数据源在组件内）

const detailTimers: DragHooks = {
  cancelPendingClick: () => {},
  markSuppress: () => {},
};

export const DRAG_TARGETS: DragTarget[] = [
  {
    name: "todos",
    scopeId: "todo-active",
    rowSel: ".todo-row",
    itemSel: ".todo-item",
    ghostClass: "todo-item todo-ghost",
    // PATCH002.89：ghost 留到收场数据就位（详情板 .82 同款），不再换整个 <ul>
    keepGhost: true,
    // 按下钩子：掐掉待开详情定时器；删除钮/行内编辑框/勾选框区域归各自语义，不起拖
    onDown(row, e) {
      detailTimers.cancelPendingClick();
      if ((e.target as HTMLElement).closest(".del") || (e.target as HTMLElement).closest(".t-edit"))
        return false;
      const cb = row.querySelector(".neon-checkbox")?.getBoundingClientRect();
      if (!cb) return false;
      const inBox =
        e.clientX >= cb.left &&
        e.clientX <= cb.right &&
        e.clientY >= cb.top &&
        e.clientY <= cb.bottom;
      if (inBox) return false;
      return true;
    },
    // 落点提交：仅活动项参与排序（done 项不进清单，保持相对次序不变）。
    // 行序从 DOM 快照读——被拖行已重挂 #board 不在清单内（design 的 commit 读数据数组
    // 无此问题），先移除残余引用再把 ctx.id 插到 toIdx 槽位
    commit(ctx) {
      const ul = document.getElementById("todo-active");
      if (!ul) return;
      const ids = [...ul.querySelectorAll(".todo-row")]
        .map((r) => Number((r as HTMLElement).dataset.rowId))
        .filter((id) => Number.isInteger(id) && id !== ctx.id);
      ids.splice(ctx.toIdx, 0, ctx.id);
      ctx.expectedIds = ids;
      // PATCH002.89：返回 Promise——收场等落库完成再重拉，杜绝“重拉抢读旧序”
      return invoke("todo_reorder", { ids }).catch((err) => {
        console.error("重排失败", err);
        ctx.expectedIds = undefined; // 失败时库序未变：不设顺序门槛，交给兜底重拉
      });
    },
    rerender(ctx) {
      // 收场重拉走 hookBag（installDragHooks 注册）——detailTimers 只承载
      // cancelPendingClick/markSuppress 两钩子（DragHooks 接口无 rerender 字段）
      hookBag.rerenderTodos?.(ctx);
    },
  },
  {
    name: "bubbles",
    scopeId: "bubble-list",
    rowSel: ".bubble-row",
    itemSel: ".bubble-row", // 气泡行身兼外壳（无内层行容器）
    ghostClass: "bubble-row bubble-ghost", // 气泡行自带玻璃底——ghost 剥视觉见 bubbles.css
    // PATCH002.89：ghost 留到收场数据就位；ghost 自身也匹配 .bubble-row，commit 须滤 NaN
    keepGhost: true,
    onDown(_row, e) {
      detailTimers.cancelPendingClick(); // 新按下取代上次点击的待开板（与清单待开详情同款竞态）
      if ((e.target as HTMLElement).closest(".del")) return false;
      return true;
    },
    commit(ctx) {
      const ul = document.getElementById("bubble-list");
      if (!ul) return;
      const ids = [...ul.querySelectorAll(".bubble-row")]
        .map((r) => Number((r as HTMLElement).dataset.rowId))
        // PATCH002.89：占位符 ghost 也匹配 .bubble-row，无 rowId ⇒ 必须滤掉 NaN
        .filter((id) => Number.isInteger(id) && id !== ctx.id);
      ids.splice(ctx.toIdx, 0, ctx.id);
      ctx.expectedIds = ids;
      return invoke("bubble_reorder", { ids }).catch((err) => {
        console.error("重排失败", err);
        ctx.expectedIds = undefined;
      });
    },
    rerender(ctx) {
      hookBag.rerenderBubbles?.(ctx); // 收场重拉走 hookBag（同清单路，见上注）
    },
  },
  {
    // PL027 详情块（**仅图块可拖**：文字块拖动会与文本选择/光标冲突；拖图即改变它与
    // 文字块的相对顺序，文字块随让位自动重排）。槽位号 = 渲染序号（1 起）——块 id 在
    // 本地新块落库前为 0，用序号做 DOM 标识；落点提交交给组件 hook 换算成真实块 id
    name: "note",
    scopeId: "detail-note",
    rowSel: ".note-block",
    itemSel: ".note-block",
    ghostClass: "note-block note-ghost",
    // PATCH002.82：占位符留到重拉完成（列表高度不塌，滚动不被浏览器夹到短内容底部）
    keepGhost: true,
    // PATCH002.54 / .55（注入点，机制缺省不动）：详情块的几何策略落详情自己的模块
    autoScrollProbe: "pointer",
    autoScrollZone: 12, // PATCH002.79：详情板矮，32px 带会让末尾拖图一路滚到底
    swapRule: noteSwapRule,
    onDown(_row, e) {
      // PATCH002.77：Shift+点击 = 跨块扩选——不起拖、也不许清掉刚提升的选区锚点
      if (e.shiftKey) return false;
      if (!isNoteDragTarget(e.target as HTMLElement)) return false;
      noteHooks.onDown?.(); // 轻量：只清选区；全量保存延到 onEngage（FIX014.34）
      return true;
    },
    onEngage() {
      noteHooks.onEngage?.();
    },
    commit(ctx) {
      const ul = document.getElementById("detail-note");
      if (!ul) return;
      const slots = [...ul.querySelectorAll(".note-block")]
        .map((r) => Number((r as HTMLElement).dataset.rowId))
        // PATCH002.82：ghost 无 data-row-id ⇒ NaN，显式过滤（占位符留到重拉后再撤）
        .filter((slot) => Number.isInteger(slot) && slot !== ctx.id);
      slots.splice(ctx.toIdx, 0, ctx.id);
      noteHooks.commit?.(slots);
    },
    rerender(ctx) {
      noteHooks.rerender?.(ctx);
    },
  },
];

/** 详情块拖拽钩子（组件层注入：按下清选 / 起拖保存 / 落点提交 / 收场重绘；机制本体零差异） */
const noteHooks: {
  onDown?: () => void;
  onEngage?: () => void;
  commit?: (slots: number[]) => void;
  rerender?: (ctx?: DragCtx) => void;
} = {};

/**
 * 安装详情块拖拽钩子（DetailOverlay 挂载时调用一次）
 * @param hooks onDown（按下轻量清选）/ onEngage（真正长按起拖时 flush 保存）/
 *   commit（槽位序 → 真实块序落库）/ rerender（收场重拉块列表）
 */
export function installNoteDragHooks(hooks: {
  onDown: () => void;
  onEngage: () => void;
  commit: (slots: number[]) => void;
  rerender: (ctx?: DragCtx) => void;
}): void {
  noteHooks.onDown = hooks.onDown;
  noteHooks.onEngage = hooks.onEngage;
  noteHooks.commit = hooks.commit;
  noteHooks.rerender = hooks.rerender;
}

// 组件层注入的收场重拉句柄：cancelPendingClick / markSuppress / rerenderTodos /
// rerenderBubbles（四个钩子全部由 installDragTargets 注册——组合式不持组件引用）
const hookBag: {
  rerenderTodos?: (ctx?: DragCtx) => void;
  rerenderBubbles?: (ctx?: DragCtx) => void;
} = {};

/**
 * 组件层安装拖拽钩子（TodoList/BubblesView 挂载时调用一次）
 * @param hooks cancelPendingClick（掐待办定时器）/ markSuppress（落点抑制写回）/
 *   rerenderTodos / rerenderBubbles（收场重拉，接 ctx 取 ghost / expectedIds）
 */
export function installDragHooks(hooks: {
  cancelPendingClick: () => void;
  markSuppress: () => void;
  rerenderTodos: (ctx?: DragCtx) => void;
  rerenderBubbles: (ctx?: DragCtx) => void;
}): void {
  detailTimers.cancelPendingClick = hooks.cancelPendingClick;
  detailTimers.markSuppress = hooks.markSuppress;
  hookBag.rerenderTodos = hooks.rerenderTodos;
  hookBag.rerenderBubbles = hooks.rerenderBubbles;
}

let dragCtx: DragCtx | null = null;
let suppressUntil = 0; // 拖拽结束后 350ms 内的点击不当作行点击

/** 拖拽在飞判定（useBoardRead skipDuringDrag 注册用） */
function dragEngaged(): boolean {
  return dragCtx !== null && dragCtx.engaged;
}
registerDragProbe(dragEngaged);

/** Vue 冻结防线（红线：拖拽期冻结列表响应式重渲染）：engaged 期间外部变更到达
 * 不重渲染（重挂 DOM 与虚拟 DOM 打架），落点收场统一重拉。组件 watch 经此早退 */
export function deferDuringDrag(): boolean {
  return dragEngaged();
}

/**
 * 安装拖拽机制：document 级 mousedown/mousemove/mouseup + window blur 收尾。
 * 在 App 顶层调用一次（组合式无 design 的全局脚本语义）
 */
export function useDragReorder(): void {
  const onMouseDown = (e: MouseEvent): void => {
    if (e.button !== 0 || dragCtx) return;
    const el = e.target as HTMLElement | null;
    if (!el) return;
    for (const t of DRAG_TARGETS) {
      const row = el.closest(`#${t.scopeId} ${t.rowSel}`) as HTMLElement | null;
      if (!row) continue;
      if (rowMaskDead(row)) return; // 侵入溶解带 ≥38%：整卡罩死，点/拖全失效
      if (t.onDown(row, e) === false) return;
      const li = row.closest(t.itemSel) as HTMLElement | null;
      if (!li || isLeavingRow(li)) return; // 塌缩中不可拖（FIX014.33）
      const cardEl = document.getElementById("board") as HTMLElement;
      dragCtx = {
        t,
        li,
        row,
        id: Number(row.dataset.rowId),
        startX: e.clientX,
        startY: e.clientY,
        engaged: false,
        timer: window.setTimeout(() => {
          if (dragCtx) engageDrag(dragCtx);
        }, HOLD_MS), // 长按 0.25s 进入拖拽
        ghost: document.createElement("li"),
        cardEl,
        cardRect: new DOMRect(),
        selfTop: 0,
        selfMid: 0,
        selfHeight: 0,
        listEl: cardEl,
        rows: [],
        fromIdx: 0,
        toIdx: 0,
        scrollDelta: 0,
        centerMin: 0,
        centerMax: 0,
        center: 0,
        pointerY: 0,
        moved: false,
        padTop: 0,
        padBottom: 0,
        raf: 0,
      };
      return;
    }
  };

  /** 长按到点：行浮起进入拖拽（重挂卡片层 + 让位快照，全套起拖动作） */
  function engageDrag(ctx: DragCtx): void {
    // FIX014.33：按住期间行也可能开始离场（或已被移除）；到点复核一次，绝不重挂
    // 一个正在塌缩的节点。
    if (!ctx.li.isConnected || isLeavingRow(ctx.li)) {
      dragCtx = null;
      return;
    }
    ctx.engaged = true;
    detailTimers.cancelPendingClick(); // 起拖即掐待办点击（design 的 setDetail(false) 语义由父级在 onRowClick 抑制实现）
    ctx.t.onEngage?.(ctx);
    ctx.li.classList.remove("entering");
    const selfRect = ctx.li.getBoundingClientRect();
    ctx.selfTop = selfRect.top;
    ctx.selfMid = selfRect.top + selfRect.height / 2;
    ctx.selfHeight = selfRect.height;
    // 等高占位符留在原槽位：布局零扰动，重排时滑向目标槽充当落点指示
    ctx.ghost.className = `${ctx.t.ghostClass} shifting`;
    ctx.ghost.style.height = `${selfRect.height}px`;
    ctx.li.before(ctx.ghost);
    // 行本体重挂卡片层：absolute 锁定原位（left/top/width 取实测），随指针平移浮起
    ctx.cardRect = ctx.cardEl.getBoundingClientRect();
    ctx.li.classList.add("dragging");
    ctx.li.style.position = "absolute";
    ctx.li.style.left = `${selfRect.left - ctx.cardRect.left}px`;
    ctx.li.style.top = `${selfRect.top - ctx.cardRect.top}px`;
    ctx.li.style.width = `${selfRect.width}px`;
    ctx.cardEl.appendChild(ctx.li);
    // 列表快照：占位符占 fromIdx，其余行为让位对象（transform 不改布局，实测即原位）。
    // 单行也照常进入拖拽循环（FIX004.4 终版交互：长按浮起跟手，松手回原位——
    // 无可让位行只是排序无变化，不能因此剥夺拖拽体验；收场由目标 ghost 交接收口）
    ctx.listEl = document.getElementById(ctx.t.scopeId) as HTMLElement;
    ctx.rows = [...ctx.listEl.querySelectorAll(ctx.t.itemSel)].map((el) => {
      const r = (el as HTMLElement).getBoundingClientRect();
      return { el: el as HTMLElement, top: r.top, mid: r.top + r.height / 2, height: r.height };
    });
    ctx.fromIdx = ctx.rows.findIndex((x) => x.el === ctx.ghost);
    ctx.toIdx = ctx.fromIdx; // 未移动即松手 = 原位（toIdx 缺省 0 会把行误排到顶）
    ctx.rows.forEach((r, i) => {
      if (i !== ctx.fromIdx) r.el.classList.add("shifting");
    });
    ctx.listEl.classList.add("drag-live"); // 拖拽中锁滚动：防布局快照失真
    // 起拖清收尾带：拖拽从"到底静止"起步时内联 --fade-btm 停在 100%（到底分支所置），
    // 拖拽期 scroll 监听不清内联值——不清会让自动滚动中越过底缘的行被硬切无软边
    ctx.listEl.style.removeProperty("--fade-btm");
    ctx.listEl.classList.remove("at-bottom");
    document.body.style.userSelect = "none";
    const sel = window.getSelection();
    if (sel) sel.removeAllRanges();
    ctx.li.style.transform = "translateY(0px) scale(1.03)"; // 起拖浮起反馈
    // 自动滚动状态：滚动补偿累加器、当前被拖行中心、列表上下内距（边缘带基准）
    ctx.scrollDelta = 0;
    ctx.center = ctx.selfMid;
    ctx.moved = false; // 边缘自动滚动的解锁钥匙：指针实际位移超 3px 才置真
    const cs = getComputedStyle(ctx.listEl);
    ctx.padTop = parseFloat(cs.paddingTop) || 0;
    ctx.padBottom = parseFloat(cs.paddingBottom) || 0;
    ctx.raf = requestAnimationFrame(dragAutoScroll);
  }

  /** 拖拽让位编排：按被拖行中心算目标槽——占位符滑向目标槽、途经行 ±一槽平移 */
  function applyDragShifts(ctx: DragCtx, center: number): void {
    const eff = center + ctx.scrollDelta; // 快照定格在起拖视口：滚过的增量须加回
    // PATCH002.83：引擎钳制边界换算到 eff 空间交给策略（策略不再 import 引擎常量）
    ctx.centerMin = ctx.cardRect.top + DRAG_EDGE_MARGIN + ctx.selfHeight / 2 + ctx.scrollDelta;
    ctx.centerMax = ctx.cardRect.bottom - DRAG_EDGE_MARGIN - ctx.selfHeight / 2 + ctx.scrollDelta;
    // PATCH002.55（注入点）：缺省 = 现"中心对中心"公式；详情块注入"按方向取缘"
    let to = 0;
    if (ctx.t.swapRule) {
      to = ctx.t.swapRule(ctx, eff);
    } else {
      to = defaultSwapRule(ctx, eff);
    }
    ctx.toIdx = to;
    const unit = ctx.selfHeight + ROW_GAP;
    const target = ctx.rows[to]; // 目标槽相邻参照行（to === fromIdx 时即占位符自身，位移为零）
    ctx.rows.forEach((r, i) => {
      if (i === ctx.fromIdx) {
        // 占位符滑向目标槽位：变高行按实测行高推算落点（to>from 取行[to]新底 + 半隙，
        // to<from 对齐行[to]原顶），与让位行的位移严格咬合
        let pShift = 0;
        if (to > ctx.fromIdx) pShift = target.top + target.height - ctx.selfHeight - ctx.selfTop;
        else if (to < ctx.fromIdx) pShift = target.top - ctx.selfTop;
        r.el.style.transform = pShift ? `translateY(${pShift}px)` : "";
        return;
      }
      let shift = 0;
      if (ctx.fromIdx < to && i > ctx.fromIdx && i <= to) shift = -unit; // 下拖：途经行上移
      if (to < ctx.fromIdx && i >= to && i < ctx.fromIdx) shift = unit; // 上拖：途经行下移
      r.el.style.transform = shift ? `translateY(${shift}px)` : "";
    });
  }

  /** 拖拽边缘自动滚动：中心进边缘带按贴近程度渐加速，越出全速；滚量补偿进换位判定 */
  function dragAutoScroll(): void {
    const ctx = dragCtx;
    if (!ctx || !ctx.engaged) return; // 松手/收场后下一帧自然终止
    if (!ctx.moved) {
      ctx.raf = requestAnimationFrame(dragAutoScroll);
      return;
    }
    const sr = ctx.listEl.getBoundingClientRect();
    const topEdge = sr.top + ctx.padTop;
    const bottomEdge = sr.bottom - ctx.padBottom;
    // PATCH002.79：内容不溢出时不滚（短列表直接跳）；触发带按 target 覆盖
    if (ctx.listEl.scrollHeight - ctx.listEl.clientHeight <= 1) {
      ctx.raf = requestAnimationFrame(dragAutoScroll);
      return;
    }
    const zone = ctx.t.autoScrollZone ?? AUTO_SCROLL_ZONE;
    // PATCH002.54（注入点）：缺省 = 被钳制的行中心（现行为）；详情块用未钳制的指针 y
    const probe = ctx.t.autoScrollProbe === "pointer" ? ctx.pointerY : ctx.center;
    let speed = 0;
    if (probe < topEdge + zone) {
      speed = -AUTO_SCROLL_SPEED * Math.min(1, (topEdge + zone - probe) / zone);
    } else if (probe > bottomEdge - zone) {
      speed = AUTO_SCROLL_SPEED * Math.min(1, (probe - (bottomEdge - zone)) / zone);
    }
    if (speed) {
      const before = ctx.listEl.scrollTop;
      ctx.listEl.scrollTop = before + speed;
      const moved = ctx.listEl.scrollTop - before; // 滚到头被截断：只补实际增量
      if (moved) {
        ctx.scrollDelta += moved;
        applyDragShifts(ctx, ctx.center);
      }
    }
    ctx.raf = requestAnimationFrame(dragAutoScroll);
  }

  const onMouseMove = (e: MouseEvent): void => {
    const ctx = dragCtx;
    if (!ctx) return;
    // FIX016.17：mouseup 丢失但窗口未失焦时，无左键移动按收场处理，不能让行跟手。
    if ((e.buttons & 1) === 0) {
      onMouseUp();
      return;
    }
    if (!ctx.engaged) {
      // 长按等待期：移动超阈值 = 不是长按拖拽，取消本次手势（松手仍是点击）
      if (
        Math.abs(e.clientY - ctx.startY) > DRAG_THRESHOLD ||
        Math.abs(e.clientX - ctx.startX) > DRAG_THRESHOLD
      ) {
        window.clearTimeout(ctx.timer);
        dragCtx = null;
      }
      return;
    }
    // 被拖行中心钳制在卡片内（上下留边距）：行已重挂卡片层，浮起不受列表容器裁剪
    const dy = e.clientY - ctx.startY;
    ctx.pointerY = e.clientY; // PATCH002.54：未钳制指针 y（详情块自动滚动基准）
    if (!ctx.moved && Math.abs(dy) > 3) ctx.moved = true; // 指针位移超 3px：解锁边缘自动滚动
    const minCenter = ctx.cardRect.top + DRAG_EDGE_MARGIN + ctx.selfHeight / 2;
    const maxCenter = ctx.cardRect.bottom - DRAG_EDGE_MARGIN - ctx.selfHeight / 2;
    const cdy = Math.min(Math.max(ctx.selfMid + dy, minCenter), maxCenter) - ctx.selfMid;
    ctx.li.style.transform = `translateY(${cdy}px) scale(1.03)`; // 只 Y 轴跟手，锁 X 防晃
    const center = ctx.selfMid + cdy;
    ctx.center = center; // 自动滚动循环按当前中心决定是否进边缘带
    applyDragShifts(ctx, center);
  };

  const onMouseUp = (): void => {
    const ctx = dragCtx;
    if (!ctx) return;
    dragCtx = null;
    window.clearTimeout(ctx.timer); // 长按未到点：撤销定时器
    if (ctx.raf) cancelAnimationFrame(ctx.raf); // 停自动滚动循环
    document.body.style.userSelect = "";
    if (!ctx.engaged) return; // 长按前的松手 = 点击（详情板/复制接管）
    suppressUntil = Date.now() + 350; // 拖拽结束的落点点击不当作行点击
    detailTimers.markSuppress();
    ctx.listEl.classList.remove("drag-live"); // 解锁列表滚动
    ctx.rows.forEach((r) => r.el.classList.remove("shifting"));
    // PATCH002.89（详情板 .82 同款）：ghost 留到新数据就位；不换 <ul>、不碰 scrollTop。
    // 旧行内联 transform 保持落点预览，交接时由目标在同一任务撤 ghost + 重建行。
    ctx.li.remove(); // 被拖行本体已重挂卡片层，收场由目标数据收回列表
    const committed = ctx.fromIdx !== ctx.toIdx ? ctx.t.commit(ctx) : undefined;
    if (committed) {
      // 等 reorder 落库后再重拉：数据回来才可能命中 ctx.expectedIds（治顺序回弹）
      void Promise.resolve(committed).then(
        () => ctx.t.rerender(ctx),
        (err) => {
          console.error("重排失败", err);
          ctx.expectedIds = undefined; // 失败时库序未变，不设顺序门槛
          ctx.t.rerender(ctx);
        },
      );
    } else {
      ctx.t.rerender(ctx); // 未换位：只交接 ghost，不提交
    }
  };

  const onBlur = (): void => {
    // 拖拽中窗口失焦（鼠标在窗外释放等）：按当前落点收尾，防行本体滞留卡片层
    if (dragCtx) document.dispatchEvent(new MouseEvent("mouseup"));
  };

  document.addEventListener("mousedown", onMouseDown);
  document.addEventListener("mousemove", onMouseMove);
  document.addEventListener("mouseup", onMouseUp);
  window.addEventListener("blur", onBlur);

  /** FIX016.17：卸载/重挂时无提交地收掉拖拽残留（ghost/timer/RAF/body 选择）。 */
  const abortDrag = (): void => {
    const ctx = dragCtx;
    dragCtx = null;
    if (!ctx) return;
    window.clearTimeout(ctx.timer);
    if (ctx.raf) cancelAnimationFrame(ctx.raf);
    document.body.style.userSelect = "";
    ctx.listEl.classList.remove("drag-live");
    ctx.rows.forEach((row) => row.el.classList.remove("shifting"));
    ctx.ghost.remove();
  };

  onUnmounted(() => {
    abortDrag();
    document.removeEventListener("mousedown", onMouseDown);
    document.removeEventListener("mousemove", onMouseMove);
    document.removeEventListener("mouseup", onMouseUp);
    window.removeEventListener("blur", onBlur);
  });
}

/** 拖拽落点点击抑制读取（组件行点击入口用；350ms 窗口内视为拖拽收场误触） */
export function isSuppressed(): boolean {
  return Date.now() < suppressUntil;
}
