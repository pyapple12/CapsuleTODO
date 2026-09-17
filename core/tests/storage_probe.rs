//! rusqlite bundled 工具链冒烟探针（沿 Pulse 资产模式）：验证 bundled SQLite 编译与基本读写可用。

use rusqlite::Connection;

#[test]
fn bundled_sqlite_smoke() {
    let conn = Connection::open_in_memory().expect("内存库必须可开");
    conn.execute_batch(
        "CREATE TABLE probe(id INTEGER PRIMARY KEY); INSERT INTO probe(id) VALUES (1);",
    )
    .expect("建表写入必须成功");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM probe", [], |row| row.get(0))
        .expect("查询必须成功");
    assert_eq!(count, 1);
}
