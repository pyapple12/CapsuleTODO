import { ref, watch, type Ref, type WatchSource } from "vue";

// ===== 空态显隐（FIX004.6 共存渲染方案的逻辑层收敛，FIX005.24）：
// TransitionGroup 恒挂载（v-if/v-else 互斥时删末条走分支整体卸载，leave 塌缩动画
// 无机会播 = 瞬间消失），空态文案延至末条 leave 播完（after-leave）出现；无 leave
// 路径（清空 DOM 直改）由 420ms 定时器兜底置位（FIX004.20 卸载清理内聚于此）。
// TodoList/BubblesView/ArchiveOverlay 三页共用

/** 兜底定时器时长（= leave 塌缩动画时长上限，三页同款） */
const EMPTY_FALLBACK_MS = 420;

/**
 * 空态显隐状态机：监听列表长度源驱动 showEmpty
 * @param lengthSource 列表长度响应源（() => props.items.length 或 computed）
 * @returns showEmpty ref + onAfterLeave 回调（模板 @after-leave 绑定）+ 卸载清理
 */
export function useEmptyState(lengthSource: WatchSource<number>) {
  // 初始值按当前事实求值（惰性）：组件创建可能晚于首拉数据（启动链路 refresh 先于
  // 子组件挂载完成时长度已非 0），写死 true 会让空态与列表共存（live 实测 2026-10-01）
  const showEmpty = ref(
    (typeof lengthSource === "function" ? lengthSource() : lengthSource.value) === 0,
  );
  let fallbackTimer = 0; // showEmpty 兜底句柄（卸载清理，FIX004.20）

  // WatchSource = getter | Ref——统一解引用为 getter（兜底与 after-leave 复用）
  const read = (): number =>
    typeof lengthSource === "function" ? lengthSource() : lengthSource.value;

  watch(lengthSource, (n, o) => {
    if (n > 0) {
      showEmpty.value = false;
    } else if ((o ?? 0) > 0) {
      // 清空：延 420ms（leave 播完）且仍为空才亮空态
      window.clearTimeout(fallbackTimer);
      fallbackTimer = window.setTimeout(() => {
        if (read() === 0) showEmpty.value = true;
      }, EMPTY_FALLBACK_MS);
    } else {
      showEmpty.value = true;
    }
  });

  /** 末条 leave 播完（TransitionGroup @after-leave）：列表真空才亮空态文案 */
  function onAfterLeave(): void {
    if (read() === 0) showEmpty.value = true;
  }

  /** 卸载清理（onUnmounted 调用）：兜底定时器 */
  function dispose(): void {
    window.clearTimeout(fallbackTimer);
  }

  return { showEmpty: showEmpty as Ref<boolean>, onAfterLeave, dispose };
}
