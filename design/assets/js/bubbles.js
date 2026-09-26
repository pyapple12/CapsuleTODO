// ===== 气泡（捕获/复制回/删除/满 5 横幅/二态清空） =====
// 外围状态壳同步：空态/列表显隐、满仓警告、清空钮灰染、页签徽章——不动行 DOM，
// 供增量摘除（单删塌缩/集体清空）在动画收尾后就地刷新
function syncBubbleChrome() {
  $("bubble-empty").hidden = bubbles.length > 0;
  $("bubble-list").hidden = bubbles.length === 0;
  // 满仓警告：滚动容器内部首项，随内容一起滚动（用户定案 2026-09-25）——
  // 超过上限动态增删；渲染路径 innerHTML 会抹掉它，此处统一重建/移除。
  // has-warning 类同步切换容器偏移（-4 ↔ -9.5）：无警告恢复节奏、有警告定位墨迹 4.5px
  const hasWarning = bubbles.length > MAX_BUBBLES;
  $("bubble-list").classList.toggle("has-warning", hasWarning);
  const existing = $("bubble-banner");
  if (hasWarning) {
    const text = `气泡已经超过${MAX_BUBBLES}个啦！都溢出来啦！(*ﾉωﾉ) EEK`;
    if (existing) existing.textContent = text;
    else {
      const li = document.createElement("li");
      li.id = "bubble-banner";
      li.className = "full-warning";
      li.textContent = text;
      $("bubble-list").prepend(li);
    }
  } else {
    existing?.remove();
    $("bubble-list").style.removeProperty("--mask-shift"); // 警告退场同步清隐区位移（防无警告残留下推带）
  }
  $("bubble-clear").disabled = bubbles.length === 0; // 无气泡灰染不可点（用户定案 2026-09-25）
  // 页签红色徽章：读实际气泡数，固定正圆，≥10 显示"9+"（用户定案）
  $("bubble-badge").textContent = bubbles.length > 9 ? "9+" : String(bubbles.length);
  $("bubble-badge").hidden = bubbles.length === 0;
}

function renderBubbles() {
  $("bubble-list").innerHTML = bubbles
    .map(
      (b) => `
          <li class="bubble-row" data-id="${b.id}">
            <span class="b-text">${b.text}</span>
            ${delButtonHtml(b.id)}
          </li>`,
    )
    .join("");
  syncBubbleChrome();
}

// 新气泡入场：重绘后在新行挂 entering，320ms 后摘类（0.3s 动画 + 余量；与清单 popInRow
// 同拍同法——不用 animationend，webview 遮挡时动画事件不派发，定时器确定性成立）
function popInBubble(id) {
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  const li = $("bubble-list").querySelector(`.bubble-row[data-id="${id}"]`);
  if (!li) return;
  li.classList.add("entering");
  window.setTimeout(() => li.classList.remove("entering"), 320);
}

// 捕获钮占字态模板（FA6 solid clipboard-check，与删除钮/待捕获 clipboard 同族行内 SVG）
const CAPTURE_COPIED_HTML = `<svg class="cap-icon" viewBox="0 0 384 512"><path d="M192 0c-41.8 0-77.4 26.7-90.5 64H64C28.7 64 0 92.7 0 128V448c0 35.3 28.7 64 64 64H320c35.3 0 64-28.7 64-64V128c0-35.3-28.7-64-64-64H282.5C269.4 26.7 233.8 0 192 0zm0 64a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM305 273L177 401c-9.4 9.4-24.6 9.4-33.9 0L79 337c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l47 47L271 239c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9z"></path></svg>已复制到剪贴板`;
// 待捕获形态自 HTML 捕获一次（图标 + 文案），恢复态直接回放免重复维护
const captureIdleHTML = $("bubble-capture").innerHTML;

// 复制回反馈（用户定案 2026-09-25）：占用捕获钮本体——图标换 clipboard-check、文字换
// "已复制到剪贴板"、底色 50% 紫并禁点，1s 后恢复；重复触发重置计时（连点不叠加不提前解禁）
function flashCaptureCopied() {
  const btn = $("bubble-capture");
  btn.disabled = true;
  btn.innerHTML = CAPTURE_COPIED_HTML;
  window.clearTimeout(copiedTimer);
  copiedTimer = window.setTimeout(() => {
    btn.disabled = false;
    btn.innerHTML = captureIdleHTML;
  }, 1000);
}

// 确认态旁路取消（用户定案 2026-09-25）：确认清空期间点了别的入口（捕获/气泡行），
// 一键清空退回原状——复用同一复位函数，缩回动画与超时复位一致
function cancelClearConfirm() {
  resetClearButton();
}

// 捕获：示例片段池轮转（真实剪贴板链路属 APP，实验场只管观感）；新行原位弹出
function captureBubble() {
  cancelClearConfirm();
  bubbles.unshift({
    id: ++bubbleSeq,
    text: SAMPLE_SNIPPETS[snippetIdx % SAMPLE_SNIPPETS.length],
  });
  snippetIdx += 1;
  renderBubbles();
  popInBubble(bubbleSeq);
}

$("bubble-capture").addEventListener("click", captureBubble);

// 清空钮宽度动画（用户定案 2026-09-25）：width 无法从 auto 起过渡，先量两端再锁值过渡，
// 330ms 后解锁回 auto（定时器确定性成立，沿 popInBubble 先例不用 transitionend）；
// 捕获钮 flex:1 让位随 flex 逐帧重排自动成对（右钮向左扩、左钮缩小）
function animateClearWidth(from, to) {
  const btn = $("bubble-clear");
  btn.style.width = `${from}px`;
  void btn.offsetWidth; // 强制回流：过渡从锁定的旧宽起算
  btn.style.width = `${to}px`;
  window.clearTimeout(clearWidthTimer);
  clearWidthTimer = window.setTimeout(() => {
    btn.style.width = "";
  }, 330);
}

// 复位到"一键清空"（缩回路径共用：再点执行 / 悬停离开 2s 超时 / 捕获或气泡旁路取消）：
// 锁当前宽 → 换文字 → 瞬时解锁量收起宽 → 反向过渡；reduced-motion 直接换文字不动几何
function resetClearButton() {
  if (!confirmingClear) return;
  confirmingClear = false;
  window.clearTimeout(confirmTimer);
  const btn = $("bubble-clear");
  btn.classList.remove("confirming");
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    btn.textContent = "一键清空";
    return;
  }
  const from = btn.offsetWidth;
  btn.textContent = "一键清空";
  btn.style.width = "auto"; // 瞬时解锁量收起宽（同步块内改回，不产生视觉帧）
  const to = btn.offsetWidth;
  animateClearWidth(from, to);
}

// 确认态超时起算（用户定案 2026-09-25）：2s；由 click（鼠标不在钮上）或 mouseleave 触发
function startClearTimeout() {
  window.clearTimeout(confirmTimer);
  confirmTimer = window.setTimeout(resetClearButton, 2000);
}

// 悬停感知计时：确认态下鼠标回到钮上即暂停（不算超时），离开再重新起算满 2s
$("bubble-clear").addEventListener("mouseenter", () => {
  if (confirmingClear) window.clearTimeout(confirmTimer);
});

$("bubble-clear").addEventListener("mouseleave", () => {
  if (confirmingClear) startClearTimeout();
});

$("bubble-clear").addEventListener("click", () => {
  // 二态确认：首点展开"确认清空 N 条？"（宽度过渡，红色随 background 过渡加深），
  // 再点执行；超时 2s 且悬停感知——首点时鼠标在钮上则等离开才起算
  if (!confirmingClear) {
    confirmingClear = true;
    const btn = $("bubble-clear");
    btn.classList.add("confirming");
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      btn.textContent = `确认清空 ${bubbles.length} 条？`;
    } else {
      const from = btn.offsetWidth;
      btn.textContent = `确认清空 ${bubbles.length} 条？`;
      animateClearWidth(from, btn.offsetWidth);
    }
    if (!btn.matches(":hover")) startClearTimeout();
    return;
  }
  resetClearButton();
  // 集体退场特效（用户定案 2026-09-25）：所有行一并塌缩，300ms 收尾后清数据重绘；
  // reduced-motion 或空列表直接清
  const rows = [...$("bubble-list").querySelectorAll(".bubble-row")];
  if (!rows.length || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    bubbles.length = 0;
    renderBubbles();
    return;
  }
  rows.forEach((li) => collapseRow(li));
  window.setTimeout(() => {
    bubbles.length = 0;
    renderBubbles();
  }, 300); // 与 li.leaving 的塌缩时长同步（collapseRow 同源）
});

$("lab-seed").addEventListener("click", captureBubble);
