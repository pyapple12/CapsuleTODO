/**
 * FIX015.34：有效聚焦态 = 系统焦点 || 原生拖拽标记。
 * 原生窗口拖拽会给成对的失焦/回焦事件；拖拽期间必须继续按聚焦绘制纱态。
 */
export function isEffectivelyFocused(focused: boolean, dragging: boolean): boolean {
  return focused || dragging;
}
