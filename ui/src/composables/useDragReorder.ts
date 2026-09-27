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
import { registerDragProbe } from "./useBoardRead";

/** 长按等待期允许的抖动幅度：超出即取消本次手势 */
const DRAG_THRESHOLD = 6;
/** 长按时长（用户定案：0.5s 缩减一半） */
const HOLD_MS = 250;
/** 与 .group 的 gap 对应：一个槽位 = 被拖行高 + 间隙 */
const ROW_GAP = 6;
/** 自动滚动边缘带宽度（用户定案按推荐值） */
const AUTO_SCROLL_ZONE = 32;
/** 自动滚动全速：每帧像素数（约 600px/s） */
const AUTO_SCROLL_SPEED = 10;

/** 拖拽上下文（重挂模型的在飞状态体） */
interface DragCtx {
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
  center: number;
  moved: boolean;
  padTop: number;
  padBottom: number;
  raf: number;
}

/** 拖拽目标注册项（差异点全在此——机制本体零差异） */
export interface DragTarget {
  /** 目标名（清单 todos / 气泡 bubbles——对应 invoke 命令前缀） */
  name: "todos" | "bubbles";
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
  /** 落点提交：toIdx = 目标槽位（invoke reorder 落库） */
  commit: (ctx: { id: number; toIdx: number }) => void;
  /** 收场重绘（无论是否换位都重拉——行本体以（可能新）顺序收回列表） */
  rerender: () => void;
}

/** 组件层注入钩子（清单掐 detailOpenTimer / 气泡掐 bubbleCopyTimer；suppress 写回；
 * rerender 由收场重拉转发 hookBag） */
export interface DragHooks {
  /** 新按下取代上次点击的待办动作（清单开详情 / 气泡开板同款竞态） */
  cancelPendingClick: () => void;
  /** 拖拽结束后 350ms 内的点击不当作行点击（组件读 suppressUntil） */
  markSuppress: () => void;
  /** 收场重拉转发（hookBag 透传——todo/bubbles 两路） */
  rerenderTodos?: () => void;
  rerenderBubbles?: () => void;
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
        .filter((id) => id !== ctx.id);
      ids.splice(ctx.toIdx, 0, ctx.id);
      void invoke("todo_reorder", { ids }).catch((err) => console.error("重排失败", err));
    },
    rerender() {
      // 收场重拉走 hookBag（installDragHooks 注册）——detailTimers 只承载
      // cancelPendingClick/markSuppress 两钩子（DragHooks 接口无 rerender 字段）
      hookBag.rerenderTodos?.();
    },
  },
  {
    name: "bubbles",
    scopeId: "bubble-list",
    rowSel: ".bubble-row",
    itemSel: ".bubble-row", // 气泡行身兼外壳（无内层行容器）
    ghostClass: "bubble-row bubble-ghost", // 气泡行自带玻璃底——ghost 剥视觉见 bubbles.css
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
        .filter((id) => id !== ctx.id);
      ids.splice(ctx.toIdx, 0, ctx.id);
      void invoke("bubble_reorder", { ids }).catch((err) => console.error("重排失败", err));
    },
    rerender() {
      hookBag.rerenderBubbles?.(); // 收场重拉走 hookBag（同清单路，见上注）
    },
  },
];

/** 组件层注入的收场重拉句柄（rerender 回调转发） */
declare module "./useDragReorder" {}

// 组件层注入：cancelPendingClick / markSuppress / rerenderTodos / rerenderBubbles
// （四个钩子全部由 installDragTargets 注册——组合式不持组件引用）
const hookBag: {
  rerenderTodos?: () => void;
  rerenderBubbles?: () => void;
  /** 强制列表整建重建（vnode↔DOM 断链修复——li.remove() 外部摘除 Vue 不感知，
   * 后续 patch 全部写进失效节点；收场重建与 design 全量重绘语义等价） */
  forceRemount?: () => void;
} = {};

/**
 * 组件层安装拖拽钩子（TodoList/BubblesView 挂载时调用一次）
 * @param hooks cancelPendingClick（掐待办定时器）/ markSuppress（落点抑制写回）/
 *   rerenderTodos / rerenderBubbles（收场重拉）/ forceRemount（列表强制重建）
 */
export function installDragHooks(hooks: {
  cancelPendingClick: () => void;
  markSuppress: () => void;
  rerenderTodos: () => void;
  rerenderBubbles: () => void;
  forceRemount?: () => void;
}): void {
  detailTimers.cancelPendingClick = hooks.cancelPendingClick;
  detailTimers.markSuppress = hooks.markSuppress;
  hookBag.rerenderTodos = hooks.rerenderTodos;
  hookBag.rerenderBubbles = hooks.rerenderBubbles;
  hookBag.forceRemount = hooks.forceRemount;
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
      if (!li || li.classList.contains("leaving")) return; // 塌缩中不可拖
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
        center: 0,
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
    ctx.engaged = true;
    detailTimers.cancelPendingClick(); // 起拖即掐待办点击（design 的 setDetail(false) 语义由父级在 onRowClick 抑制实现）
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
    // 列表快照：占位符占 fromIdx，其余行为让位对象（transform 不改布局，实测即原位）
    ctx.listEl = document.getElementById(ctx.t.scopeId) as HTMLElement;
    ctx.rows = [...ctx.listEl.querySelectorAll(ctx.t.itemSel)].map((el) => {
      const r = (el as HTMLElement).getBoundingClientRect();
      return { el: el as HTMLElement, top: r.top, mid: r.top + r.height / 2, height: r.height };
    });
    if (ctx.rows.length < 2) {
      // 单行无可排序：现场还原
      dragCtx = null;
      ctx.ghost.remove();
      ctx.li.remove();
      ctx.t.rerender();
      return;
    }
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
    let to = 0;
    ctx.rows.forEach((r, i) => {
      if (i !== ctx.fromIdx && r.mid < eff) to += 1;
    });
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
    let speed = 0;
    if (ctx.center < topEdge + AUTO_SCROLL_ZONE) {
      speed =
        -AUTO_SCROLL_SPEED *
        Math.min(1, (topEdge + AUTO_SCROLL_ZONE - ctx.center) / AUTO_SCROLL_ZONE);
    } else if (ctx.center > bottomEdge - AUTO_SCROLL_ZONE) {
      speed =
        AUTO_SCROLL_SPEED *
        Math.min(1, (ctx.center - (bottomEdge - AUTO_SCROLL_ZONE)) / AUTO_SCROLL_ZONE);
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
    // 被拖行中心钳制在卡片内（上下留 12px）：行已重挂卡片层，浮起不受列表容器裁剪
    const dy = e.clientY - ctx.startY;
    if (!ctx.moved && Math.abs(dy) > 3) ctx.moved = true; // 指针位移超 3px：解锁边缘自动滚动
    const minCenter = ctx.cardRect.top + 12 + ctx.selfHeight / 2;
    const maxCenter = ctx.cardRect.bottom - 12 - ctx.selfHeight / 2;
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
    ctx.ghost.remove(); // 占位符使命完成
    ctx.li.remove(); // 重挂的行本体由重建收回列表
    if (ctx.fromIdx !== ctx.toIdx) ctx.t.commit(ctx); // 一次性提交：落点重排（invoke 落库）
    hookBag.forceRemount?.(); // vnode↔DOM 断链修复：整列表强制重建（design 全量重绘同构）
    ctx.t.rerender(); // 收场重拉：行本体以新顺序收回列表
  };

  const onBlur = (): void => {
    // 拖拽中窗口失焦（鼠标在窗外释放等）：按当前落点收尾，防行本体滞留卡片层
    if (dragCtx) document.dispatchEvent(new MouseEvent("mouseup"));
  };

  document.addEventListener("mousedown", onMouseDown);
  document.addEventListener("mousemove", onMouseMove);
  document.addEventListener("mouseup", onMouseUp);
  window.addEventListener("blur", onBlur);

  onUnmounted(() => {
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
