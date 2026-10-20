// ===== 详情 note 写链（PATCH002.84，S2） =====
// 背景：拖图收场有“起拖 force save / 失焦 flush → note_reorder → 重拉”三条异步链；
// reorder 未 await 就重拉时，load 先读到旧序，界面回弹（首次拖回弹、二次成功）。
// 本模块只做“按入队顺序串行 + 失败隔离”，不感知 DOM / Tauri / 业务；
// S2 范围 = 只接拖拽收场链（在飞保存 → reorder → 重拉），不接粘贴/撤销/切源/关板
// （S3 全包写队列见 y.problems.md #24，未来做）。

/** 串行写链：前序（含失败）结束后才执行后序；失败只影响自身，不阻断后续 */
export interface NoteWriteChain {
  /** 入队一个任务；返回本任务结果（失败为 undefined，错误已落 console） */
  run<T>(task: () => Promise<T>): Promise<T | undefined>;
  /** 等当前队列全部收场（测试 / 收尾用） */
  idle(): Promise<void>;
}

/** 创建一个 note 写链（组件内单实例；当前只给详情拖拽收场用） */
export function useNoteWriteChain(): NoteWriteChain {
  let tail: Promise<unknown> = Promise.resolve();
  const run = <T>(task: () => Promise<T>): Promise<T | undefined> => {
    const next = tail.then(task, task).catch((err) => {
      // 任务失败不阻断后续（reorder 失败仍要重拉，显示库里现状）
      console.error("详情写链任务失败", err);
      return undefined;
    });
    tail = next;
    return next;
  };
  const idle = async (): Promise<void> => {
    await tail;
  };
  return { run, idle };
}
