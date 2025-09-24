mod config;
mod model;

pub(crate) use config::*;
pub(crate) use model::*;

use rusqlite::Connection;
use std::{fs::create_dir_all, path::Path, process::exit};

const INIT_DB_SQL: &str = include_str!("schema.sql");

/// 获取数据库连接
pub fn open(path: impl AsRef<Path>, db_name: impl AsRef<str>) -> anyhow::Result<Connection> {
    let path = path.as_ref();
    if path.exists() && path.is_file() {
        eprintln!("Not Allowed File Path");
        exit(-1);
    }
    if !path.exists() {
        // 创建文件夹
        create_dir_all(path)?;
    }
    let db_path = path.join(db_name.as_ref());
    // 数据库允许可读写，不存在则创建，允许将 path 创建为 URI，使用非 Mutex 模式
    let conn = Connection::open(db_path)?;
    // 加载 tokenizer
    sqlite_simple_tokenizer::load(&conn)?;
    check_schema(&conn)?;
    Ok(conn)
}

// 检查相关表结构有没有创建好
fn check_schema(conn: &Connection) -> anyhow::Result<()> {
    let user_version = conn.pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))?;
    if user_version <= 0 {
        conn.execute_batch(INIT_DB_SQL)?;
    }
    Ok(())
}

// 检查是否完成初始化
pub fn check_init_completed(conn: &Connection) -> anyhow::Result<bool> {
    let init_status = conn.query_row(
        "select value from config where name = 'init_status'",
        [],
        |r| r.get::<_, Vec<u8>>(0),
    )?;
    let init_status = String::from_utf8(init_status)?;
    Ok(init_status == "Completed")
}
