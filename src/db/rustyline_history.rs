use crate::error::Whatever;
use rusqlite::Connection;
use snafu::{ResultExt, ensure_whatever};
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

const INIT_RUSTYLINE_HISTORY_SCHEME_SQL: &str = include_str!("rustyline_history_schema.sql");

pub fn get_rustyline_history_db_path(path: impl AsRef<Path>) -> Result<PathBuf, Whatever> {
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
    let db_path = path.join("rustyline_history.sqlite");
    Ok(db_path)
}

/// 调整 rustyline_history 的表结构
pub fn change_rustyline_history_scheme(db_path: impl AsRef<Path>) -> Result<(), Whatever> {
    // 数据库允许可读写，不存在则创建，允许将 path 创建为 URI，使用非 Mutex 模式
    let conn = Connection::open(&db_path).with_whatever_context(|_| {
        format!("Couldn't open db in the {}", db_path.as_ref().display())
    })?;
    // 加载 tokenizer
    sqlite_simple_tokenizer::load(&conn)
        .with_whatever_context(|_| "Couldn't load sqlite_simple_tokenizer")?;
    let user_version = conn
        .pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
        .expect("Failed to get user version");
    // 如果等于 1，即 rustyline 初始化 history 数据库成功了
    if user_version == 1 {
        conn.execute_batch(INIT_RUSTYLINE_HISTORY_SCHEME_SQL)
            .with_whatever_context(|_| "Couldn't change rustyline history db schema")?;
    }
    Ok(())
}
