//! PL013.1 真实文件库迁移冒烟（集成测试，沿 tests/storage_probe.rs 同通道）：
//! 旧 schema（无 sort_order）文件库 → Storage::open 触发迁移 → 断言回填 + reorder
//! 落盘 + 重开持久化 + 二次 open 幂等。临时库落系统临时目录（禁触真实用户数据）。

use capsule_todo::storage::Storage;
use std::path::PathBuf;

#[test]
fn file_db_migrates_sort_order_and_persists_reorder() {
    let dir = std::env::temp_dir().join(format!("capsule-migrate-probe-{}", std::process::id()));
    let db = dir.join("data").join("todo.db");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(db.parent().unwrap()).expect("建目录");

    // 1) 旧 schema 手工建库（V0.1.1 原始形态：todos/bubbles 均无 sort_order）
    {
        let conn = rusqlite::Connection::open(&db).expect("开旧库");
        conn.execute_batch(
            "CREATE TABLE todos (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE bubbles (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL
            );
            CREATE TABLE whiteboard (
                id      INTEGER PRIMARY KEY CHECK (id = 1),
                content TEXT NOT NULL DEFAULT ''
            );
            INSERT INTO todos(text, done) VALUES ('旧一', 0);
            INSERT INTO todos(text, done) VALUES ('旧二', 0);
            INSERT INTO todos(text, done) VALUES ('旧三', 0);
            INSERT INTO todos(text, done) VALUES ('旧完成', 1);
            INSERT INTO bubbles(text) VALUES ('旧泡一');
            INSERT INTO bubbles(text) VALUES ('旧泡二');
            INSERT INTO bubbles(text) VALUES ('旧泡三');",
        )
        .expect("旧 schema 建库");
    }

    // 2) open 触发迁移：存量回填 = 现序号。list = 未完成段（1/2/3）+ done 段（4）；
    // 未完成段按 sort_order（= 原 id 序）、done 段在其后
    let path: PathBuf = db.clone();
    let st = Storage::open(&path).expect("迁移式打开");
    let ids: Vec<i64> = st.list().expect("list").iter().map(|t| t.id).collect();
    assert_eq!(
        ids,
        vec![1, 2, 3, 4],
        "未完成段回填序 = 原 id 序，done 段殿后"
    );
    let bids: Vec<i64> = st
        .list_bubbles()
        .expect("bubbles")
        .iter()
        .map(|b| b.id)
        .collect();
    assert_eq!(
        bids,
        vec![3, 2, 1],
        "气泡回填后展示序 = 新在前（历史 unshift 语义）"
    );

    // 3) reorder 落盘（清单 + 气泡）
    st.reorder_todos(&[3, 1, 2]).expect("重排");
    st.reorder_bubbles(&[2, 3, 1]).expect("气泡重排");
    drop(st);

    // 4) 重开验证持久化（list = 重排后的未完成段 3/1/2 + done 段殿后 4）
    let st2 = Storage::open(&path).expect("重开");
    let ids2: Vec<i64> = st2
        .list()
        .expect("list")
        .into_iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(ids2, vec![3, 1, 2, 4], "重排持久化: {ids2:?}");
    let bids2: Vec<i64> = st2
        .list_bubbles()
        .expect("bubbles")
        .into_iter()
        .map(|b| b.id)
        .collect();
    assert_eq!(bids2, vec![2, 3, 1], "气泡重排持久化: {bids2:?}");
    drop(st2);

    // 5) 二次 open 幂等（列已在位不重算回填，不覆盖已排序）
    let st3 = Storage::open(&path).expect("三次开");
    let ids3: Vec<i64> = st3
        .list()
        .expect("list")
        .into_iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(ids3, vec![3, 1, 2, 4], "二次 open 不重算回填: {ids3:?}");
    drop(st3);
    std::fs::remove_dir_all(&dir).expect("清理");
}
