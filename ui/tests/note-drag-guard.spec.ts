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
