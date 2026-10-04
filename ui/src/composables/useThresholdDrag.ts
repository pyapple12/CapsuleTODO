// ===== 标题阈值拖拽（2026-09-30）：替换 data-tauri-drag-region 的"mousedown 即拖拽" =====
// 大标题是窗口拖动把手，同时是浮板关闭的点击目标（文档级板外点击关闭）。原机制
// mousedown 即异步启动系统拖拽循环，吞掉后续 click——浮板打开时单击标题永远关不上
// 板，快速连点偶尔赶在拖拽启动前才关上（2026-09-30 实测）。阈值分流：按下只记录
// 起点，移动超阈值才启动拖拽；单击不启动，click 正常派发，两职能按意图自动分流。
import { getCurrentInstance, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** 拖拽启动阈值（px）：位移低于此视为点击，不启动拖拽 */
const DRAG_THRESHOLD = 5;

/**
 * 标题阈值拖拽：把手元素按下记录起点，移动超阈值 invoke 系统拖拽，单击不拦截
 * click（浮板文档级关闭依赖它）。左键限定；preventDefault 仅抑制文本选择，
 * click 照常派发。返回显式 destroy——调用点在异步上下文时生命周期钩子不生效
 * （Vue 铁律，FIX013.5），由持有方在顶层 onUnmounted 显式调用；同步上下文
 * 调用时仍注册卸载钩子兜底
 */
export function useThresholdDrag(handle: HTMLElement): () => void {
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
  const destroy = (): void => {
    handle.removeEventListener("mousedown", onDown);
    detach();
  };

  handle.addEventListener("mousedown", onDown);
  if (getCurrentInstance()) onUnmounted(destroy); // 仅 setup 同步上下文可注册
  return destroy;
}
