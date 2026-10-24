/** HTML 图片标签判定（FIX017.3）：前端唯一事实源。
 * 只认 `<img>` / `<image>` 别名；大小写不敏感；标签名后必须是 ASCII 空白、`/` 或 `>`。
 * 与 Rust `html_paste::find_img_tag` 保持同一语法，避免三套判定漂移。 */

/** 统计 HTML 中的真实图片标签数量。 */
export function htmlImageCount(html: string): number {
  return (html.match(/<(?:image|img)(?=[ \t\n\f\r/>])/gi) ?? []).length;
}

/** HTML 是否包含真实图片标签。 */
export function htmlHasImage(html: string): boolean {
  return htmlImageCount(html) > 0;
}
