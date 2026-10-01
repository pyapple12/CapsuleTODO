// IPC DTO 镜像类型：Rust serde 为契约单一来源（core/src/todo.rs、commands/todo.rs），防多处声明漂移

/** UI 文本输入上限（FIX007.10 单源：三处 maxlength 引用；= Rust MAX_TEXT_LEN 24
 * 的一半兼容余量，联动关系锚 core/src/todo.rs 注释——改上限两端同步） */
export const UI_TEXT_MAX_LEN = 12;

/** 龄期提醒档位（镜像 AgeLevel；裁决在 Rust，前端只读渲染） */
export type AgeLevel = "None" | "Yellow" | "Red";

/** 单条待办（镜像 TodoItem；PL010 三字段扩容） */
export interface TodoItem {
  /** 条目唯一标识（创建序） */
  id: number;
  /** 待办文本 */
  text: string;
  /** 完成态 */
  done: boolean;
  /** 创建时刻 epoch 毫秒（存量迁移行 = null，不模拟时间） */
  created_at: number | null;
  /** 完成时刻 epoch 毫秒（勾选置值、退回清空） */
  done_at: number | null;
  /** 详情板笔记（自由文本） */
  note: string;
}

/** 清单视图条目（镜像 TodoView = TodoItem 扁平 + 龄期档位） */
export type TodoView = TodoItem & {
  /** 龄期提醒档位（无/黄/红） */
  age_level: AgeLevel;
};

/** 单条气泡（镜像 BubbleItem） */
export interface BubbleItem {
  /** 条目唯一标识（创建序） */
  id: number;
  /** 气泡文本 */
  text: string;
}

/** 气泡页快照（镜像 BubbleSnapshot；满额提醒显隐由前端本地阈值裁决，FIX004.23） */
export interface BubbleSnapshot {
  /** 气泡列表（sort_order 升序 = 拖拽序，PL013 起） */
  items: BubbleItem[];
}

/** 气泡捕获结果（镜像 BubbleCaptureOutcome；duplicate = 重复内容未入库，PL015.5） */
export type BubbleCaptureOutcome = { status: "added"; item: BubbleItem } | { status: "duplicate" };

/** 窗口偏好广播载荷（FIX006.11 镜像 PrefsSnapshot；Rust 侧单一来源 = commands/settings.rs） */
export interface PrefsView {
  /** 窗口置顶开关 */
  always_on_top: boolean;
  /** 贴边吸附开关 */
  snap_to_edge: boolean;
}
