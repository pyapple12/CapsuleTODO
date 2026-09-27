// ===== 删除确认收口总线（A1/A2 对齐 design rollbackDelConfirms）：
// 行被浮板遮盖或整页切走后 mouseout 不再可能触发——开盖红态会在板收/页切回后
// "复活"（design V0.026 实测）。各列表组件挂载时注册自己的收口函数，归档板
// 开合与页签切换处调用 rollbackAllDelConfirms 批量收口 =====

const rollbackFns = new Set<() => void>();

/**
 * 组件挂载时注册收口函数（摘本列表的未决确认态）
 * @param fn 收口实现（confirmingId 置空 + DelButton rollBack 清执行窗）
 * @returns 注销函数（onUnmounted 调用）
 */
export function registerDelConfirms(fn: () => void): () => void {
  rollbackFns.add(fn);
  return () => rollbackFns.delete(fn);
}

/** 批量收口所有列表的未决确认（归档板开合、页签切换挂点调用） */
export function rollbackAllDelConfirms(): void {
  for (const fn of rollbackFns) fn();
}
