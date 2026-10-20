// ===== 详情选区状态机（PATCH002.77）：原生 / 跨块 / 回源 / 穿越 =====
// 设计沿 `core/src/tray.rs` 的“纯 step + 单测”范式：状态迁移与 DOM 副作用分离。
// 组件只负责采样（hitTest / overlay / nextTick）与原生选区读写，本模块只做状态判断。
// **单一事实源**：phase + anchor/focus + sourceIndex/nativeHideKey + dragAnchor；
// 任何“回源块/穿越源块”都不重新反推锚点（dragAnchor 手势内持久）。
import { computed, reactive, toRef } from "vue";

/** 块内字符落点（index = 块下标；offset = 块内字符偏移） */
export interface SelPoint {
  index: number;
  offset: number;
}

/** 选区相位：idle 无选区 / pressed 已按下 / native 块内原生 / cross 跨块自绘 / selected 自绘已定格 */
export type SelPhase = "idle" | "pressed" | "native" | "cross" | "selected";

/** 状态机状态（全部为纯数据；DOM 引用不进来） */
export interface NoteSelectionState {
  phase: SelPhase;
  /** 自绘锚点（cross/selected / Shift 待扩） */
  anchor: SelPoint | null;
  /** 自绘焦点 */
  focus: SelPoint | null;
  /** 本次鼠标手势的拖拽源块 */
  sourceIndex: number | null;
  /** 需要压制原生选区的块 key（拖拽=源块 / Shift=锚点块） */
  nativeHideKey: string | null;
  /** 手势内持久锚点（回源/穿越不反推） */
  dragAnchor: SelPoint | null;
  /** 按下点坐标（锚点精确命中用） */
  down: { x: number; y: number } | null;
  /** 指针所在块（覆盖层挂载/命中用） */
  hoverIndex: number;
  /** 覆盖层临时接管（nextTick 采样期） */
  sampling: boolean;
  /** 图块单击选中（与文本选区互斥） */
  imageKey: string | null;
  /** 图块 Shift 手势标记（click 消费） */
  suppressImageClick: boolean;
}

/** DOM 副作用端口（由组件注入；测试用记录器） */
export interface NoteSelectionHooks {
  /** 把除 exceptKey 外的文本域原生区间收成光标 */
  collapseAll(exceptKey?: string): void;
  /** 把指定文本域原生选区收成光标 */
  collapseTo(key: string, offset: number): void;
}

export type NoteSelectionEvent =
  | { type: "press"; index: number; down: { x: number; y: number } }
  | { type: "anchor"; point: SelPoint }
  | { type: "moveIn"; sourceKey: string }
  | { type: "beginCross"; sourceKey: string }
  | { type: "cross"; anchor: SelPoint; focus: SelPoint; hideKey: string }
  | { type: "focus"; focus: SelPoint }
  | { type: "exit"; sourceKey: string }
  | { type: "up" }
  | { type: "shiftClick"; focus: SelPoint; hideKey: string; anchor?: SelPoint }
  | { type: "shiftExit"; key: string; offset: number }
  | { type: "selectAll"; from: SelPoint; to: SelPoint }
  | { type: "clear" }
  | { type: "clearAll" }
  | { type: "selectImage"; key: string }
  | { type: "clearImage" }
  | { type: "load" }
  | { type: "sample"; on: boolean }
  | { type: "hover"; index: number }
  | { type: "suppress"; on: boolean };

/** 初始状态（每次开板/装载复位） */
export function initialNoteSelectionState(): NoteSelectionState {
  return {
    phase: "idle",
    anchor: null,
    focus: null,
    sourceIndex: null,
    nativeHideKey: null,
    dragAnchor: null,
    down: null,
    hoverIndex: -1,
    sampling: false,
    imageKey: null,
    suppressImageClick: false,
  };
}

/**
 * 纯迁移函数：输入旧状态 + 事件 + 副作用端口 → 新状态（不改入参）。
 * 副作用（收原生选区）在迁移点同步调用；DOM 采样仍由调用方完成。
 */
export function noteSelectionStep(
  prev: NoteSelectionState,
  event: NoteSelectionEvent,
  hooks: NoteSelectionHooks,
): NoteSelectionState {
  const s: NoteSelectionState = { ...prev };
  switch (event.type) {
    case "press":
      s.phase = "pressed";
      s.anchor = null;
      s.focus = null;
      s.sourceIndex = event.index;
      s.nativeHideKey = null;
      s.dragAnchor = null;
      s.down = event.down;
      s.hoverIndex = -1;
      s.sampling = false;
      s.imageKey = null;
      s.suppressImageClick = false;
      hooks.collapseAll();
      break;
    case "anchor":
      s.anchor = event.point;
      break;
    case "moveIn":
      if (s.phase === "cross" || s.phase === "selected") {
        s.phase = "native";
        s.anchor = null;
        s.focus = null;
        s.nativeHideKey = null;
        s.sampling = false;
        s.hoverIndex = -1;
        hooks.collapseAll(event.sourceKey);
      } else {
        s.phase = "native";
      }
      break;
    case "beginCross":
      // 首次越块：先挂覆盖层/接管指针（anchor/focus 留白，等下一拍采样）
      s.phase = "cross";
      s.nativeHideKey = event.sourceKey;
      s.sampling = true;
      break;
    case "cross":
      s.phase = "cross";
      s.anchor = event.anchor;
      s.focus = event.focus;
      s.dragAnchor = s.dragAnchor ?? event.anchor;
      s.nativeHideKey = event.hideKey;
      s.hoverIndex = event.focus.index;
      s.sampling = false;
      hooks.collapseAll(event.hideKey);
      break;
    case "focus":
      if (s.phase === "cross") {
        s.focus = event.focus;
        s.hoverIndex = event.focus.index;
      }
      break;
    case "exit":
      s.phase = s.sourceIndex != null ? "native" : "idle";
      s.anchor = null;
      s.focus = null;
      s.nativeHideKey = null;
      s.sampling = false;
      s.hoverIndex = -1;
      hooks.collapseAll(event.sourceKey);
      break;
    case "up":
      if (s.phase === "cross") {
        s.phase = "selected";
        s.sourceIndex = null;
        s.down = null;
        s.dragAnchor = null;
        s.hoverIndex = -1;
        s.sampling = false;
      } else if (s.phase === "pressed" || s.phase === "native") {
        s.phase = "idle";
        s.anchor = null;
        s.focus = null;
        s.sourceIndex = null;
        s.nativeHideKey = null;
        s.down = null;
        s.dragAnchor = null;
        s.hoverIndex = -1;
        s.sampling = false;
      }
      break;
    case "shiftClick":
      if (event.anchor && !s.anchor) s.anchor = event.anchor;
      s.phase = "cross";
      s.focus = event.focus;
      s.nativeHideKey = event.hideKey;
      s.hoverIndex = event.focus.index;
      s.sampling = false;
      s.sourceIndex = null;
      s.dragAnchor = null;
      s.imageKey = null; // PATCH002.77：文本自绘接管，图块选中环必须退场（单一可见选中）
      hooks.collapseAll(event.hideKey);
      break;
    case "shiftExit":
      s.phase = "idle";
      s.anchor = null;
      s.focus = null;
      s.nativeHideKey = null;
      s.sourceIndex = null;
      s.down = null;
      s.dragAnchor = null;
      s.hoverIndex = -1;
      s.sampling = false;
      s.imageKey = null;
      hooks.collapseAll(event.key);
      hooks.collapseTo(event.key, event.offset);
      break;
    case "selectAll":
      s.phase = "selected";
      s.anchor = event.from;
      s.focus = event.to;
      s.sourceIndex = null;
      s.nativeHideKey = null;
      s.down = null;
      s.dragAnchor = null;
      s.hoverIndex = -1;
      s.sampling = false;
      s.imageKey = null;
      hooks.collapseAll();
      break;
    case "clear":
      s.phase = "idle";
      s.anchor = null;
      s.focus = null;
      s.sourceIndex = null;
      s.nativeHideKey = null;
      s.down = null;
      s.dragAnchor = null;
      s.hoverIndex = -1;
      s.sampling = false;
      break;
    case "clearAll":
      s.phase = "idle";
      s.anchor = null;
      s.focus = null;
      s.sourceIndex = null;
      s.nativeHideKey = null;
      s.down = null;
      s.dragAnchor = null;
      s.hoverIndex = -1;
      s.sampling = false;
      s.imageKey = null;
      hooks.collapseAll();
      break;
    case "selectImage":
      s.phase = "idle";
      s.anchor = null;
      s.focus = null;
      s.sourceIndex = null;
      s.nativeHideKey = null;
      s.down = null;
      s.dragAnchor = null;
      s.hoverIndex = -1;
      s.sampling = false;
      s.imageKey = event.key;
      hooks.collapseAll();
      break;
    case "clearImage":
      s.imageKey = null;
      break;
    case "load":
      Object.assign(s, initialNoteSelectionState());
      hooks.collapseAll();
      break;
    case "sample":
      s.sampling = event.on;
      break;
    case "hover":
      s.hoverIndex = event.index;
      break;
    case "suppress":
      s.suppressImageClick = event.on;
      break;
  }
  return s;
}

/**
 * Vue 包装：reactive 状态 + 迁移派发 + 派生选区。
 * @param hooks 原生选区副作用端口（组件注入）
 */
export function useNoteSelection(hooks: NoteSelectionHooks) {
  const state = reactive<NoteSelectionState>(initialNoteSelectionState());
  const dispatch = (event: NoteSelectionEvent): void => {
    Object.assign(state, noteSelectionStep(state, event, hooks));
  };

  const anchor = toRef(state, "anchor");
  const focus = toRef(state, "focus");
  const hideKey = toRef(state, "nativeHideKey");
  const hoverIndex = toRef(state, "hoverIndex");
  const imageKey = toRef(state, "imageKey");
  const dragAnchor = toRef(state, "dragAnchor");
  const suppressImageClick = toRef(state, "suppressImageClick");
  const sourceIndex = computed(() => state.sourceIndex ?? -1);
  const down = computed(() => state.down);
  const sampling = computed(() => state.sampling);
  /** 跨块自绘区（仅 cross/selected；单块恒 null） */
  const range = computed<{ from: SelPoint; to: SelPoint } | null>(() => {
    const a = state.anchor;
    const f = state.focus;
    if (!a || !f || a.index === f.index) return null;
    if (state.phase !== "cross" && state.phase !== "selected") return null;
    const forward = a.index < f.index || (a.index === f.index && a.offset <= f.offset);
    return forward ? { from: a, to: f } : { from: f, to: a };
  });
  /** 覆盖层接管中（拖拽/采样） */
  const crossing = computed(() => state.phase === "cross" || state.sampling);

  return {
    state,
    anchor,
    focus,
    range,
    crossing,
    hideKey,
    hoverIndex,
    imageKey,
    dragAnchor,
    sourceIndex,
    down,
    sampling,
    suppressImageClick,
    press: (index: number, at: { x: number; y: number }) =>
      dispatch({ type: "press", index, down: at }),
    setAnchor: (point: SelPoint) => dispatch({ type: "anchor", point }),
    moveInSource: (key: string) => dispatch({ type: "moveIn", sourceKey: key }),
    beginCross: (key: string) => dispatch({ type: "beginCross", sourceKey: key }),
    enterCross: (a: SelPoint, f: SelPoint, key: string) =>
      dispatch({ type: "cross", anchor: a, focus: f, hideKey: key }),
    setFocus: (point: SelPoint) => dispatch({ type: "focus", focus: point }),
    exitToNative: (sourceKey: string) => dispatch({ type: "exit", sourceKey }),
    up: () => dispatch({ type: "up" }),
    shiftFocus: (point: SelPoint, key: string, anchor?: SelPoint) =>
      dispatch({ type: "shiftClick", focus: point, hideKey: key, anchor }),
    shiftExit: (key: string, offset: number) => dispatch({ type: "shiftExit", key, offset }),
    selectAll: (from: SelPoint, to: SelPoint) => dispatch({ type: "selectAll", from, to }),
    clear: () => dispatch({ type: "clear" }),
    clearAll: () => dispatch({ type: "clearAll" }),
    selectImage: (key: string) => dispatch({ type: "selectImage", key }),
    clearImage: () => dispatch({ type: "clearImage" }),
    reset: () => dispatch({ type: "load" }),
    setSampling: (on: boolean) => dispatch({ type: "sample", on }),
    setHover: (index: number) => dispatch({ type: "hover", index }),
    setSuppressImageClick: (on: boolean) => dispatch({ type: "suppress", on }),
  };
}
