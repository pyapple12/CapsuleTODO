// ===== 错误行自动隐藏（PATCH002.66） =====
// 只服务 #1 AddBar（10s）与 #2 气泡（5s）；#3 热键 / #4 白板是常驻，不用本件。
// 计时器随 effect scope 清理；UI 只存短句，完整 Rust 错误由调用方落 console / todo.log。
import { onScopeDispose, ref } from "vue";

/** 创建一个自动隐藏的错误消息（show 重新计时；hide 立即清） */
export function useAutoHideError(hideMs: number) {
  const message = ref("");
  let timer = 0;

  /** 立即清除消息并停表（成功路径 / 手动关闭） */
  const hide = (): void => {
    window.clearTimeout(timer);
    timer = 0;
    message.value = "";
  };

  /** 显示一条短消息，并在 hideMs 后自动清空（重复 show 重新计时） */
  const show = (text: string): void => {
    message.value = text;
    window.clearTimeout(timer);
    timer = window.setTimeout(() => {
      timer = 0;
      message.value = "";
    }, hideMs);
  };

  onScopeDispose(() => window.clearTimeout(timer));
  return { message, show, hide };
}
