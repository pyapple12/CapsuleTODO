import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const archiveCss = readFileSync("ui/src/styles/archive.css", "utf8");
const detailCss = readFileSync("ui/src/styles/detail.css", "utf8");

describe("浮板收起态 pointer-events（FIX017.11）", () => {
  it("archive/settings 收起态 none、open 态 auto", () => {
    // 收起态透明根不得参与命中；0.4s visibility 退场只负责视觉。
    expect(archiveCss).toMatch(
      /\.archive-overlay,\s*\.settings-overlay\s*\{[^}]*pointer-events:\s*none/,
    );
    expect(archiveCss).toMatch(
      /\.archive-overlay\.open,\s*\.settings-overlay\.open\s*\{[^}]*pointer-events:\s*auto/,
    );
  });

  it("detail 收起态 none、open 态 auto", () => {
    expect(detailCss).toMatch(/\.detail-overlay\s*\{[^}]*pointer-events:\s*none/);
    expect(detailCss).toMatch(/\.detail-overlay\.open\s*\{[^}]*pointer-events:\s*auto/);
  });
});
