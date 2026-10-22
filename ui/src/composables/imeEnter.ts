/** FIX014.30：输入法组字中的 Enter 只用于选词，不得提交业务动作。 */
export function shouldIgnoreImeEnter(
  event: Pick<KeyboardEvent, "isComposing" | "keyCode">,
): boolean {
  return event.isComposing || event.keyCode === 229;
}
