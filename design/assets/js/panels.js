// ===== 归档板开关（用户定案双出口：再点归档图标 / 点板空白处收回） =====
const boardOverlay = $("board-overlay");
const setBoard = (open) => {
  if (open) {
    if (settingsOverlay.classList.contains("open")) setSettings(false); // 两板互斥（用户定案）
    if (detailOverlay.classList.contains("open")) setDetail(false); // 三板互斥（用户定案）
    // top 动态落位：页签顶再上扩 4px（用户定案）；归档图标中心的板内坐标注入 transform-origin
    // 变量（板左缘 14px，图标中心卡内 x=20 → 板内 x=6）
    const cr = $("board").getBoundingClientRect();
    const tr = document.querySelector(".tabs").getBoundingClientRect();
    const top = Math.max(0, tr.top - cr.top - 4);
    boardOverlay.style.top = `${top}px`;
    boardOverlay.style.setProperty("--origin-x", "6px");
    boardOverlay.style.setProperty("--origin-y", `${20 - top}px`);
    renderBoard();
    // 揭示动画期间几何在缩放：动画落定后再重算滑杆/三角/收尾带（否则锚点停在中间
    // 态，2026-09-25）；settle 顺带清掉上次会话残留的内联 --fade-btm（settle 内联值
    // 粘滞：收板冻结、开板复用，实测开板即罩错末行——逐实例重算是根治）
    window.setTimeout(() => {
      for (const g of glassBars) g.sync();
      for (const r of boardReads) r.settle();
      syncHints();
    }, 450);
  }
  boardOverlay.classList.toggle("open", open);
  $("card-archive").classList.toggle("open", open); // 按钮随开合保色/褪色（用户定案）
  syncVeils(); // 滑杆/三角随浮板开合隐现（归档自家滑杆与三角豁免）
};

$("card-archive").addEventListener("click", () => {
  setBoard(!boardOverlay.classList.contains("open"));
});

// 点任一板外任何位置收板（用户定案）；板内点击与开关按钮自身的开合不在此列。
// 目标已脱离文档（入档/退回/删除触发重渲染移除原节点）时不视为板外点击，防误收板
document.addEventListener("click", (e) => {
  if (!e.target.isConnected) return;
  if (boardOverlay.classList.contains("open")) {
    if (!e.target.closest("#board-overlay") && !e.target.closest("#card-archive")) {
      setBoard(false);
    }
  }
  if (settingsOverlay.classList.contains("open")) {
    if (!e.target.closest("#settings-overlay") && !e.target.closest("#card-settings")) {
      setSettings(false);
    }
  }
  // 详情板：点板外收起；点清单行（勾选框/正文/删除）不在此列——由行分支接管开合
  if (detailOverlay.classList.contains("open")) {
    if (!e.target.closest("#detail-overlay") && !e.target.closest("#todo-active")) {
      setDetail(false);
    }
  }
});

// ===== 设置板开关编排（用户定案）：与归档板同构——齿轮原点飞出、双出口收板、互斥、保色 =====
const settingsOverlay = $("settings-overlay");
const setSettings = (open) => {
  if (open) {
    if (boardOverlay.classList.contains("open")) setBoard(false); // 两板互斥（用户定案）
    if (detailOverlay.classList.contains("open")) setDetail(false); // 三板互斥（用户定案）
    // top 动态落位同归档板；齿轮中心的板内坐标注入 transform-origin
    //（板左缘 14px，齿轮中心卡内 x = 卡宽 - 落位10 - 半宽10）
    const cr = $("board").getBoundingClientRect();
    const tr = document.querySelector(".tabs").getBoundingClientRect();
    const top = Math.max(0, tr.top - cr.top - 4);
    settingsOverlay.style.top = `${top}px`;
    settingsOverlay.style.setProperty("--origin-x", `${Math.round(cr.width - 20 - 14)}px`);
    settingsOverlay.style.setProperty("--origin-y", `${20 - top}px`);
  }
  settingsOverlay.classList.toggle("open", open);
  $("card-settings").classList.toggle("open", open); // 开板保色、收板褪色（用户定案）
  syncVeils(); // 滑杆/三角随浮板开合隐现
};

$("card-settings").addEventListener("click", () => {
  setSettings(!settingsOverlay.classList.contains("open"));
});

// 气泡提醒数量步进（范围 1~20 钳制，横幅即时重算；会话内有效刷新重置，用户定案）
const BUBBLE_MAX_LIMIT = 20;
const setBubbleMax = (v) => {
  MAX_BUBBLES = Math.min(BUBBLE_MAX_LIMIT, Math.max(1, v));
  $("bubble-count-value").textContent = String(MAX_BUBBLES);
  $("bubble-count-dec").disabled = MAX_BUBBLES <= 1;
  $("bubble-count-inc").disabled = MAX_BUBBLES >= BUBBLE_MAX_LIMIT;
  renderBubbles();
};

$("bubble-count-dec").addEventListener("click", () => setBubbleMax(MAX_BUBBLES - 1));
$("bubble-count-inc").addEventListener("click", () => setBubbleMax(MAX_BUBBLES + 1));
