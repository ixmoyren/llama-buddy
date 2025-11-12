pub(crate) mod config;
pub(crate) mod model;

use crate::error::Whatever;
use rusqlite::Connection;
use snafu::prelude::*;
use std::{fs::create_dir_all, path::Path};

pub enum CompletedStatus {
    NotStarted,
    Completed,
    InProgress,
    Failed,
}

impl AsRef<str> for CompletedStatus {
    fn as_ref(&self) -> &str {
        match self {
            Self::NotStarted => "Not Started",
            Self::Completed => "Completed",
            Self::InProgress => "In Progress",
            Self::Failed => "Failed",
        }
    }
}

const INIT_DB_SQL: &str = include_str!("schema.sql");

/// 获取数据库连接
pub fn open(path: impl AsRef<Path>, db_name: impl AsRef<str>) -> Result<Connection, Whatever> {
    let path = path.as_ref();
    // 保存数据库的路径不能是一个文件
    ensure_whatever!(
        !path.exists() || !path.is_file(),
        "The path for saving the database cannot be a file"
    );
    if !path.exists() {
        // 创建文件夹
        create_dir_all(path).with_whatever_context(|_| {
            format!("Couldn't create dir in the path({})", path.display())
        })?;
    }
    let db_path = path.join(db_name.as_ref());
    // 数据库允许可读写，不存在则创建，允许将 path 创建为 URI，使用非 Mutex 模式
    let conn = Connection::open(db_path)
        .with_whatever_context(|_| format!("Couldn't open db in the {}", path.display()))?;
    // 加载 tokenizer
    sqlite_simple_tokenizer::load(&conn)
        .with_whatever_context(|_| "Couldn't load sqlite_simple_tokenizer")?;
    check_schema(&conn).with_whatever_context(|_| "Couldn't check schema")?;
    Ok(conn)
}

// 检查相关表结构有没有创建好
fn check_schema(conn: &Connection) -> Result<(), Whatever> {
    let user_version = conn
        .pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
        .with_whatever_context(|_| "Couldn't check user_version")?;
    if user_version <= 0 {
        conn.execute_batch(INIT_DB_SQL)
            .with_whatever_context(|_| "Couldn't init db")?;
    }
    Ok(())
}

// 检查是否完成初始化
pub fn check_init_completed(conn: &Connection) -> Result<bool, Whatever> {
    let init_status = conn
        .query_row(
            "select value from config where name = 'init_status'",
            [],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .with_whatever_context(|_| "Couldn't get init_status")?;
    let init_status = String::from_utf8(init_status)
        .with_whatever_context(|_| "Couldn't convert init_status to string")?;
    Ok(init_status == CompletedStatus::Completed.as_ref())
}
