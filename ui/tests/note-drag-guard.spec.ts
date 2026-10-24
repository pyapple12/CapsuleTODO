import { afterEach, describe, expect, it } from "vitest";

import { rowMaskDead } from "../src/composables/useMaskDead";
import { isNoteDragTarget } from "../src/composables/useDragReorder";

describe("详情拖拽边界（FIX015.32）", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("详情 note 不参与三列表罩死", () => {
    const list = document.createElement("div");
    list.id = "detail-note";
    list.className = "board-read";
    const row = document.createElement("div");
    row.className = "note-block";
    list.append(row);
    document.body.append(list);

    expect(rowMaskDead(row)).toBe(false);
  });

  it("三列表顶部侵入仍按 38% 罩死", () => {
    // FIX017.11：todo/bubble/archive 三个 listId 都必须命中罩死。
    for (const [listId, rowClass, itemClass] of [
      ["todo-active", "todo-row", "todo-item"],
      ["bubble-list", "bubble-row", "bubble-row"],
      ["archive-list", "todo-row", "todo-item"],
    ]) {
      document.body.innerHTML = "";
      const list = document.createElement("div");
      list.id = listId;
      list.className = "board-read";
      list.getBoundingClientRect = () =>
        ({ top: 0, bottom: 100, left: 0, right: 100, height: 100, width: 100 }) as DOMRect;
      const item = document.createElement("div");
      item.className = itemClass;
      const row = document.createElement("div");
      row.className = rowClass;
      row.getBoundingClientRect = () =>
        ({ top: -20, bottom: 20, left: 0, right: 100, height: 40, width: 100 }) as DOMRect;
      item.append(row);
      list.append(item);
      document.body.append(list);

      expect(rowMaskDead(row), listId).toBe(true);
    }
  });

  it("图块可拖，但删除按钮不可作为起拖目标", () => {
    const image = document.createElement("div");
    image.className = "note-block-image";
    const del = document.createElement("button");
    del.className = "note-block-del";
    image.append(del);

    expect(isNoteDragTarget(image)).toBe(true);
    expect(isNoteDragTarget(del)).toBe(false);
  });
});
