import { describe, expect, it, vi } from "vitest";

import { useNoteWriteChain } from "../src/composables/useNoteWriteChain";

/** 等宏任务，让链上的微任务全部落定 */
const flush = (): Promise<void> => new Promise<void>((resolve) => setTimeout(resolve, 0));

/** 手动控制 resolve 的 Promise（模拟慢保存 / 慢 reorder） */
function deferred(): { promise: Promise<void>; resolve: () => void } {
  let resolve!: () => void;
  const promise = new Promise<void>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

describe("useNoteWriteChain（PATCH002.84 / S2）", () => {
  it("保存未完成前不执行 reorder，reorder 完成前不重拉", async () => {
    const chain = useNoteWriteChain();
    const calls: string[] = [];
    const save = deferred();
    const saveTask = chain.run(async () => {
      calls.push("save:start");
      await save.promise;
      calls.push("save:end");
    });
    const reorderTask = chain.run(async () => {
      calls.push("reorder");
    });
    const reloadTask = chain.run(async () => {
      calls.push("reload");
    });
    await flush();
    expect(calls).toEqual(["save:start"]);
    save.resolve();
    await Promise.all([saveTask, reorderTask, reloadTask]);
    expect(calls).toEqual(["save:start", "save:end", "reorder", "reload"]);
  });

  it("任务失败不阻断后续，idle 等全部收场", async () => {
    const chain = useNoteWriteChain();
    const calls: string[] = [];
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    const bad = chain.run(async () => {
      calls.push("bad");
      throw new Error("save fail");
    });
    const good = chain.run(async () => {
      calls.push("good");
      return 42;
    });
    await chain.idle();
    await expect(bad).resolves.toBeUndefined();
    await expect(good).resolves.toBe(42);
    expect(calls).toEqual(["bad", "good"]);
    spy.mockRestore();
  });

  it("S2 收场：在飞保存全部收场 → reorder → 重拉", async () => {
    const chain = useNoteWriteChain();
    const calls: string[] = [];
    const saves = new Set<Promise<void>>();
    const saveA = deferred();
    const saveB = deferred();
    const track = (name: string, d: { promise: Promise<void> }): void => {
      const p = (async () => {
        calls.push(`${name}:start`);
        await d.promise;
        calls.push(`${name}:end`);
      })();
      saves.add(p);
      void p.finally(() => saves.delete(p));
    };
    track("A", saveA);
    track("B", saveB);
    const commit = chain.run(async () => {
      await Promise.all([...saves]);
      calls.push("reorder");
    });
    const reload = chain.run(async () => {
      calls.push("reload");
    });
    await flush();
    expect(calls).toEqual(["A:start", "B:start"]);
    saveB.resolve();
    await flush();
    expect(calls).toEqual(["A:start", "B:start", "B:end"]);
    saveA.resolve();
    await Promise.all([commit, reload]);
    expect(calls).toEqual(["A:start", "B:start", "B:end", "A:end", "reorder", "reload"]);
  });
});
