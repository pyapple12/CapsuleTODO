// ===== 主题三态状态源（设置板双开关与实验场按钮共用，双向同步，用户定案保留两处入口） =====
const themeStates = [
  { label: "跟随系统", value: null },
  { label: "浅色", value: "light" },
  { label: "暗色", value: "dark" },
];
const systemDark = window.matchMedia("(prefers-color-scheme: dark)");
let themeIdx = systemDark.matches ? 2 : 1; // 默认手动模式：跟随系统开关默认关闭，档位取系统当前深浅（用户定案）

// 双开关与实验场按钮的界面同步（跟随系统时日夜档位实时反映系统深浅并整体灰化）；
// 手动档位的 data-theme 也在此落属性——初始加载即一致，不依赖首次切换
function syncThemeControls() {
  $("follow-switch").checked = themeIdx === 0;
  $("daynight-label").classList.toggle("is-disabled", themeIdx === 0);
  $("daynight-switch").checked = themeIdx === 0 ? systemDark.matches : themeIdx === 2;
  $("daynight-switch").disabled = themeIdx === 0; // 跟随时彻底锁死：disabled 控件对任何来源的点击免疫
  $("daynight-desc").textContent =
    themeIdx === 0 ? "跟随系统当前深浅自动切换" : "手动选择浅色或暗色";
  $("lab-theme").textContent = `主题：${themeStates[themeIdx].label}`;
  const state = themeStates[themeIdx];
  if (state.value === null) {
    document.documentElement.removeAttribute("data-theme");
  } else {
    document.documentElement.dataset.theme = state.value;
  }
}

function applyTheme(idx) {
  if (themeIdx === idx) {
    syncThemeControls();
    return;
  }
  themeIdx = idx;
  syncThemeControls();
  if (titleFX) titleFX.refresh(); // 主题 accent 换色 → 标题粒子重建
}

$("lab-theme").addEventListener("click", () => {
  applyTheme((themeIdx + 1) % themeStates.length);
});

$("follow-switch").addEventListener("change", (e) => {
  // 关闭跟随的瞬间：以系统当前深浅作为日夜开关的起始档位（用户定案）
  applyTheme(e.target.checked ? 0 : systemDark.matches ? 2 : 1);
});

$("daynight-switch").addEventListener("change", (e) => {
  applyTheme(e.target.checked ? 2 : 1);
});

systemDark.addEventListener("change", () => {
  if (themeIdx === 0) syncThemeControls(); // 跟随期间系统深浅实时变化 → 日夜档位跟随
});

syncThemeControls(); // 初始界面同步：默认跟随系统（开关勾选 + 日夜灰化随系统档位）

document.querySelectorAll(".lab [data-bg]").forEach((btn) => {
  btn.addEventListener("click", () => {
    document.body.dataset.bg = btn.dataset.bg;
  });
});

$("lab-focus").addEventListener("click", () => {
  const on = $("board").classList.toggle("focused");
  $("lab-focus").textContent = `聚焦预览：${on ? "开" : "关"}`;
});

renderTodos();
renderBubbles();
