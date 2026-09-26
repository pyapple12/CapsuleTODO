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

// 删除钮统一模板（清单/归档/气泡三处共用）：FA6 trash-can 双 path——盖子 .del-lid
// 在后=绘制在上（二态确认开盖用）。抽公共函数防多处副本漂移（气泡页旧单 path 图标
// 即是副本脱节的实例）
function delButtonHtml(id) {
  return `<button class="del" data-del="${id}" aria-label="删除">
              <svg class="del-icon" viewBox="0 0 448 512">
                <path
                  d="M32 128H416V448c0 35.3-28.7 64-64 64H96c-35.3 0-64-28.7-64-64V128zm96 64c-8.8 0-16 7.2-16 16V432c0 8.8 7.2 16 16 16s16-7.2 16-16V208c0-8.8-7.2-16-16-16zm96 0c-8.8 0-16 7.2-16 16V432c0 8.8 7.2 16 16 16s16-7.2 16-16V208c0-8.8-7.2-16-16-16zm96 0c-8.8 0-16 7.2-16 16V432c0 8.8 7.2 16 16 16s16-7.2 16-16V208c0-8.8-7.2-16-16-16z"
                ></path>
                <path class="del-lid"
                  d="M135.2 17.7C140.6 6.8 151.7 0 163.8 0H284.2c12.1 0 23.2 6.8 28.6 17.7L320 32h96c17.7 0 32 14.3 32 32s-14.3 32-32 32H32C14.3 96 0 81.7 0 64S14.3 32 32 32h96l7.2-14.3z"
                ></path>
              </svg>
            </button>`;
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
            ${delButtonHtml(t.id)}
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

// 归档板渲染：已完成条目陈列（勾选退回清单 / 删除彻底移除），
// 按 doneAt 倒序（最新完成的在最上，用户定案 2026-09-25）
function renderBoard() {
  const done = todos.filter((t) => t.done).sort((a, b) => (b.doneAt || 0) - (a.doneAt || 0));
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
// 时长与 CSS li.todo-item.leaving 的 transition 同步。after 可缺省（气泡一键清空只
// 要塌缩本身，收尾由调用方统一在 300ms 后重绘）
function collapseRow(li, after) {
  if (!li || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    after?.();
    return;
  }
  li.style.height = `${li.offsetHeight}px`;
  li.classList.add("leaving");
  void li.offsetHeight; // 强制回流：锁定起步高度，随后收 0 才有过渡
  li.style.height = "0";
  window.setTimeout(() => after?.(), 300);
}

// 删除钮按压脉冲（用户定案 2026-09-26）：两拍点击共用的微缩放反馈，以按钮自身
// 中心为基准快缩快弹（样式见 CSS del-press 关键帧，0.18s）。行动态重建直接挂钮；
// remove + 回流 + add 保证快速连点时动画从头重播，animationend 后摘类复原
function pressPulse(btn) {
  btn.classList.remove("del-press");
  void btn.offsetWidth; // 强制回流：重入时动画能从头重播
  btn.classList.add("del-press");
  btn.addEventListener("animationend", () => btn.classList.remove("del-press"), {
    once: true,
  });
}

// 未决确认批量回退（板开合/换页挂点调用）：行被浮板遮盖或整页切走后，开盖态不可见
// 且 mouseout 不再可能触发——留着会在板收/页切回后以开盖红态复活（鼠标已不在钮上）
function rollbackDelConfirms() {
  document.querySelectorAll(".del-open").forEach((li) => li.classList.remove("del-open"));
}

// 行内删除通用链路：气泡页删气泡，清单/归档删 todo。两类行共用 data-del 通道
// 但 id 是两套独立计数器——原"先查气泡再查清单"在 id 撞车时会误删（实测删 todo
// id=1 命中气泡 id=1），故以容器为准。三处二态确认的第二拍统一延迟 200ms 调用
function runRowDelete(del) {
  const id = Number(del.dataset.del);
  if (del.closest("#bubble-list")) {
    // 气泡删除：塌缩一格再增量摘除（与清单同款退场，2026-09-25）——全量重绘会
    // 拔掉其他行在飞的动画；外围状态壳由 syncBubbleChrome 就地刷新
    const li = del.closest(".bubble-row");
    collapseRow(li, () => {
      const i = bubbles.findIndex((x) => x.id === id);
      if (i >= 0) bubbles.splice(i, 1);
      li?.remove();
      syncBubbleChrome();
    });
    return;
  }
  const idx = todos.findIndex((t) => t.id === id);
  if (idx < 0) return;
  const li = del.closest(".todo-item");
  const inBoard = !!del.closest("#board-overlay"); // 板内删除：需同步计数与空态
  // 彻底删除：先塌缩一格再提交数据（清单/归档两处同款退场，用户定案）；
  // 提交时重查下标，防塌缩期间新增条目导致序号漂移。
  // 收尾增量摘除（用户定案）：塌缩行已收 0 高直接摘壳、不做全量重绘——
  // 归档板快速连删时，另一行在飞的塌缩动画不再被重建拔除
  collapseRow(li, () => {
    const i = todos.findIndex((t) => t.id === id);
    if (i >= 0) todos.splice(i, 1);
    li?.remove();
    if (inBoard) {
      const doneCount = todos.filter((t) => t.done).length;
      $("archive-count").textContent = String(doneCount);
      $("archive-empty").hidden = doneCount > 0;
    } else {
      updateTodoChrome(); // 清单侧删除：空态壳同步
    }
  });
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
  todos.push({
    id: ++todoSeq,
    text,
    done: false,
    doneAt: null,
    createdAt: Date.now(),
    note: "",
  });
  $("todo-input").value = "";
  $("todo-add").disabled = true;
  renderTodos();
  popInRow(todoSeq); // 新条目在末尾原位弹出（用户定案）
}

document.addEventListener("click", (e) => {
  const del = e.target.closest("[data-del]");
  if (del) {
    // 罩死行删除钮失效（用户定案 2026-09-26，气泡并入）：侵入溶解带 ≥38% 整卡死透；
    // 归档板无罩死语义
    const deadRow =
      del.closest("#todo-active .todo-row") ?? del.closest("#bubble-list .bubble-row");
    if (deadRow && rowMaskDead(deadRow)) return;
    // 行内删除二态确认（清单/归档/气泡统一，用户定案 2026-09-26）：首点盖翻起 +
    // Delete 字渐隐 + 按压脉冲，状态冻结保持——鼠标离开删除钮即回退（盖子直接
    // 归位），移回显红。再点执行：脉冲先播完再进删除链路（同拍起跑按压反馈会被
    // 塌缩吞掉）
    const li = del.closest(".todo-item, .bubble-row");
    if (li.dataset.delBusy) return; // 执行窗口锁：200ms 窗口内再点忽略，防双删
    if (li.classList.contains("del-open")) {
      // 第二拍（执行删除）：del-open 此拍保持——红底稳定走完脉冲，进链路时才
      // 解锁（缩灰与塌缩并行退场）
      li.dataset.delBusy = "1";
      pressPulse(del);
      setTimeout(() => {
        delete li.dataset.delBusy;
        li.classList.remove("del-open");
        runRowDelete(del);
      }, 200);
    } else {
      // 单实例全局唯一：开盖时点别的行，旧的合盖缩回（执行窗口中的行保持红底等塌缩）
      const stale = document.querySelector(".del-open:not([data-del-busy])");
      if (stale) stale.classList.remove("del-open");
      li.classList.add("del-open");
      pressPulse(del); // 第一拍：确认态出现瞬间的按压反馈
      // 离开回退不在点击链路处理：开盖后由文件尾的 mouseout 委托统一立即回退
    }
    return;
  }
  const row = e.target.closest(".todo-row");
  if (row) {
    const t = todos.find((x) => x.id === Number(row.dataset.id));
    if (!t) return;
    // 罩死行勾选框失效（用户定案 2026-09-26）：仅约束清单行，归档板无罩死语义
    if (row.closest("#todo-active") && rowMaskDead(row)) return;
    // 勾选框装饰层不参与命中（点击落在行元素上），清单/归档两页统一按点击坐标
    // 是否落在勾选框范围内判定（用户定案）
    const cb = row.querySelector(".neon-checkbox").getBoundingClientRect();
    const inBox =
      e.clientX >= cb.left &&
      e.clientX <= cb.right &&
      e.clientY >= cb.top &&
      e.clientY <= cb.bottom;
    // 板内行：点勾选框 = 退回清单（两拍动效）；点正文不动作（归档态 note 不可见，用户定案）
    if (row.closest("#board-overlay")) {
      if (!inBox) return;
      t.done = false;
      t.doneAt = null; // 退回清单：清除入档时间
      row.querySelector(".neon-checkbox input").checked = false;
      row.classList.remove("is-done");
      clearTimeout(row._moveTimer);
      row._moveTimer = setTimeout(() => {
        if (!row.isConnected) return;
        const li = row.closest(".todo-item");
        collapseRow(li, () => {
          // 归档板增量摘除（用户定案）：塌缩行已收 0 高直接摘壳 + 计数/空态同步——
          // 快速连点退回时，另一行在飞的动画不再被 renderBoard 重建拔除；
          // 清单侧 renderTodos 在板遮挡下进行，可能的截断不可见
          li?.remove();
          const doneCount = todos.filter((t) => t.done).length;
          $("archive-count").textContent = String(doneCount);
          $("archive-empty").hidden = doneCount > 0;
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
      t.doneAt = Date.now(); // 入档时间（归档板排序依据，退回清单时清空）
      row.querySelector(".neon-checkbox input").checked = true;
      row.classList.add("is-done");
      clearTimeout(row._moveTimer);
      row._moveTimer = setTimeout(() => {
        // 全量重渲染/删除会换掉节点，已脱离文档则跳过
        if (!row.isConnected) return;
        collapseRow(row.closest(".todo-item"), () => {
          // 增量摘除（用户定案）：塌缩行已收到 0 高，摘壳替代全量重绘——
          // 快速连勾两行时，另一行在飞的塌缩动画不再被重建连根拔除
          row.closest(".todo-item")?.remove();
          renderBoard();
          updateTodoChrome();
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
    if (rowMaskDead(row)) return; // 侵入溶解带 ≥30%：整卡罩死不可点开（用户定案 2026-09-26）
    clearTimeout(detailOpenTimer);
    detailOpenTimer = setTimeout(() => openDetail(t, row), 180);
    return;
  }
  const brow = e.target.closest(".bubble-row");
  if (brow) {
    if (Date.now() < suppressDetailUntil) return; // 拖拽收场落点点击不当作行点击（用户定案）
    if (rowMaskDead(brow)) return; // 侵入溶解带 ≥38%：整卡罩死不可点开（用户定案 2026-09-26）
    cancelClearConfirm(); // 确认清空期间点气泡：一键清空旁路退回（开板/复制两动作共通，首点即退）
    // 单击开全文板延迟 180ms（与清单开详情同拍）：留双击窗口给"双击复制"（用户定案
    // 2026-09-26 对调：单击开板、双击复制）；双击的第一次点击不开板，detail.js 的
    // dblclick 分支清此定时器
    const b = bubbles.find((x) => x.id === Number(brow.dataset.id));
    if (!b) return;
    window.clearTimeout(bubbleCopyTimer);
    bubbleCopyTimer = window.setTimeout(() => openBubbleDetail(b, brow), 180);
  }
});

// 确认态离开即回退（用户定案 2026-09-26 三次修正；归档/气泡并入同款）：开盖后鼠标
// 离开删除钮，立即回退非红垃圾桶态（盖子直接归位，无合盖动画）。行动态重建，走委托
// 不走逐钮挂
document.addEventListener("mouseout", (e) => {
  const del = e.target.closest(".del");
  if (!del) return;
  // relatedTarget 仍在按钮内 = 只是按钮内部子元素间移动（svg/path 之间），
  // 不是真离开——开盖后手部 1px 微动跨子元素边界曾致开盖瞬间回退（实测）
  if (del.contains(e.relatedTarget)) return;
  const li = del.closest(".todo-item, .bubble-row");
  if (li?.dataset.delBusy) return; // 执行窗口锁：红底保持到塌缩起跑，不被离开打断
  if (li?.classList.contains("del-open")) {
    li.classList.remove("del-open"); // 离开即回退：盖子直接归位
  }
});
