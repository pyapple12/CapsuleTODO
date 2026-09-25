// ===== 清单与归档板共用行模板（is-done 勾选态由数据驱动） =====
// 超时提醒（用户定案）：未完成且超 24h 黄字、超 48h 红字，均 10px 加粗；已完成不渲染。
// 提醒渲染在玻璃条外部的下方（不进 pill）
function overdueAlertHtml(t) {
  if (t.done || !t.createdAt) return "";
  const age = Date.now() - t.createdAt;
  if (age > 48 * HOUR)
    return '<p class="age-alert age-alert--red">2天都过去了哟&nbsp;&nbsp;( ﾟдﾟ) …… 大懒虫</p>';
  if (age > 24 * HOUR)
    return '<p class="age-alert age-alert--yellow">已超过24小时了哦&nbsp;&nbsp;｜ω･) WATCHING</p>';
  return "";
}

const todoRowHtml = (t) => `
          <li class="todo-item">
            <div class="todo-row ${t.done ? "is-done" : ""}" data-id="${t.id}">
            <div class="neon-checkbox" aria-hidden="true">
              <input type="checkbox" tabindex="-1" ${t.done ? "checked" : ""} />
              <div class="neon-checkbox__frame">
                <div class="neon-checkbox__box">
                  <div class="neon-checkbox__check-container">
                    <svg class="neon-checkbox__check" viewBox="0 0 24 24">
                      <path d="M5 12l5 5L20 7"></path>
                    </svg>
                  </div>
                  <div class="neon-checkbox__glow"></div>
                  <div class="neon-checkbox__borders">
                    <span></span><span></span><span></span><span></span>
                  </div>
                </div>
                <div class="neon-checkbox__particles">
                  <span></span><span></span><span></span><span></span><span></span><span></span
                  ><span></span><span></span><span></span><span></span><span></span><span></span>
                </div>
                <div class="neon-checkbox__rings">
                  <div class="ring"></div><div class="ring"></div><div class="ring"></div>
                </div>
                <div class="neon-checkbox__sparks">
                  <span></span><span></span><span></span><span></span>
                </div>
              </div>
            </div>
            <span class="t-text">${t.text}</span>
            <button class="del" data-del="${t.id}" aria-label="删除">
              <svg class="del-icon" viewBox="0 0 448 512">
                <path
                  d="M135.2 17.7L128 32H32C14.3 32 0 46.3 0 64S14.3 96 32 96H416c17.7 0 32-14.3 32-32s-14.3-32-32-32H320l-7.2-14.3C307.4 6.8 296.3 0 284.2 0H163.8c-12.1 0-23.2 6.8-28.4 17.7zM416 128H32L53.2 467c1.6 25.3 22.6 45 47.9 45H346.9c25.3 0 46.3-19.7 47.9-45L416 128z"
                ></path>
              </svg>
            </button>
            </div>
            ${overdueAlertHtml(t)}
          </li>`;

function renderTodos() {
  $("todo-active").innerHTML = todos
    .filter((t) => !t.done)
    .map(todoRowHtml)
    .join("");
  updateTodoChrome();
}

// 归档板渲染：已完成条目陈列（勾选退回清单 / 删除彻底移除）
function renderBoard() {
  const done = todos.filter((t) => t.done);
  $("archive-list").innerHTML = done.map(todoRowHtml).join("");
  $("archive-empty").hidden = done.length > 0;
  $("archive-count").textContent = String(done.length);
}

// ===== 单条目出入场编排（用户定案 2026-09-21）：全量重绘是唯一事实源，动画是渲染前后的覆盖层 =====
// 出场：重绘后在新行 li 挂 entering，320ms 后摘类（0.3s 动画 + 余量）。
// 不用 animationend——webview 窗口被遮挡时动画事件不派发（实测），定时器确定性成立
function popInRow(id) {
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  const row = $("todo-active").querySelector(`.todo-row[data-id="${id}"]`);
  const li = row ? row.closest(".todo-item") : null;
  if (!li) return;
  li.classList.add("entering");
  window.setTimeout(() => li.classList.remove("entering"), 320);
}

// 退场：钉住实测高度再挂 leaving（行高不一：超时提醒行更高），塌缩 0.3s 后执行 after；
// 时长与 CSS li.todo-item.leaving 的 transition 同步
function collapseRow(li, after) {
  if (!li || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    after();
    return;
  }
  li.style.height = `${li.offsetHeight}px`;
  li.classList.add("leaving");
  void li.offsetHeight; // 强制回流：锁定起步高度，随后收 0 才有过渡
  li.style.height = "0";
  window.setTimeout(after, 300);
}

// 空态外壳（未完成条目驱动；"已完成"已迁入归档板）
function updateTodoChrome() {
  const undone = todos.filter((t) => !t.done).length;
  $("todo-empty").hidden = undone > 0;
  $("todo-active").hidden = undone === 0;
}

$("todo-add").addEventListener("click", addTodo);
$("todo-input").addEventListener("keydown", (e) => {
  if (e.key === "Enter") addTodo();
});
$("todo-input").addEventListener("input", (e) => {
  $("todo-add").disabled = e.target.value.trim().length === 0;
});

function addTodo() {
  const text = $("todo-input").value.trim();
  if (!text) return;
  todos.push({ id: ++todoSeq, text, done: false, createdAt: Date.now(), note: "" });
  $("todo-input").value = "";
  $("todo-add").disabled = true;
  renderTodos();
  popInRow(todoSeq); // 新条目在末尾原位弹出（用户定案）
}

document.addEventListener("click", (e) => {
  const del = e.target.closest("[data-del]");
  if (del) {
    const id = Number(del.dataset.del);
    // 行内删除按所在容器分流：气泡页删气泡，清单/归档删 todo。两类行共用 data-del
    // 通道但 id 是两套独立计数器——原"先查气泡再查清单"在 id 撞车时会误删（实测删
    // todo id=1 命中气泡 id=1），故以容器为准
    if (del.closest("#bubble-list")) {
      const bi = bubbles.findIndex((x) => x.id === id);
      if (bi >= 0) bubbles.splice(bi, 1);
      renderBubbles();
      return;
    }
    const idx = todos.findIndex((t) => t.id === id);
    if (idx < 0) return;
    // 彻底删除：先塌缩一格再提交数据重绘（清单/归档两处同款退场，用户定案）；
    // 提交时重查下标，防塌缩期间新增条目导致序号漂移
    collapseRow(del.closest(".todo-item"), () => {
      const i = todos.findIndex((t) => t.id === id);
      if (i >= 0) todos.splice(i, 1);
      renderTodos();
      renderBoard(); // 板内彻底删除时同步收敛
    });
    return;
  }
  const row = e.target.closest(".todo-row");
  if (row) {
    const t = todos.find((x) => x.id === Number(row.dataset.id));
    if (!t) return;
    // 勾选框装饰层不参与命中（点击落在行元素上），清单/归档两页统一按点击坐标
    // 是否落在勾选框范围内判定（用户定案）
    const cb = row.querySelector(".neon-checkbox").getBoundingClientRect();
    const inBox =
      e.clientX >= cb.left &&
      e.clientX <= cb.right &&
      e.clientY >= cb.top &&
      e.clientY <= cb.bottom;
    // 板内行：点勾选框 = 退回清单（两拍动效）；点正文不动作（归档板只有归档功能）
    if (row.closest("#board-overlay")) {
      if (!inBox) return;
      t.done = false;
      row.querySelector(".neon-checkbox input").checked = false;
      row.classList.remove("is-done");
      clearTimeout(row._moveTimer);
      row._moveTimer = setTimeout(() => {
        if (!row.isConnected) return;
        collapseRow(row.closest(".todo-item"), () => {
          renderBoard();
          renderTodos();
          popInRow(t.id);
        });
      }, 300); // 与清单入档同拍（用户定案 300ms）
      return;
    }
    // 清单行：点勾选框 = 勾选入档——动效就地播放（组件靠 :checked 过渡，
    // 整页重渲染会打断），播完塌缩一格，再重绘进归档板（用户定案退场）
    if (inBox) {
      t.done = true;
      row.querySelector(".neon-checkbox input").checked = true;
      row.classList.add("is-done");
      clearTimeout(row._moveTimer);
      row._moveTimer = setTimeout(() => {
        // 全量重渲染/删除会换掉节点，已脱离文档则跳过
        if (!row.isConnected) return;
        collapseRow(row.closest(".todo-item"), () => {
          renderTodos();
          renderBoard();
        });
      }, 300); // 勾选动效主拍 300ms（用户定案，原 600）：粒子尾巴会被塌缩轻微截断
      // 入档即离场：详情板若正开着这条，随行一起收起
      if (detailTodo === t) setDetail(false);
      return;
    }
    // 清单行正文：单击开详情板（延迟 180ms 留双击窗口给行内改标题，用户定案缩短窗口）；
    // 拖拽落点 350ms 内的点击不算（见 mouseup）
    if (Date.now() < suppressDetailUntil) return;
    if (e.target.closest(".t-edit")) return; // 行内编辑中：点击是光标操作
    clearTimeout(detailOpenTimer);
    detailOpenTimer = setTimeout(() => openDetail(t, row), 180);
    return;
  }
  const brow = e.target.closest(".bubble-row");
  if (brow) {
    // 复制回剪贴板（实验场尽力而为：非 https 环境可能被拒，观感不受影响）
    const b = bubbles.find((x) => x.id === Number(brow.dataset.id));
    if (b && navigator.clipboard) {
      navigator.clipboard.writeText(b.text).catch(() => {});
    }
    showCopied();
  }
});
