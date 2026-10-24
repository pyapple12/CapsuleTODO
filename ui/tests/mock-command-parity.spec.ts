import { describe, expect, it } from "vitest";

import { hasMockCommand, mockInvoke } from "../src/dev/mock-invoke";

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

  it("关键返回形状与新建语义对齐 Rust", async () => {
    // FIX015.35：命令名一致不够，DEV 还要镜像“新建无块/缺失 todo 空列表/粘贴块序”。
    const item = await mockInvoke<{ id: number; sort_order?: number }>("todo_add", {
      text: "mock 新条目",
    });
    // FIX018.13：新增条目必须与 Rust 一样排在未完成段顶部（种子已按迁移口径回填 0..n）。
    const list = await mockInvoke<Array<{ id: number; sort_order?: number }>>("todo_list");
    expect(list[0]?.id).toBe(item.id);
    expect(list[0]?.sort_order).toBe(-1);
    expect(await mockInvoke("note_blocks_list", { todoId: item.id })).toEqual([]);
    expect(await mockInvoke("note_blocks_list", { todoId: 999_999 })).toEqual([]);

    const saved = await mockInvoke<Array<{ id: number; kind: string; text: string }>>(
      "note_blocks_save",
      {
        todoId: item.id,
        blocks: [{ id: 0, kind: "Text", text: "正文" }],
      },
    );
    expect(saved).toHaveLength(1);
    expect(saved[0].id).toBeGreaterThan(0);

    const pasted = await mockInvoke<{
      status: string;
      blocks: Array<{ kind: string }>;
    }>("note_paste_image", {
      todoId: item.id,
      blockId: saved[0].id,
      before: "",
      after: "尾",
    });
    expect(pasted.status).toBe("image");
    expect(pasted.blocks.filter((block) => block.kind === "Image")).toHaveLength(1);
  });

  it("设置 mock 与 Rust 规范化/钳制一致", async () => {
    // FIX016.19：主键不能被修饰键排序带偏；0 必须钳到 1。
    expect(await mockInvoke("settings_set_bubble_hotkey", { combo: "Ctrl+Alt+C" })).toBe(
      "Ctrl+Alt+C",
    );
    await expect(
      mockInvoke("settings_set_bubble_hotkey", { combo: "Ctrl+Ctrl+A" }),
    ).rejects.toBeTruthy();
    // FIX017.10：CONTROL/SUPER 别名与 Rust parse 对齐。
    expect(await mockInvoke("settings_set_bubble_hotkey", { combo: "Control+Alt+C" })).toBe(
      "Ctrl+Alt+C",
    );
    expect(await mockInvoke("settings_set_bubble_hotkey", { combo: "Super+Shift+F12" })).toBe(
      "Shift+Win+F12",
    );
    await mockInvoke("settings_set_max_bubbles", { value: 0 });
    expect(await mockInvoke("settings_get_max_bubbles")).toBe(1);
  });

  it("HTML 多图粘贴按真实 img 数建块，不降级为单图", async () => {
    const item = await mockInvoke<{ id: number }>("todo_add", { text: "mock HTML" });
    const saved = await mockInvoke<Array<{ id: number }>>("note_blocks_save", {
      todoId: item.id,
      blocks: [{ id: 0, kind: "Text", text: "正文" }],
    });
    const out = await mockInvoke<{
      outcome: { status: string; blocks: Array<{ kind: string }> };
      pending: unknown[];
    }>("note_paste_html", {
      todoId: item.id,
      blockId: saved[0].id,
      before: "",
      after: "",
      html: '<p>前</p><img src="a"><img src="b"><p>后</p>',
    });
    expect(out.outcome.status).toBe("image");
    expect(out.outcome.blocks.filter((block) => block.kind === "Image")).toHaveLength(2);
    expect(out.pending).toEqual([]);
  });

  it("HTML 图片判定与共享 helper 一致（FIX017.3）", async () => {
    const item = await mockInvoke<{ id: number }>("todo_add", { text: "mock img tags" });
    const saved = await mockInvoke<Array<{ id: number }>>("note_blocks_save", {
      todoId: item.id,
      blocks: [{ id: 0, kind: "Text", text: "正文" }],
    });

    const fake = await mockInvoke<{ outcome: { status: string } }>("note_paste_html", {
      todoId: item.id,
      blockId: saved[0].id,
      before: "",
      after: "",
      html: '<img-foo src="a">',
    });
    expect(fake.outcome.status).toBe("no_image");

    const alias = await mockInvoke<{
      outcome: { status: string; blocks: Array<{ kind: string }> };
    }>("note_paste_html", {
      todoId: item.id,
      blockId: saved[0].id,
      before: "",
      after: "",
      html: '<IMAGE SRC="a">',
    });
    expect(alias.outcome.status).toBe("image");
    expect(alias.outcome.blocks.filter((block) => block.kind === "Image")).toHaveLength(1);
  });

  it("mock 空 before 顶替原块且末图补空文本（FIX017.10）", async () => {
    const item = await mockInvoke<{ id: number }>("todo_add", { text: "mock tail" });
    const saved = await mockInvoke<Array<{ id: number }>>("note_blocks_save", {
      todoId: item.id,
      blocks: [{ id: 0, kind: "Text", text: "正文" }],
    });
    const out = await mockInvoke<{
      blocks: Array<{ id: number; kind: string; text: string }>;
    }>("note_paste_image", {
      todoId: item.id,
      blockId: saved[0].id,
      before: "",
      after: "",
    });
    expect(out.blocks[0]).toMatchObject({ id: saved[0].id, kind: "Image" });
    expect(out.blocks[out.blocks.length - 1]).toMatchObject({ kind: "Text", text: "" });
  });

  it("mock 块保存拒绝重复 id、支持删图撤销与深拷贝（FIX017.10）", async () => {
    const item = await mockInvoke<{ id: number }>("todo_add", { text: "mock blocks" });
    const saved = await mockInvoke<Array<{ id: number }>>("note_blocks_save", {
      todoId: item.id,
      blocks: [{ id: 0, kind: "Text", text: "正文" }],
    });
    await expect(
      mockInvoke("note_blocks_save", {
        todoId: item.id,
        blocks: [
          { id: saved[0].id, kind: "Text", text: "甲" },
          { id: saved[0].id, kind: "Text", text: "乙" },
        ],
      }),
    ).rejects.toBeTruthy();

    const pasted = await mockInvoke<{
      blocks: Array<{ id: number; kind: string; text: string }>;
    }>("note_paste_image", {
      todoId: item.id,
      blockId: saved[0].id,
      before: "前",
      after: "后",
    });
    const image = pasted.blocks.find((block) => block.kind === "Image");
    expect(image).toBeTruthy();

    // 删图（软删除）→ 撤销（旧 id 复活）
    const without = pasted.blocks.filter((block) => block.kind !== "Image");
    await mockInvoke("note_blocks_save", { todoId: item.id, blocks: without });
    const revived = await mockInvoke<Array<{ id: number; kind: string }>>("note_blocks_save", {
      todoId: item.id,
      blocks: pasted.blocks,
    });
    expect(revived.some((block) => block.id === image!.id && block.kind === "Image")).toBe(true);

    // 返回深拷贝：改返回值不得污染 mock 状态
    pasted.blocks[0].text = "被改坏";
    const listed = await mockInvoke<Array<{ text: string }>>("note_blocks_list", {
      todoId: item.id,
    });
    expect(listed[0].text).not.toBe("被改坏");
  });
});
