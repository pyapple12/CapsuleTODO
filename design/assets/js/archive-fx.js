// ===== 归档按钮换页出入场编排（用户定案）：离场=吸气放大→快速塌缩成点→粒子迸裂；
// 入场=粒子四周汇聚成点→按钮带回弹弹出。粒子为临时元素，播完即删零残留 =====
let archiveAnimTimer = 0;

function spawnArchiveParticles(mode) {
  const count = 18 + Math.floor(Math.random() * 11); // 每次随机 18~28 颗（用户定案）
  for (let i = 0; i < count; i += 1) {
    const p = document.createElement("span");
    p.className = `archive-particle${mode === "in" ? " in" : ""}`;
    const angle = (Math.PI * 2 * i) / count + (Math.random() - 0.5) * 0.6; // 均匀布角+抖动
    const dist = 10 + Math.random() * 8; // 飞行距离 10~18px（用户定案）
    p.style.left = "25px"; // 按钮中心恒定：落位10px + 半宽15px（30×30 按钮，随 CP 几何调整）
    p.style.top = "25px";
    p.style.setProperty("--dx", `${Math.cos(angle) * dist}px`);
    p.style.setProperty("--dy", `${Math.sin(angle) * dist}px`);
    p.addEventListener("animationend", () => p.remove());
    $("board").appendChild(p);
  }
}

function setArchiveVisible(visible) {
  const btn = $("card-archive");
  if (btn.hidden === !visible) return; // 幂等守卫：已处目标态不再编排（修复气泡↔白板互切误喷粒子）
  window.clearTimeout(archiveAnimTimer);
  btn.classList.remove("leaving", "entering");
  // reduced-motion：跳过编排直接显隐（粒子不生成，防 animation:none 下元素滞留）
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    btn.hidden = !visible;
    return;
  }
  if (visible) {
    spawnArchiveParticles("in"); // 粒子先四周汇聚成点
    archiveAnimTimer = window.setTimeout(() => {
      btn.hidden = false;
      btn.classList.add("entering"); // 按钮从点带回弹弹出
    }, 240); // 入场提速 50%（原 360ms）
  } else {
    btn.classList.add("leaving"); // 吸气放大→快速塌缩成点
    archiveAnimTimer = window.setTimeout(() => {
      btn.hidden = true;
      btn.classList.remove("leaving");
      spawnArchiveParticles("out"); // 塌缩触底瞬间向四周迸裂
    }, 320);
  }
}

$("card-archive").addEventListener("animationend", (e) => {
  if (e.animationName === "archive-btn-in") $("card-archive").classList.remove("entering");
});
