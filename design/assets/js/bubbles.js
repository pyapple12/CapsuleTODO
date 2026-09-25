// ===== 气泡（捕获/复制回/删除/满 5 横幅/二态清空） =====
// 外围状态壳同步：空态/列表显隐、满仓警告、清空钮灰染、页签徽章——不动行 DOM，
// 供增量摘除（单删塌缩/集体清空）在动画收尾后就地刷新
function syncBubbleChrome() {
  $("bubble-empty").hidden = bubbles.length > 0;
  $("bubble-list").hidden = bubbles.length === 0;
  // 满仓警告：滚动容器内部首项，随内容一起滚动（用户定案 2026-09-25）——
  // 超过上限动态增删；渲染路径 innerHTML 会抹掉它，此处统一重建/移除
  const existing = $("bubble-banner");
  if (bubbles.length > MAX_BUBBLES) {
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
            <button class="del" data-del="${b.id}" aria-label="删除">
              <svg class="del-icon" viewBox="0 0 448 512">
                <path
                  d="M135.2 17.7L128 32H32C14.3 32 0 46.3 0 64S14.3 96 32 96H416c17.7 0 32-14.3 32-32s-14.3-32-32-32H320l-7.2-14.3C307.4 6.8 296.3 0 284.2 0H163.8c-12.1 0-23.2 6.8-28.4 17.7zM416 128H32L53.2 467c1.6 25.3 22.6 45 47.9 45H346.9c25.3 0 46.3-19.7 47.9-45L416 128z"
                ></path>
              </svg>
            </button>
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

function showCopied() {
  $("bubble-copied").hidden = false;
  window.clearTimeout(copiedTimer);
  copiedTimer = window.setTimeout(() => {
    $("bubble-copied").hidden = true;
  }, 2000);
}

// 捕获：示例片段池轮转（真实剪贴板链路属 APP，实验场只管观感）；新行原位弹出
function captureBubble() {
  bubbles.unshift({
    id: ++bubbleSeq,
    text: SAMPLE_SNIPPETS[snippetIdx % SAMPLE_SNIPPETS.length],
  });
  snippetIdx += 1;
  renderBubbles();
  popInBubble(bubbleSeq);
}

$("bubble-capture").addEventListener("click", captureBubble);

$("bubble-clear").addEventListener("click", () => {
  // 二态确认：首点变红"确认清空 N 条？"再点执行，3 秒未点复位
  if (!confirmingClear) {
    confirmingClear = true;
    $("bubble-clear").textContent = `确认清空 ${bubbles.length} 条？`;
    $("bubble-clear").classList.add("confirming");
    window.clearTimeout(confirmTimer);
    confirmTimer = window.setTimeout(() => {
      confirmingClear = false;
      $("bubble-clear").textContent = "一键清空";
      $("bubble-clear").classList.remove("confirming");
    }, 3000);
    return;
  }
  window.clearTimeout(confirmTimer);
  confirmingClear = false;
  $("bubble-clear").textContent = "一键清空";
  $("bubble-clear").classList.remove("confirming");
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
