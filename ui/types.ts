// IPC DTO 镜像类型：Rust serde 为契约单一来源（core/src/todo.rs），防多处声明漂移

/** 单条待办（镜像 TodoItem） */
export interface TodoItem {
  /** 条目唯一标识（创建序） */
  id: number;
  /** 待办文本 */
  text: string;
  /** 完成态 */
  done: boolean;
}

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
