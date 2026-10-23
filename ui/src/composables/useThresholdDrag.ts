// ===== 标题阈值拖拽（2026-09-30）：替换 data-tauri-drag-region 的"mousedown 即拖拽" =====
// 大标题是窗口拖动把手，同时是浮板关闭的点击目标（文档级板外点击关闭）。原机制
// mousedown 即异步启动系统拖拽循环，吞掉后续 click——浮板打开时单击标题永远关不上
// 板，快速连点偶尔赶在拖拽启动前才关上（2026-09-30 实测）。阈值分流：按下只记录
// 起点，移动超阈值才启动拖拽；单击不启动，click 正常派发，两职能按意图自动分流。
import { getCurrentInstance, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** 拖拽启动阈值（px）：位移低于此视为点击，不启动拖拽 */
const DRAG_THRESHOLD = 5;

/** 给把手元素装上阈值拖拽（左键限定；`preventDefault` 仅抑制文本选择，不拦 click）。
 * `options.onDragStart`（PATCH002.61）：在 `startDragging()` **之前**触发——原生拖拽
 * 循环会给出成对的 `Focused(false/true)` 抖动，调用方据此在抖动发生**前**置"拖拽中"
 * 标记，屏蔽系统背板 / 纱态被误摘（"磨砂闪烁"）。 */
export function useThresholdDrag(
  handle: HTMLElement,
  options?: { onDragStart?: () => void; onDragStartError?: (err: unknown) => void },
): () => void {
  let startX = 0;
  let startY = 0;
  let tracking = false;

  /** 位移超阈值 → 启动系统拖拽并解绑（本次手势只触发一次） */
  const onMove = (e: MouseEvent): void => {
    if (!tracking) return;
    // FIX015.31：窗外松手/失焦后残留的 tracking 不能在下一次无按键移动时启动拖窗。
    if ((e.buttons & 1) === 0) {
      stop();
      return;
    }
    if (Math.hypot(e.clientX - startX, e.clientY - startY) < DRAG_THRESHOLD) return;
    stop();
    options?.onDragStart?.();
    void getCurrentWindow()
      .startDragging()
      .catch((err) => {
        options?.onDragStartError?.(err);
        console.error("窗口拖拽启动失败", err);
      });
  };
  const onUp = (): void => {
    stop();
  };
  const onBlur = (): void => {
    stop();
  };
  const onDown = (e: MouseEvent): void => {
    if (e.button !== 0) return;
    stop(); // 幂等清掉上一次可能残留的手势
    e.preventDefault();
    startX = e.clientX;
    startY = e.clientY;
    tracking = true;
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    window.addEventListener("blur", onBlur);
  };
  const detach = (): void => {
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
    window.removeEventListener("blur", onBlur);
  };
  const stop = (): void => {
    tracking = false;
    detach();
  };
  const destroy = (): void => {
    handle.removeEventListener("mousedown", onDown);
    stop();
  };

  handle.addEventListener("mousedown", onDown);
  if (getCurrentInstance()) onUnmounted(destroy); // 仅 setup 同步上下文可注册
  return destroy;
}
