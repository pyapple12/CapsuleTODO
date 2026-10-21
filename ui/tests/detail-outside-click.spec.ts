import { describe, expect, it } from "vitest";

import { shouldCloseDetailOnOutsideClick } from "../src/composables/detailOutsideClick";

describe("shouldCloseDetailOnOutsideClick（PATCH002.88）", () => {
  it("板外新按下 + 板外空白 → 关板", () => {
    expect(
      shouldCloseDetailOnOutsideClick({
        startedInside: false,
        targetInside: false,
        targetIsRow: false,
      }),
    ).toBe(true);
  });

  it("click target 在板内 → 不关板", () => {
    expect(
      shouldCloseDetailOnOutsideClick({
        startedInside: false,
        targetInside: true,
        targetIsRow: false,
      }),
    ).toBe(false);
  });

  it("click target 是清单行或气泡行 → 不关板", () => {
    expect(
      shouldCloseDetailOnOutsideClick({
        startedInside: false,
        targetInside: false,
        targetIsRow: true,
      }),
    ).toBe(false);
  });

  it("板内起手、板外空白松手 → 不关板", () => {
    expect(
      shouldCloseDetailOnOutsideClick({
        startedInside: true,
        targetInside: false,
        targetIsRow: false,
      }),
    ).toBe(false);
  });

  it("板内起手、板外行上松手 → 不关板", () => {
    expect(
      shouldCloseDetailOnOutsideClick({
        startedInside: true,
        targetInside: false,
        targetIsRow: true,
      }),
    ).toBe(false);
  });
});
