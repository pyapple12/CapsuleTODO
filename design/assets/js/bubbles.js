// ===== 气泡（捕获/复制回/删除/满 5 横幅/二态清空） =====
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
  $("bubble-empty").hidden = bubbles.length > 0;
  $("bubble-list").hidden = bubbles.length === 0;
  $("bubble-banner-text").textContent = `气泡已满 ${MAX_BUBBLES} 个，该清理了`;
  $("bubble-banner").hidden = bubbles.length < MAX_BUBBLES;
  // 页签红色徽章：读实际气泡数，固定正圆，≥10 显示"9+"（用户定案）
  $("bubble-badge").textContent = bubbles.length > 9 ? "9+" : String(bubbles.length);
  $("bubble-badge").hidden = bubbles.length === 0;
}

function showCopied() {
  $("bubble-copied").hidden = false;
  window.clearTimeout(copiedTimer);
  copiedTimer = window.setTimeout(() => {
    $("bubble-copied").hidden = true;
  }, 2000);
}

// 演示捕获：示例片段池轮转（真实剪贴板链路属 APP，实验场只管观感）
$("bubble-capture").addEventListener("click", () => {
  bubbles.unshift({
    id: ++bubbleSeq,
    text: SAMPLE_SNIPPETS[snippetIdx % SAMPLE_SNIPPETS.length],
  });
  snippetIdx += 1;
  renderBubbles();
});

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
  bubbles.length = 0;
  renderBubbles();
});

$("lab-seed").addEventListener("click", () => {
  bubbles.unshift({
    id: ++bubbleSeq,
    text: SAMPLE_SNIPPETS[snippetIdx % SAMPLE_SNIPPETS.length],
  });
  snippetIdx += 1;
  renderBubbles();
});
