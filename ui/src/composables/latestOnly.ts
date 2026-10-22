/**
 * 只采纳“最后一次发起”的异步请求结果。
 *
 * 背景（FIX014.31）：归档/徽章等读取没有代际守卫时，旧请求晚到会覆盖新状态。
 * 每次调用 `load` 时同步递增代际号；只有仍为最新代际时才执行 `apply`。
 *
 * @param apply 最新请求成功后的写入回调
 * @returns 包装后的加载函数（错误原样抛给调用方，由调用方决定是否记录）
 */
export function latestOnly<T>(
  apply: (value: T) => void,
): (load: () => Promise<T>) => Promise<void> {
  let seq = 0;
  return async (load) => {
    const mine = ++seq;
    const value = await load();
    if (mine === seq) apply(value);
  };
}
