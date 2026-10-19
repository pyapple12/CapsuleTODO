import { describe, expect, it } from "vitest";

import { bindOverlayState, boardOpen, syncVeils } from "../src/composables/useVeils";

/**
 * `boardOpen` 是 PATCH002.60 系统背板按需挂摘的**唯一判据源**（与帘联动同源）。
 * 断言：三浮板任一开 ⇒ true；全关 ⇒ false。
 */
describe("useVeils.boardOpen（背板判据源）", () => {
  it("板开合刷新 boardOpen", () => {
    let archive = false;
    let settings = false;
    bindOverlayState("board", () => archive);
    bindOverlayState("settings", () => settings);

    syncVeils();
    expect(boardOpen.value).toBe(false);

    archive = true;
    syncVeils();
    expect(boardOpen.value).toBe(true);

    archive = false;
    settings = true;
    syncVeils();
    expect(boardOpen.value).toBe(true);

    settings = false;
    syncVeils();
    expect(boardOpen.value).toBe(false);
  });
});
