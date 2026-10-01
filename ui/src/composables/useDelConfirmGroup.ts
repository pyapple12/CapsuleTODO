import { onUnmounted, ref } from "vue";

import { registerDelConfirms } from "./delConfirmBus";

// ===== 删除确认单实例收口（FIX005.24 三组件收敛）：confirmingId + DelButton
// expose 句柄表 + 收口总线注册。首点进确认态（旧确认行立即回退）、再点执行、
// mouseleave 回退、板开合/页签切换批量收口（A1/A2）。TodoList/ArchiveOverlay
// （TodoItem）与 BubblesView（BubbleItem）共用，泛型参数化条目类型

/** DelButton 组件 expose 的最小接口（rollBack 清执行窗） */
interface DelButtonLike {
  rollBack: () => void;
}

/**
 * 删除确认组状态机（泛型：条目 id 类型）
 * @param onConfirmSecondClick 再点执行回调（组件内做 IPC + 刷新；确认态已先摘除）
 * @returns 确认态与四事件处理器（模板/DelButton 事件绑定）+ 卸载清理
 */
export function useDelConfirmGroup<T>(onConfirmSecondClick: (item: T) => Promise<void>) {
  const confirmingId = ref<number | null>(null);

  /** 行组件的 rollBack expose 句柄（id → 组件实例） */
  const delRefs = new Map<number, DelButtonLike>();

  /** DelButton :ref 绑定（挂载登记 / 卸载除名） */
  function setDelRef(id: number, el: DelButtonLike | null): void {
    if (el) delRefs.set(id, el);
    else delRefs.delete(id);
  }

  /** 首点进确认态：单实例收口（旧确认行立即回退） */
  function onPress(item: T & { id: number }): void {
    if (confirmingId.value != null && confirmingId.value !== item.id) {
      delRefs.get(confirmingId.value)?.rollBack();
    }
    confirmingId.value = item.id;
  }

  /** 再点执行：摘确认态 + 委托组件回调（DelButton 已延迟 200ms 播完脉冲） */
  async function onConfirm(item: T): Promise<void> {
    confirmingId.value = null;
    await onConfirmSecondClick(item);
  }

  /** 确认态鼠标离开即回退（A1：DelButton mouseleave 上抛，父级摘 confirming） */
  function onCancel(item: T & { id: number }): void {
    if (confirmingId.value === item.id) confirmingId.value = null;
  }

  /** 批量收口（收口总线回调体：rollBack 清执行窗 + 摘确认态） */
  function rollBackAll(): void {
    if (confirmingId.value != null) {
      delRefs.get(confirmingId.value)?.rollBack();
      confirmingId.value = null;
    }
  }

  // 收口总线注册（A2 = design rollbackDelConfirms）：归档板开合/页签切换时批量摘
  // 本列表的未决确认——行被遮盖后 mouseout 不再来，不收口会在板收/切回后红态复活
  const unregister = registerDelConfirms(rollBackAll);
  onUnmounted(() => {
    unregister();
  });

  return { confirmingId, setDelRef, onPress, onConfirm, onCancel };
}
