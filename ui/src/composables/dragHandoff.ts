// ===== 拖拽收场顺序判定（PATCH002.89）：重排后的服务端数据必须先命中目标 id 序，
// 才允许撤 ghost / 重建行；旧刷新响应即使后到也不能把列表打回旧序 =====

/**
 * 判断两组 id 是否**顺序完全一致**（长度与逐项相等；不做集合比较）。
 * @param actual 实际数据序
 * @param expected 目标序
 * @returns 完全一致为 true
 */
export function sameIdOrder(actual: readonly number[], expected: readonly number[]): boolean {
  return actual.length === expected.length && actual.every((id, i) => id === expected[i]);
}

/**
 * 拖拽收场是否可交接：无目标序（未换位 / 重排失败）直接放行，否则必须命中目标序。
 * @param actual 实际数据序
 * @param expected 目标序；null / undefined 表示不设顺序门槛
 * @returns 可交接为 true
 */
export function expectedOrderMatches(
  actual: readonly number[],
  expected: readonly number[] | null | undefined,
): boolean {
  return expected == null || sameIdOrder(actual, expected);
}
