// IPC DTO 镜像类型：Rust serde 为契约单一来源（core/src/todo.rs、commands/todo.rs），防多处声明漂移

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
  /** 条目唯一标识（创建序，展示按 id 倒序 = 新在前） */
  id: number;
  /** 气泡文本 */
  text: string;
}

/** 气泡页快照（镜像 BubbleSnapshot；满 5 提醒标记由 Rust 侧裁决，前端零业务） */
export interface BubbleSnapshot {
  /** 气泡列表（新在前） */
  items: BubbleItem[];
  /** 是否达到提醒阈值 */
  remind: boolean;
}
