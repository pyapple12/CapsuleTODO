// ===== 浮板开合帘联动（PL009.4 自 design/assets/js/glass-bar.js syncVeils 1:1）：
// 板子打开时被覆盖内容的滑杆/三角淡出隐去、关板淡入；归档板自家的滑杆例外
// （!boardOpen）；归档三角结构性豁免（V0.028 方案 B——三角住玻璃板随板隐显）。
// 浮板开合状态由组件层经 bindOverlayState 注入（组合式不持有浮板 DOM 引用） =====
import { glassBars } from "./useGlassBar";
import { boardReads } from "./useBoardRead";

/** 三浮板开合读取器（组件层挂载时注入；未注入按关板计） */
const reads: {
  board: (() => boolean) | null;
  settings: (() => boolean) | null;
  detail: (() => boolean) | null;
} = { board: null, settings: null, detail: null };

/**
 * 浮板组件挂载时注入开合态读取器（ArchiveOverlay/SettingsOverlay/DetailOverlay 调用）
 * @param name 板名
 * @param read 开合读取器（读组件 open ref 或 .open 类）
 */
export function bindOverlayState(name: "board" | "settings" | "detail", read: () => boolean): void {
  reads[name] = read;
}

/**
 * 帘联动主入口：浮板开合处调用（组件 watch 或显式挂点），遍历两注册表挂摘 .veiled
 */
export function syncVeils(): void {
  const boardOpen = reads.board?.() ?? false;
  const settingsOpen = reads.settings?.() ?? false;
  const detailOpen = reads.detail?.() ?? false;
  const anyOpen = boardOpen || settingsOpen || detailOpen;
  for (const g of glassBars) {
    const veil = g.scroller.id === "archive-list" ? !boardOpen : anyOpen;
    g.bar.classList.toggle("veiled", veil);
  }
  for (const r of boardReads) {
    // 归档实例跳过（方案 B）：三角住玻璃板随板隐显，不能吃帘——否则板开时
    // anyOpen=true 会把自家三角罩死；关板泄漏由"玻璃 visibility 连带隐藏"根治
    if (r.el.id === "archive-list") continue;
    // 详情板实例随详情板开合隐现（板开时自家三角必须可见，不吃 anyOpen 的帘）
    const veil = r.el.id === "detail-note" ? !detailOpen : anyOpen;
    for (const hint of r.hints) hint.classList.toggle("veiled", veil);
  }
}
