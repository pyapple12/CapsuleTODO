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
