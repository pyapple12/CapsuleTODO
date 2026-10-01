import type { Ref } from "vue";

import { useBoardRead, type BoardReadOptions } from "./useBoardRead";
import { useGlassBar, type GlassBarOptions } from "./useGlassBar";

// ===== 整板阅读套件挂载管理（FIX005.24 三组件收敛）：TransitionGroup 的 template
// ref 指向组件实例——真实 UL 须经 $el 取（PL011 同教训）；dataset.mounted 防重挂；
// composable 在异步上下文挂载时生命周期钩子失效（无组件实例），destroy 由调用方
// 持有，重挂/卸载时显式调 unmount。TodoList/BubblesView/ArchiveOverlay 三页共用，
// 差异全部走选项（boardRead 选项 / glassBar 选项 / 防重挂标记键）

/** 套件选项：boardRead/glassBar 透传各自配置 + 防重挂标记键（三页各异） */
export interface ScrollKitOptions {
  /** useBoardRead 选项（行选择器/拖拽抑制/罩位移量等，各页不同） */
  boardRead: BoardReadOptions;
  /** useGlassBar 选项（inset/anchor/right 等，各页不同） */
  glassBar: GlassBarOptions;
  /**
   * 防重挂 dataset 标记（标记键 + 卸载时按 id 摘标记的宿主元素 id，三页各异——
   * 沿原实现：标记打在 UL 上，卸载经 getElementById 按 id 摘）
   */
  mountedFlag: { key: string; hostId: string };
}

/**
 * 整板阅读套件（玻璃滑杆 + 整板阅读）：挂载/拆卸显式管理
 * @param listEl TransitionGroup template ref（真实 UL 经 $el 解引用）
 * @param opts 套件选项（boardRead/glassBar 配置 + mountedFlag）
 * @returns mount（ul 在 DOM 后调用，防重挂）/ unmount（监听/observer/三角/浮钮全清）
 */
export function useScrollKit(
  listEl: Ref<unknown>,
  opts: ScrollKitOptions,
): { mount: () => void; unmount: () => void } {
  let boardRead: { destroy: () => void } | null = null;
  let glassBar: { destroy: () => void } | null = null;

  /** 挂套件（ul 已在 DOM 时执行；dataset[opts.mountedFlag.key] 防重挂） */
  function mount(): void {
    const ul =
      (listEl.value as { $el?: HTMLElement } | null)?.$el ?? (listEl.value as HTMLElement | null);
    if (!ul || boardRead || ul.dataset[opts.mountedFlag.key] === "1") return;
    ul.dataset[opts.mountedFlag.key] = "1";
    boardRead = useBoardRead(ul, opts.boardRead);
    glassBar = useGlassBar(ul, opts.glassBar);
  }

  /** 拆旧挂新前显式清理（监听/observer/三角/浮钮全清 + 摘防重挂标记） */
  function unmount(): void {
    boardRead?.destroy();
    glassBar?.destroy();
    boardRead = null;
    glassBar = null;
    // FIX006.9 摘标记与 mount 打标记走同一选项键（原硬编码 mounted 与接口承诺
    // 不一致——换 key 的调用方会摘不掉标记，下次挂载被防重挂守卫挡住）
    delete document.getElementById(opts.mountedFlag.hostId)?.dataset[opts.mountedFlag.key];
  }

  return { mount, unmount };
}
