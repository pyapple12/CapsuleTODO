// ===== 详情板编排（用户定案 2026-09-22）：点清单行正文弹出，与设置板同构——
// 行中心原点飞出、三板互斥、点板外收板。标题实时同步回清单（≤12 字由 maxlength 管），
// 内容区实时写入 todo.note，不设长度上限 =====
const detailOverlay = $("detail-overlay");
let detailTodo = null;
let detailOpenTimer = 0; // 单击开详情的延迟句柄（留双击窗口给行内改标题）
const setDetail = (open) => {
  if (!open) {
    detailOverlay.classList.remove("open");
    detailTodo = null;
    syncVeils(); // 收板：滑杆/三角淡入恢复
  }
};
const openDetail = (t, rowEl) => {
  if (boardOverlay.classList.contains("open")) setBoard(false); // 三板互斥（用户定案）
  if (settingsOverlay.classList.contains("open")) setSettings(false);
  detailTodo = t;
  $("detail-title").value = t.text;
  $("detail-note").value = t.note || "";
  // top 落位同设置板；原点 = 被点行中心的板内坐标（从行处飞出）
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
  detailOverlay.classList.add("open");
  syncVeils(); // 开板：被覆盖内容的滑杆/三角淡出隐去
  syncDetailBar(); // 打开即同步浮钮显隐与位置（内容可滚才显示）
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
  };
  const up = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
  e.preventDefault();
});

// 双击行正文：行内改标题（Enter/失焦保存、Esc 取消、≤12 字与详情板同规）；
// 同时取消未决的详情板弹出（双击窗口内第二次点击到此）
document.addEventListener("dblclick", (e) => {
  const row = e.target.closest("#todo-active .todo-row");
  if (!row || e.target.closest(".t-edit")) return;
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
