// ===== 标题阈值拖拽（2026-09-30）：替换 data-tauri-drag-region 的"mousedown 即拖拽" =====
// 大标题是窗口拖动把手，同时是浮板关闭的点击目标（文档级板外点击关闭）。原机制
// mousedown 即异步启动系统拖拽循环，吞掉后续 click——浮板打开时单击标题永远关不上
// 板，快速连点偶尔赶在拖拽启动前才关上（2026-09-30 实测）。阈值分流：按下只记录
// 起点，移动超阈值才启动拖拽；单击不启动，click 正常派发，两职能按意图自动分流。
import { onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** 拖拽启动阈值（px）：位移低于此视为点击，不启动拖拽 */
const DRAG_THRESHOLD = 5;

/**
 * 标题阈值拖拽：把手元素按下记录起点，移动超阈值 invoke 系统拖拽，单击不拦截
 * click（浮板文档级关闭依赖它）。左键限定；preventDefault 仅抑制文本选择，
 * click 照常派发。卸载自动清监听
 */
export function useThresholdDrag(handle: HTMLElement): void {
  let startX = 0;
  let startY = 0;
  let tracking = false;

  const onMove = (e: MouseEvent): void => {
    if (!tracking) return;
    if (Math.hypot(e.clientX - startX, e.clientY - startY) < DRAG_THRESHOLD) return;
    tracking = false;
    detach();
    void getCurrentWindow()
      .startDragging()
      .catch((err) => console.error("窗口拖拽启动失败", err));
  };
  const onUp = (): void => {
    tracking = false;
    detach();
  };
  const onDown = (e: MouseEvent): void => {
    if (e.button !== 0) return;
    e.preventDefault();
    startX = e.clientX;
    startY = e.clientY;
    tracking = true;
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };
  const detach = (): void => {
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
  };

  handle.addEventListener("mousedown", onDown);
  onUnmounted(() => {
    handle.removeEventListener("mousedown", onDown);
    detach();
  });
}
