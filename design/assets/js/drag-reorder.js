// ===== 清单拖拽排序（生产级重挂模型 + 长按 0.5s 进入，用户定案 2026-09-22）。
// 长按模型：悬停/点击不亮抓手（点击开详情板），按住 0.5s 才进入拖拽——光标变
// 抓手、行本体重挂卡片层浮起；等待期移动超阈值即取消手势。起拖三步：等高占位
// 符留槽 → 行本体重挂卡片层随指针浮起 → 其余行平移让位、占位符滑向目标槽。松手
// 一次性重排 todos 并全量重绘。拖拽中被拖行中心贴近清单上下边缘 32px 带自动滚动
// （滚动增量补偿换位判定，玻璃滑杆沿 scroll→sync 自动跟随）。勾选框与删除按钮
// 不参与拖拽 =====
const DRAG_THRESHOLD = 6; // 长按等待期允许的抖动幅度：超出即取消本次手势
const HOLD_MS = 250; // 长按时长（用户定案：0.5s 缩减一半）
const ROW_GAP = 6; // 与 .group 的 gap 对应：一个槽位 = 被拖行高 + 间隙
const AUTO_SCROLL_ZONE = 32; // 自动滚动边缘带宽度（用户定案按推荐值）
const AUTO_SCROLL_SPEED = 10; // 自动滚动全速：每帧像素数（约 600px/s）
let dragCtx = null;
let suppressDetailUntil = 0; // 拖拽结束后 350ms 内的点击不当作"点正文开详情"

document.addEventListener("mousedown", (e) => {
  if (e.button !== 0 || dragCtx) return;
  const row = e.target.closest("#todo-active .todo-row");
  if (!row || e.target.closest(".del") || e.target.closest(".t-edit")) return; // 删除按钮/行内编辑框自有语义
  const li = row.closest(".todo-item");
  if (!li || li.classList.contains("leaving")) return; // 塌缩中不可拖
  const cb = row.querySelector(".neon-checkbox").getBoundingClientRect();
  const inBox =
    e.clientX >= cb.left && e.clientX <= cb.right && e.clientY >= cb.top && e.clientY <= cb.bottom;
  if (inBox) return; // 勾选框区域归点击切换
  dragCtx = {
    li,
    row,
    id: Number(row.dataset.id),
    startX: e.clientX,
    startY: e.clientY,
    engaged: false,
    rows: [],
    fromIdx: 0,
    toIdx: 0,
    timer: window.setTimeout(() => engageDrag(dragCtx), HOLD_MS), // 长按 0.5s 进入拖拽
  };
});

// 长按到点：行浮起进入拖拽（重挂卡片层 + 让位快照，全套起拖动作）
function engageDrag(ctx) {
  ctx.engaged = true;
  ctx.li.classList.remove("entering");
  const selfRect = ctx.li.getBoundingClientRect();
  ctx.selfTop = selfRect.top;
  ctx.selfMid = selfRect.top + selfRect.height / 2;
  ctx.selfHeight = selfRect.height;
  // 等高占位符留在原槽位：布局零扰动，重排时滑向目标槽充当落点指示
  ctx.ghost = document.createElement("li");
  ctx.ghost.className = "todo-item todo-ghost shifting";
  ctx.ghost.style.height = `${selfRect.height}px`;
  ctx.li.before(ctx.ghost);
  // 行本体重挂卡片层：absolute 锁定原位（left/top/width 取实测），随指针平移浮起
  ctx.cardEl = $("board");
  ctx.cardRect = ctx.cardEl.getBoundingClientRect();
  ctx.li.classList.add("dragging");
  ctx.li.style.position = "absolute";
  ctx.li.style.left = `${selfRect.left - ctx.cardRect.left}px`;
  ctx.li.style.top = `${selfRect.top - ctx.cardRect.top}px`;
  ctx.li.style.width = `${selfRect.width}px`;
  ctx.cardEl.appendChild(ctx.li);
  // 清单快照：占位符占 fromIdx，其余行为让位对象（transform 不改布局，实测即原位）
  ctx.listEl = document.getElementById("todo-active");
  ctx.rows = [...ctx.listEl.querySelectorAll(".todo-item")].map((el) => {
    const r = el.getBoundingClientRect();
    return { el, top: r.top, mid: r.top + r.height / 2, height: r.height };
  });
  if (ctx.rows.length < 2) {
    // 单行无可排序：现场还原
    dragCtx = null;
    ctx.ghost.remove();
    ctx.li.remove();
    renderTodos();
    return;
  }
  ctx.fromIdx = ctx.rows.findIndex((x) => x.el === ctx.ghost);
  ctx.toIdx = ctx.fromIdx; // 未移动即松手 = 原位（toIdx 缺省 0 会把行误排到顶）
  ctx.rows.forEach((r, i) => {
    if (i !== ctx.fromIdx) r.el.classList.add("shifting");
  });
  ctx.listEl.classList.add("drag-live"); // 拖拽中锁滚动：防布局快照失真
  document.body.style.userSelect = "none";
  const sel = window.getSelection();
  if (sel) sel.removeAllRanges();
  ctx.li.style.transform = "translateY(0px) scale(1.03)"; // 起拖浮起反馈
  // 自动滚动状态：滚动补偿累加器、当前被拖行中心、清单上下内距（边缘带基准）；
  // rAF 循环在 mouseup 取消
  ctx.scrollDelta = 0;
  ctx.center = ctx.selfMid;
  const cs = getComputedStyle(ctx.listEl);
  ctx.padTop = parseFloat(cs.paddingTop) || 0;
  ctx.padBottom = parseFloat(cs.paddingBottom) || 0;
  ctx.raf = requestAnimationFrame(dragAutoScroll);
}

// 拖拽让位编排：按被拖行中心算目标槽——占位符滑向目标槽、途经行 ±一槽平移。
// mousemove 与自动滚动循环共用；公式均为坐标差值，清单滚动只在换位判定补偿
function applyDragShifts(ctx, center) {
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

// 拖拽边缘自动滚动（用户定案 2026-09-24）：被拖行中心进入清单可视区上下边缘带即
// 按贴近程度渐加速，越出边缘全速；drag-live 只锁用户滚轮，程序设 scrollTop 不受限。
// 实际滚量补偿进换位判定；玻璃滑杆沿既有 scroll→sync 链路自动跟随
function dragAutoScroll() {
  const ctx = dragCtx;
  if (!ctx || !ctx.engaged) return; // 松手/收场后下一帧自然终止
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

document.addEventListener("mousemove", (e) => {
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
  // 被拖行中心钳制在卡片内（上下留 12px）：行已重挂卡片层，浮起不受清单容器裁剪
  const dy = e.clientY - ctx.startY;
  const minCenter = ctx.cardRect.top + 12 + ctx.selfHeight / 2;
  const maxCenter = ctx.cardRect.bottom - 12 - ctx.selfHeight / 2;
  const cdy = Math.min(Math.max(ctx.selfMid + dy, minCenter), maxCenter) - ctx.selfMid;
  ctx.li.style.transform = `translateY(${cdy}px) scale(1.03)`; // 只 Y 轴跟手，锁 X 防晃
  const center = ctx.selfMid + cdy;
  ctx.center = center; // 自动滚动循环按当前中心决定是否进边缘带
  applyDragShifts(ctx, center);
});

document.addEventListener("mouseup", () => {
  const ctx = dragCtx;
  if (!ctx) return;
  dragCtx = null;
  window.clearTimeout(ctx.timer); // 长按未到点：撤销定时器
  if (ctx.raf) cancelAnimationFrame(ctx.raf); // 停自动滚动循环
  document.body.style.userSelect = "";
  if (!ctx.engaged) return; // 长按前的松手 = 点击（详情板接管）
  suppressDetailUntil = Date.now() + 350; // 拖拽结束的落点点击不当作"点正文开详情"
  ctx.listEl.classList.remove("drag-live"); // 解锁清单滚动
  ctx.rows.forEach((r) => r.el.classList.remove("shifting"));
  ctx.ghost.remove(); // 占位符使命完成
  ctx.li.remove(); // 重挂的行本体由重绘收回清单
  if (ctx.fromIdx !== ctx.toIdx) {
    // 一次性提交：仅活动项参与排序（done 项不进清单，保持相对次序不变）
    const item = todos.find((t) => t.id === ctx.id);
    const actives = todos.filter((t) => !t.done);
    const dones = todos.filter((t) => t.done);
    actives.splice(actives.indexOf(item), 1);
    actives.splice(ctx.toIdx, 0, item);
    todos.splice(0, todos.length, ...actives, ...dones);
  }
  renderTodos(); // 无论是否换位都重绘：行本体以（可能新）顺序收回清单
});

// 拖拽中窗口失焦（鼠标在窗外释放等）：按当前落点收尾，防行本体滞留卡片层
window.addEventListener("blur", () => {
  if (dragCtx) document.dispatchEvent(new MouseEvent("mouseup"));
});
