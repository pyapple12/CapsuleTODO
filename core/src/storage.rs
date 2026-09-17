//! SQLite 清单存储：todos 表增删勾查（全参数化绑定，禁 SQL 拼接）。
//! 单一事实源 = db（PL002 定案）：操作即落库，list 排序 = 未完成在前按 id 升序。

use std::path::Path;

use rusqlite::Connection;
use thiserror::Error;

use crate::todo::TodoItem;

/// 存储层错误：SQLite 透传 / IO（目录自建失败）/ 指定条目不存在
#[derive(Debug, Error)]
pub enum StorageError {
    /// SQLite 操作失败（打开/建表/读写）
    #[error("SQLite 错误：{0}")]
    Sqlite(#[from] rusqlite::Error),
    /// 数据目录创建或文件 IO 失败
    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),
    /// 指定 id 的条目不存在（更新/删除零行，零静默）
    #[error("待办条目不存在：{0}")]
    NotFound(i64),
}

/// SQLite 清单存储（rusqlite Connection 非 Sync，跨线程共享须经 Mutex）
pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// 打开（或创建）库文件：父目录缺失自建，并确保表结构就位
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        let storage = Self { conn };
        storage.init()?;
        Ok(storage)
    }

    /// 打开内存库（测试专用通道，禁触真实用户数据）
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let storage = Self { conn };
        storage.init()?;
        Ok(storage)
    }

    /// 建表（幂等：CREATE TABLE IF NOT EXISTS；不做 created_at——排序按 id 即创建序）
    pub fn init(&self) -> Result<(), StorageError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS todos (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0
            );",
        )?;
        Ok(())
    }

    /// 新增待办（文本须先经 todo::validate_text 业务校验），返回含回填 id 的条目
    pub fn add(&self, text: &str) -> Result<TodoItem, StorageError> {
        self.conn
            .execute("INSERT INTO todos(text, done) VALUES (?1, 0)", [text])?;
        Ok(TodoItem {
            id: self.conn.last_insert_rowid(),
            text: text.to_string(),
            done: false,
        })
    }

    /// 翻转完成态并返回更新后的条目；零行返回 NotFound（零静默）
    pub fn toggle(&self, id: i64) -> Result<TodoItem, StorageError> {
        let changed = self.conn.execute(
            "UPDATE todos SET done = NOT done WHERE id = ?1",
            rusqlite::params![id],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound(id));
        }
        self.get(id)
    }

    /// 读取单条；不存在返回 NotFound
    pub fn get(&self, id: i64) -> Result<TodoItem, StorageError> {
        self.conn
            .query_row(
                "SELECT id, text, done FROM todos WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok(TodoItem {
                        id: row.get(0)?,
                        text: row.get(1)?,
                        done: row.get(2)?,
                    })
                },
            )
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound(id),
                other => StorageError::Sqlite(other),
            })
    }

    /// 删除条目；零行删除返回 NotFound
    pub fn remove(&self, id: i64) -> Result<(), StorageError> {
        let changed = self
            .conn
            .execute("DELETE FROM todos WHERE id = ?1", rusqlite::params![id])?;
        if changed == 0 {
            return Err(StorageError::NotFound(id));
        }
        Ok(())
    }

    /// 清单排序视图：未完成在前按 id 升序、已完成在后按 id 升序
    pub fn list(&self) -> Result<Vec<TodoItem>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, text, done FROM todos ORDER BY done ASC, id ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(TodoItem {
                id: row.get(0)?,
                text: row.get(1)?,
                done: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage() -> Storage {
        Storage::open_in_memory().expect("内存库必须可开")
    }

    #[test]
    fn init_is_idempotent() {
        let st = storage();
        st.init().expect("重复建表必须幂等");
    }

    #[test]
    fn add_and_list_roundtrip() {
        let st = storage();
        let a = st.add("任务一").expect("写入必须成功");
        let b = st.add("任务二").expect("写入必须成功");
        assert_eq!(a.id, 1);
        assert_eq!(b.id, 2);
        assert_eq!(a.text, "任务一");
        assert!(!a.done);
        assert_eq!(st.list().expect("读取必须成功").len(), 2);
    }

    #[test]
    fn toggle_roundtrip_and_get() {
        let st = storage();
        let item = st.add("任务一").expect("写入必须成功");
        let flipped = st.toggle(item.id).expect("更新必须成功");
        assert!(flipped.done);
        assert!(st.get(item.id).expect("读取必须成功").done);
        let back = st.toggle(item.id).expect("更新必须成功");
        assert!(!back.done);
    }

    #[test]
    fn remove_deletes_row() {
        let st = storage();
        let item = st.add("任务一").expect("写入必须成功");
        st.remove(item.id).expect("删除必须成功");
        assert!(st.list().expect("读取必须成功").is_empty());
    }

    #[test]
    fn missing_id_operations_are_errors() {
        let st = storage();
        assert!(matches!(st.toggle(9), Err(StorageError::NotFound(9))));
        assert!(matches!(st.get(9), Err(StorageError::NotFound(9))));
        assert!(matches!(st.remove(9), Err(StorageError::NotFound(9))));
    }

    #[test]
    fn list_orders_undone_first_by_id() {
        let st = storage();
        let a = st.add("任务一").expect("写入必须成功");
        let b = st.add("任务二").expect("写入必须成功");
        let c = st.add("任务三").expect("写入必须成功");
        st.toggle(a.id).expect("更新必须成功");
        st.toggle(c.id).expect("更新必须成功");
        let ids: Vec<i64> = st
            .list()
            .expect("读取必须成功")
            .into_iter()
            .map(|it| it.id)
            .collect();
        assert_eq!(ids, vec![b.id, a.id, c.id]);
    }

    #[test]
    fn open_creates_parent_dir_and_file() {
        let root = std::env::temp_dir().join(format!("capsule-todo-test-{}", std::process::id()));
        let db = root.join("data").join("todo.db");
        {
            let st = Storage::open(&db).expect("打开必须成功（父目录自建）");
            st.add("落盘").expect("写入必须成功");
            assert!(db.exists());
        }
        // 先 drop 关闭连接再清理（Windows 下持有句柄时目录删不掉）
        std::fs::remove_dir_all(&root).expect("清理必须成功");
    }
}
