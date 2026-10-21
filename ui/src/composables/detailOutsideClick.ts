/** 详情板“板外点击是否收板”的纯裁决（PATCH002.88）。
 *
 * 背景：框选时 mousedown 在板内、mouseup 在板外，浏览器随后补发的 click 的 target
 * 是两者在 DOM 上的最近公共祖先（通常已落到板外）。若只看 click target 是否在板内，
 * 会把“手势收尾 click”误判成“板外新点击”，导致收板。
 *
 * 因此把“本次 click 对应的 mousedown 是否从板内开始”作为独立输入：
 * - 板外新按下 → startedInside=false → 正常收板；
 * - 板内起手、板外松手 → startedInside=true → 只收手势，不关板；
 * - 行 / 气泡行的“切换详情源”例外仍优先。
 */
export interface DetailOutsideClick {
  /** 本次 click 对应的 mousedown 是否落在详情板内 */
  startedInside: boolean;
  /** click 的 target 是否在详情板内 */
  targetInside: boolean;
  /** click 是否落在清单行 / 气泡行（既有“切换详情源”例外） */
  targetIsRow: boolean;
}

/**
 * 板外点击是否应关详情板。
 * @param ctx 手势来源、click target 与行例外三态
 * @returns 仅“从板外新按下”的 click 返回 true
 */
export function shouldCloseDetailOnOutsideClick(ctx: DetailOutsideClick): boolean {
  return !ctx.targetInside && !ctx.targetIsRow && !ctx.startedInside;
}
