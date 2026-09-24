// ===== 玻璃滚动浮钮工厂（用户定案 2026-09-22）：所有玻璃滚动容器统一——隐藏原生
// 滑杆，4×16 透明玻璃浮钮随滚动比例移动、可拖拽。浮钮一律悬浮锚在卡片层（不进滚动
// 体内部——内贴会随内容滚走，出现"滚到半路浮钮消失"），矩形每次同步按滚动体可视
// 范围实测重算；列表 right 8.25px = 居中于"行右缘 ↔ 卡片描边"缝隙，白板 4.75px
// 与详情板对齐 =====
const glassBars = [];
const makeGlassBar = (scroller, opts = {}) => {
  scroller.classList.add("glass-scroll");
  const bar = document.createElement("div");
  bar.className = "glass-bar";
  bar.hidden = true;
  const thumb = document.createElement("i");
  thumb.className = "glass-thumb";
  bar.appendChild(thumb);
  $("board").appendChild(bar);
  bar.dataset.for = scroller.id || scroller.className; // 标识归属（调试/测试定位用）
  const sync = () => {
    const sr = scroller.getBoundingClientRect();
    const hr = $("board").getBoundingClientRect();
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
makeGlassBar($("archive-list"), { inset: true }); // 归档板列表

// ===== 整板阅读边缘三角 + 吸附收尾（用户定案 2026-09-24）：▲/▼ 锚卡片层悬浮于
// 清单上下缘通栏居中，上下还有内容即闪烁提示（换页经页签监听里 syncHints 重算）；
// scrollend 把底部"不够一整板"的半截行用 --fade-btm 渐隐隐去，残余空间让给三角 =====
const todoList = $("todo-active");
const upHint = document.createElement("div");
upHint.className = "edge-hint up";
upHint.textContent = "▲";
const downHint = document.createElement("div");
downHint.className = "edge-hint down";
downHint.textContent = "▼";
$("board").append(upHint, downHint);

// 边缘三角几何与显隐：上下缘各占 26px 通栏；清单可滚动且未到对应尽头才显示
function syncHints() {
  const sr = todoList.getBoundingClientRect();
  const hr = $("board").getBoundingClientRect();
  const scrollable = todoList.scrollHeight > todoList.clientHeight + 1;
  const showUp = scrollable && todoList.scrollTop > 2;
  const showDown =
    scrollable && todoList.scrollTop < todoList.scrollHeight - todoList.clientHeight - 2;
  upHint.hidden = !showUp;
  downHint.hidden = !showDown;
  for (const hint of [upHint, downHint]) {
    hint.style.left = `${sr.left - hr.left}px`;
    hint.style.width = `${sr.width}px`;
  }
  upHint.style.top = `${sr.top - hr.top}px`;
  downHint.style.top = `${sr.bottom - hr.top - 26}px`;
}

// 吸附收尾：mandatory snap 保证顶部已对齐行首，此处只处理底部残余——半截行顶
// 推给 --fade-btm 渐隐（不足一整板的空位让给 ▼），无半截行则回落默认软边。
// 拖拽中跳过：让位平移会令半截判定失真，松手重绘后由 MutationObserver 再收尾
function settleBoardRead() {
  if (dragCtx && dragCtx.engaged) return;
  const sr = todoList.getBoundingClientRect();
  if (!sr.height) return;
  const bottomEdge = sr.bottom - 8; // 与 scroll-padding-bottom 同源
  let fadeStart = null;
  for (const li of todoList.querySelectorAll(".todo-item")) {
    const r = li.getBoundingClientRect();
    if (r.top < bottomEdge && r.bottom > bottomEdge + 0.5) {
      fadeStart = r.top - sr.top - 2; // 半截行顶略上 2px，连间隙一起隐去
      break;
    }
  }
  if (fadeStart !== null) todoList.style.setProperty("--fade-btm", `${Math.round(fadeStart)}px`);
  else todoList.style.removeProperty("--fade-btm");
  syncHints();
}

todoList.addEventListener("scrollend", () => settleBoardRead());
todoList.addEventListener("scroll", () => {
  // 滚动途中回落默认软边：半截行允许短暂可见，停稳（scrollend）再隐
  if (!(dragCtx && dragCtx.engaged)) todoList.style.removeProperty("--fade-btm");
  syncHints(); // 三角随滚动实时显隐（用户定案）：离开顶端即亮 ▲、滚到底即熄 ▼，不等落定
});
new MutationObserver(() => settleBoardRead()).observe(todoList, { childList: true });
syncHints();
