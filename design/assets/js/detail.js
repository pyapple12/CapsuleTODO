// ===== 详情板编排（用户定案 2026-09-22）：点清单行正文弹出，与设置板同构——
// 行中心原点飞出、三板互斥、点板外收板。标题实时同步回清单（≤12 字由 maxlength 管），
// 内容区实时写入 todo.note，不设长度上限 =====
const detailOverlay = $("detail-overlay");
let detailTodo = null;
let detailBubble = null; // 气泡模式：双击气泡行弹只读全文板（用户定案 2026-09-25）
let detailOpenTimer = 0; // 单击开详情的延迟句柄（留双击窗口给行内改标题）
const setDetail = (open) => {
  if (!open) {
    detailOverlay.classList.remove("open");
    detailTodo = null;
    detailBubble = null; // gate 失效先于同步：三角立即隐去，不等 0.4s 收板动画
    syncHints();
    syncVeils(); // 收板：滑杆/三角淡入恢复
    // 清到底抬带残留：类常驻后（双模式都出溶解带），下次开板不继承上次滚动态
    const note = $("detail-note");
    note.classList.remove("at-bottom");
    note.style.removeProperty("--fade-btm");
  }
};

// 详情板双模式（用户定案 2026-09-25）：todo = 标签"标题"+ 标题笔记可编辑（笔记卡
// 带溶解带，2026-09-25 双模式接入整板阅读）；bubble = 单层玻璃——标签头整个隐藏、
// 正文只读直接落板面（无内嵌卡），textarea 自带滚动 + detail-bar 浮钮照常工作。
// board-read 类常驻（双模式都出三角与溶解带），gate 限详情板打开时生效
const applyDetailMode = (mode) => {
  const isBubble = mode === "bubble";
  detailOverlay.classList.toggle("bubble-mode", isBubble); // 标签头/内嵌卡由 CSS 按类摘除
  $("detail-note").readOnly = isBubble;
};

// 落位与飞出原点（设置板同款，双模式共用）：top 对齐页签下缘，原点 = 行中心
const positionDetailOverlay = (rowEl) => {
  const cr = $("board").getBoundingClientRect();
  const tr = document.querySelector(".tabs").getBoundingClientRect();
  const top = Math.max(0, tr.top - cr.top - 4);
  detailOverlay.style.top = `${top}px`;
  const or = detailOverlay.getBoundingClientRect();
  const rr = rowEl.getBoundingClientRect();
  detailOverlay.style.setProperty(
    "--origin-x",
    `${Math.round(rr.left + rr.width / 2 - or.left)}px`,
  );
  detailOverlay.style.setProperty("--origin-y", `${Math.round(rr.top + rr.height / 2 - or.top)}px`);
};

const openDetail = (t, rowEl) => {
  if (boardOverlay.classList.contains("open")) setBoard(false); // 三板互斥（用户定案）
  if (settingsOverlay.classList.contains("open")) setSettings(false);
  detailTodo = t;
  detailBubble = null;
  applyDetailMode("todo");
  $("detail-title").value = t.text;
  $("detail-note").value = t.note || "";
  resetDetailScroll(); // 换内容必复位滚动：上一次会话的 scrollTop 会残留
  positionDetailOverlay(rowEl);
  detailOverlay.classList.add("open");
  syncVeils(); // 开板：被覆盖内容的滑杆/三角淡出隐去
  syncDetailBar(); // 打开即同步浮钮显隐与位置（内容可滚才显示）
  syncHints(); // 详情板实例三角几何重算（todo 模式 gate 失效即隐）
};

// 气泡全文板（只读，用户定案 2026-09-25）：复用详情板骨架，只读展示全文不回写
const openBubbleDetail = (b, rowEl) => {
  if (boardOverlay.classList.contains("open")) setBoard(false);
  if (settingsOverlay.classList.contains("open")) setSettings(false);
  detailBubble = b;
  detailTodo = null;
  applyDetailMode("bubble");
  const note = $("detail-note");
  note.value = b.text;
  resetDetailScroll(); // 换内容必复位滚动：上一次会话的 scrollTop 会残留（实测开板落在文末）
  positionDetailOverlay(rowEl);
  detailOverlay.classList.add("open");
  syncVeils();
  syncDetailBar();
  syncHints(); // 三角几何按 textarea 布局盒重算（layout 模式不受揭示动画 transform 污染）
};

// 滚动复位（双模式开板共用）：scrollTop 归零并补发合成 scroll——textarea 程序赋值
// 不派发事件（Chromium 固有），不补发则三角/到底抬带停在复位前状态
const resetDetailScroll = () => {
  const note = $("detail-note");
  note.scrollTop = 0;
  note.dispatchEvent(new Event("scroll"));
};

$("detail-title").addEventListener("input", () => {
  if (!detailTodo) return;
  detailTodo.text = $("detail-title").value;
  renderTodos(); // 标题实时同步回清单行
});

$("detail-note").addEventListener("input", () => {
  if (!detailTodo) return;
  detailTodo.note = $("detail-note").value;
  syncDetailBar(); // 输入改变内容量，浮钮位置/显隐随之同步
});

// 自绘滚动浮钮同步：内容可滚才显示；浮钮 4×16 固定高，top 随滚动比例移动
const syncDetailBar = () => {
  const ta = $("detail-note");
  const bar = $("detail-bar");
  const scrollable = ta.scrollHeight > ta.clientHeight + 1;
  bar.hidden = !scrollable;
  if (!scrollable) return;
  const trackH = ta.offsetHeight - 16;
  const ratio = ta.scrollTop / (ta.scrollHeight - ta.clientHeight || 1);
  bar.style.top = `${ta.offsetTop}px`;
  bar.style.height = `${ta.offsetHeight}px`;
  $("detail-thumb").style.top = `${Math.round(ratio * trackH)}px`;
};

$("detail-note").addEventListener("scroll", syncDetailBar);

// 浮钮拖拽：按住上下拖 = 滚动内容（指针捕获思路，移出浮钮不丢）
$("detail-thumb").addEventListener("pointerdown", (e) => {
  const ta = $("detail-note");
  const trackH = $("detail-bar").offsetHeight - 16;
  if (trackH <= 0) return;
  const startY = e.clientY;
  const startScroll = ta.scrollTop;
  const range = ta.scrollHeight - ta.clientHeight;
  const move = (ev) => {
    ta.scrollTop = startScroll + ((ev.clientY - startY) / trackH) * range;
    // textarea 程序赋值不派发 scroll 事件（Chromium 固有行为，实测 todo/气泡模式皆然），
    // 手动补发驱动浮钮位置与整板阅读三角/到底抬带同步
    ta.dispatchEvent(new Event("scroll"));
  };
  const up = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
  e.preventDefault();
});

// 双击行正文：气泡行 = 开只读全文板（并取消未决的单击复制，双击不闪占字）；
// 清单行 = 行内改标题（Enter/失焦保存、Esc 取消、≤12 字与详情板同规），
// 同时取消未决的详情板弹出（双击窗口内第二次点击到此）
document.addEventListener("dblclick", (e) => {
  const brow = e.target.closest("#bubble-list .bubble-row");
  if (brow) {
    window.clearTimeout(bubbleCopyTimer);
    cancelClearConfirm(); // 点气泡旁路取消一键清空确认（与单击语义一致）
    const b = bubbles.find((x) => x.id === Number(brow.dataset.id));
    if (b) openBubbleDetail(b, brow);
    return;
  }
  const row = e.target.closest("#todo-active .todo-row");
  if (!row || e.target.closest(".t-edit")) return;
  if (rowMaskDead(row)) return; // 侵入溶解带 ≥30%：整卡罩死不可双击编辑（用户定案 2026-09-26）
  clearTimeout(detailOpenTimer);
  startInlineEdit(row);
});

const startInlineEdit = (row) => {
  const t = todos.find((x) => x.id === Number(row.dataset.id));
  if (!t) return;
  const span = row.querySelector(".t-text");
  if (!span) return;
  const input = document.createElement("input");
  input.className = "t-edit";
  input.maxLength = 12;
  input.value = t.text;
  span.replaceWith(input);
  input.focus();
  input.select();
  let finished = false;
  const finish = (save) => {
    if (finished) return;
    finished = true;
    if (save) {
      const v = input.value.trim();
      if (v) t.text = v;
    }
    renderTodos(); // 重建行，编辑框换回文字
  };
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") finish(true);
    else if (e.key === "Escape") finish(false);
  });
  input.addEventListener("blur", () => finish(true));
};
