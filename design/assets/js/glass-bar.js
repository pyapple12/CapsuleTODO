// ===== 玻璃滚动浮钮工厂（用户定案 2026-09-22）：所有玻璃滚动容器统一——隐藏原生
// 滑杆，4×16 透明玻璃浮钮随滚动比例移动、可拖拽。浮钮一律悬浮锚在卡片层（不进滚动
// 体内部——内贴会随内容滚走，出现"滚到半路浮钮消失"），矩形每次同步按滚动体可视
// 范围实测重算；列表 right 8.25px = 居中于"行右缘 ↔ 卡片描边"缝隙，白板 4.75px
// 与详情板对齐 =====
const glassBars = [];
const makeGlassBar = (scroller, opts = {}) => {
  const anchor = opts.anchor ?? $("board"); // 浮钮锚层：默认卡片层；归档板锚自家玻璃卡（用户定案 2026-09-25）
  scroller.classList.add("glass-scroll");
  const bar = document.createElement("div");
  bar.className = "glass-bar";
  bar.hidden = true;
  const thumb = document.createElement("i");
  thumb.className = "glass-thumb";
  bar.appendChild(thumb);
  anchor.appendChild(bar);
  bar.dataset.for = scroller.id || scroller.className; // 标识归属（调试/测试定位用）
  const sync = () => {
    const sr = scroller.getBoundingClientRect();
    const hr = anchor.getBoundingClientRect();
    if (!sr.height) {
      bar.hidden = true; // 所在页隐藏：无几何，先隐藏
      return;
    }
    // 轨道纵向：清单（inset）按上下内距内缩 = 行可视范围（滚到底浮钮底与末行底齐平）；
    // 白板无框，浮钮走满编辑区全高（到顶到底）
    const cs = getComputedStyle(scroller);
    const padT = opts.inset ? parseFloat(cs.paddingTop) || 0 : 0;
    const padB = opts.inset ? parseFloat(cs.paddingBottom) || 0 : 0;
    bar.style.top = `${sr.top - hr.top + padT}px`;
    bar.style.height = `${sr.height - padT - padB}px`;
    bar.style.right = `${opts.right ?? 7.75}px`;
    const scrollable = scroller.scrollHeight > scroller.clientHeight + 1;
    bar.hidden = !scrollable;
    if (!scrollable) return;
    const trackH = sr.height - padT - padB - 16; // 轨道 = 行可视高 − 浮钮高
    const ratio = scroller.scrollTop / (scroller.scrollHeight - scroller.clientHeight || 1);
    thumb.style.top = `${Math.round(ratio * trackH)}px`;
  };
  scroller.addEventListener("scroll", sync);
  scroller.addEventListener("input", sync);
  // 内容增删改变滚动量：观测子树重建即同步（bar 在滚动体外部，无需重挂）
  new MutationObserver(sync).observe(scroller, { childList: true });
  thumb.addEventListener("pointerdown", (e) => {
    const trackH = bar.getBoundingClientRect().height - 16; // 与 sync 同源：轨道实际高度
    if (trackH <= 0) return;
    const startY = e.clientY;
    const startScroll = scroller.scrollTop;
    const range = scroller.scrollHeight - scroller.clientHeight;
    const move = (ev) => {
      scroller.scrollTop = startScroll + ((ev.clientY - startY) / trackH) * range;
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    e.preventDefault();
  });
  sync();
  const entry = { scroller, bar, thumb, sync };
  glassBars.push(entry);
};

makeGlassBar($("wb-board"), { right: 7.75 }); // 白板编辑区（居中于"编辑区右缘↔卡片描边"）
makeGlassBar($("todo-active"), { inset: true }); // 清单
makeGlassBar($("bubble-list"), { inset: true }); // 气泡
// 归档板列表：锚自家玻璃卡（原锚卡片层，浮钮落在卡右 14px 沟槽、悬在卡外——用户定案
// 移上板子）；right 4.5 = 几何缝心 6 再左移 0.5px（用户定案：微避描边亮线视觉重量，
// 浮钮中心距玻璃缘 6.5px、距行缘 5.5px）
makeGlassBar($("archive-list"), {
  inset: true,
  anchor: document.querySelector("#board-overlay .board-glass"),
  right: 4.5,
});

// ===== 整板阅读边缘三角 + 吸附收尾（用户定案 2026-09-24，2026-09-25 泛化到归档板）：
// 每个挂载容器一套实例——▲/▼ 三角对、scrollend 收尾（半截行 --fade-btm 渐隐）、滚动/
// 变更监听；syncHints() 为全局别名（换页监听调用），逐实例重算显隐与几何 =====
const boardReads = [];

function makeBoardRead(el, opts = {}) {
  el.classList.add("board-read"); // 吸附/遮罩/对齐样式（glass-bar.css 按类挂载）
  const up = document.createElement("div");
  up.className = "edge-hint up";
  up.textContent = "▲";
  const down = document.createElement("div");
  down.className = "edge-hint down";
  down.textContent = "▼";
  $("board").append(up, down);
  const inst = {
    el,
    hints: [up, down],
    padB: opts.padB ?? 8, // 底缘判定：与 CSS scroll-padding-bottom 同源
    rowSel: opts.rowSel ?? ".todo-item", // 半截行判定用行选择器（气泡列表为 .bubble-row）
  };

  // 边缘三角几何与显隐：上下缘各占 26px 通栏；容器可滚动且未到对应尽头才显示
  inst.sync = () => {
    const sr = el.getBoundingClientRect();
    const hr = $("board").getBoundingClientRect();
    const scrollable = el.scrollHeight > el.clientHeight + 1;
    const showUp = scrollable && el.scrollTop > 2;
    const showDown = scrollable && el.scrollTop < el.scrollHeight - el.clientHeight - 2;
    up.hidden = !showUp;
    down.hidden = !showDown;
    for (const hint of inst.hints) {
      hint.style.left = `${sr.left - hr.left}px`;
      hint.style.width = `${sr.width}px`;
    }
    up.style.top = `${sr.top - hr.top}px`;
    down.style.top = `${sr.bottom - hr.top - 26}px`;
  };

  // 吸附收尾：mandatory snap 保证顶部已对齐行首，此处只处理底部残余——半截行顶
  // 推给 --fade-btm 渐隐（不足一整板的空位让给 ▼），无半截行则回落默认软边。
  // 拖拽中跳过（仅清单挂此选项）：让位平移会令半截判定失真，松手重绘后再收尾
  inst.settle = () => {
    if (opts.skipDuringDrag && dragCtx && dragCtx.engaged) return;
    const sr = el.getBoundingClientRect();
    if (!sr.height) return;
    // 到底分支（用户定案 2026-09-25）：底部已是内容尽头，无下一行可邀请，遮罩失去
    // 意义——带抬到 100% 全显（软边只在"底下还有内容"时才溶解）；滚离底部由 scroll
    // 监听清内联值回落默认带。顺带根治：默认软边恒比底内距宽、末行底部永远泡在带里
    if (el.scrollTop >= el.scrollHeight - el.clientHeight - 1) {
      el.classList.add("at-bottom"); // 抬带走 100ms 单独时长（类随到底态挂摘）
      el.style.setProperty("--fade-btm", "100%");
      inst.sync();
      return;
    }
    el.classList.remove("at-bottom"); // 非到底态：回落默认 1s 时长
    const bottomEdge = sr.bottom - inst.padB;
    let fadeStart = null;
    for (const li of el.querySelectorAll(inst.rowSel)) {
      const r = li.getBoundingClientRect();
      if (r.top < bottomEdge && r.bottom > bottomEdge + 0.5) {
        fadeStart = r.top - sr.top - 2; // 半截行顶略上 2px，连间隙一起隐去
        break;
      }
    }
    if (fadeStart !== null) el.style.setProperty("--fade-btm", `${Math.round(fadeStart)}px`);
    else el.style.removeProperty("--fade-btm");
    inst.sync();
  };

  el.addEventListener("scrollend", () => inst.settle());
  el.addEventListener("scroll", () => {
    // 到底判定实时化（用户定案 2026-09-25）：滚动途中内容底缘一贴住容器底缘（接触
    // 到最后一行）就地抬带（100ms 类）——落定时 settle 零变化，消除"停稳才闪没"的
    // 一瞬间；拖拽中不抬（软边全程在位，对齐 engageDrag 清带语义）。未到底照旧清值
    // 回落默认软边：半截行允许短暂可见，停稳（scrollend）再隐
    if (el.scrollTop >= el.scrollHeight - el.clientHeight - 1 && !(dragCtx && dragCtx.engaged)) {
      el.classList.add("at-bottom");
      el.style.setProperty("--fade-btm", "100%");
    } else {
      el.classList.remove("at-bottom"); // 离底即摘类：回落必须回到 1s 时长，100ms 只属于到底抬带
      if (!(dragCtx && dragCtx.engaged)) el.style.removeProperty("--fade-btm");
    }
    inst.sync(); // 三角随滚动实时显隐（用户定案）：离开顶端即亮 ▲、滚到底即熄 ▼，不等落定
  });
  new MutationObserver(() => inst.settle()).observe(el, { childList: true });
  inst.sync();
  boardReads.push(inst);
  return inst;
}

makeBoardRead($("todo-active"), { skipDuringDrag: true }); // 清单（拖拽收尾互斥）
makeBoardRead($("archive-list")); // 归档板（P1 泛化，2026-09-25）
makeBoardRead($("bubble-list"), { rowSel: ".bubble-row" }); // 气泡列表（同套整板阅读规则，2026-09-25）

// 全局别名：换页监听等既有挂点调用，逐实例重算三角显隐与几何
function syncHints() {
  for (const r of boardReads) r.sync();
}

// ===== 浮板开合联动（用户定案 2026-09-25）：板子打开时，被覆盖内容的滑杆/三角淡出
// 隐去、关板淡入；归档板自家的滑杆与三角例外（随归档板出现）——顺带根治"归档关闭
// 且条目可滚时，归档滑杆浮在清单页"的泄漏。挂点：setBoard/setSettings/openDetail/setDetail
function syncVeils() {
  const boardOpen = boardOverlay.classList.contains("open");
  const anyOpen =
    boardOpen ||
    settingsOverlay.classList.contains("open") ||
    detailOverlay.classList.contains("open");
  for (const g of glassBars) {
    const veil = g.scroller.id === "archive-list" ? !boardOpen : anyOpen;
    g.bar.classList.toggle("veiled", veil);
  }
  for (const r of boardReads) {
    // 归档实例只在归档板打开时可见（关板即隐——否则收板后三角以最后已知位置
    // 浮在清单页上，2026-09-25 实测）；清单实例随任意浮板开合隐现
    const veil = r.el.id === "archive-list" ? !boardOpen : anyOpen;
    for (const hint of r.hints) hint.classList.toggle("veiled", veil);
  }
}
