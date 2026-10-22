import { describe, expect, it } from "vitest";

import { hasMockCommand } from "../src/dev/mock-invoke";

/** FIX014.27：直接读取真机注册块，避免“手写常量 + mock”两边一起漂移。 */
const libSource = Object.values(
  import.meta.glob("../../core/src/lib.rs", {
    query: "?raw",
    import: "default",
    eager: true,
  }),
)[0] as string | undefined;

/** 从 generate_handler! 块提取命令名（只匹配该块，避免误收同文件其它调用）。 */
function commandsFromLib(source: string): string[] {
  const handler = source.match(/generate_handler!\[([\s\S]*?)\]\)/)?.[1] ?? "";
  return [...handler.matchAll(/commands::[a-zA-Z0-9_]+::([a-zA-Z0-9_]+)/g)].map(
    (match) => match[1],
  );
}

/** 与 core/src/lib.rs 的 generate_handler! 注册块同步（新增命令时同时补此表与 mock）。 */
const REGISTERED_COMMANDS = [
  "todo_add",
  "set_board_backdrop",
  "todo_toggle",
  "todo_rename",
  "todo_remove",
  "todo_list",
  "todo_archive_list",
  "todo_reorder",
  "note_blocks_list",
  "note_blocks_save",
  "note_blocks_purge_deleted",
  "note_paste_image",
  "note_reorder",
  "note_open_image",
  "note_copy_selection",
  "note_paste_html",
  "note_fetch_pending_images",
  "bubble_capture",
  "bubble_list",
  "bubble_get_image_path",
  "bubble_open_image",
  "bubble_copy",
  "bubble_remove",
  "bubble_clear",
  "bubble_reorder",
  "whiteboard_load",
  "whiteboard_save",
  "settings_get_max_bubbles",
  "settings_get_bubble_max_limit",
  "settings_set_max_bubbles",
  "settings_get_bubble_hotkey",
  "settings_set_bubble_hotkey",
  "settings_suspend_bubble_hotkey",
  "settings_resume_bubble_hotkey",
  "settings_get_always_on_top",
  "settings_set_always_on_top",
  "settings_get_snap_to_edge",
  "settings_set_snap_to_edge",
  "settings_get_theme",
  "settings_set_theme",
  "tray_preview_resize",
  "tray_menu_action",
  "main_hide_to_tray",
] as const;

describe("mock 命令覆盖（FIX014.27）", () => {
  it("静态表与 lib.rs generate_handler 完全一致", () => {
    expect(libSource).toBeTypeOf("string");
    const actual = commandsFromLib(libSource ?? "");
    expect(actual.length).toBeGreaterThan(0);
    expect([...actual].sort()).toEqual([...REGISTERED_COMMANDS].sort());
  });

  it("真机注册的每个命令都有 DEV mock 路由", () => {
    const missing = REGISTERED_COMMANDS.filter((name) => !hasMockCommand(name));
    expect(missing).toEqual([]);
  });
});
