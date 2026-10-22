import { describe, expect, it } from "vitest";

import { latestOnly } from "../src/composables/latestOnly";

/** 可手动控制 resolve/reject 时机的 Promise，用于制造“旧请求晚到”。 */
function deferred<T>(): {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason: unknown) => void;
} {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

describe("latestOnly（FIX014.31）", () => {
  it("只应用最后一次请求的结果，旧响应不得覆盖新值", async () => {
    const applied: number[] = [];
    const load = latestOnly<number>((value) => applied.push(value));
    const old = deferred<number>();
    const fresh = deferred<number>();

    const oldTask = load(() => old.promise);
    const freshTask = load(() => fresh.promise);

    fresh.resolve(2);
    await freshTask;
    expect(applied).toEqual([2]);

    old.resolve(1);
    await oldTask;
    expect(applied).toEqual([2]);
  });

  it("最新请求失败时，旧响应也不得被当成新值应用", async () => {
    const applied: number[] = [];
    const load = latestOnly<number>((value) => applied.push(value));
    const old = deferred<number>();
    const fresh = deferred<number>();

    const oldTask = load(() => old.promise);
    const freshTask = load(() => fresh.promise).catch(() => undefined);

    fresh.reject(new Error("latest failed"));
    await freshTask;
    expect(applied).toEqual([]);

    old.resolve(1);
    await oldTask;
    expect(applied).toEqual([]);
  });
});
