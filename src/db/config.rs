use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

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

// 完成初始化
pub fn completed_init(conn: &Connection, completed_status: CompletedStatus) -> anyhow::Result<()> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let status = completed_status.as_ref();
    conn.execute(
        "update config set value = cast(?1 as blob), updated_at = (?2) where name = 'init_status'",
        (status, &now),
    )?;
    Ok(())
}

// 检查是否完成初始化
pub fn check_insert_model_info_completed(conn: &Connection) -> anyhow::Result<bool> {
    let init_status: Vec<u8> = conn.query_row(
        "select value from config where name = 'insert_model_info_completed'",
        [],
        |r| r.get(0),
    )?;
    let init_status = String::from_utf8(init_status)?;
    Ok(init_status == "Completed")
}

// 插入一个新的配置项
pub fn insert_config(conn: &Connection, name: impl ToString, value: Vec<u8>) -> anyhow::Result<()> {
    let name = name.to_string();
    conn.execute(
        "insert into config (name, value) values (?1, ?2)",
        (&name, &value),
    )?;
    Ok(())
}

// 检查是否下载好 libsimple 插件
pub fn check_libsimple(conn: &Connection) -> anyhow::Result<bool> {
    let libsimple_version: Vec<u8> = conn.query_row(
        "select value from config where name = 'libsimple_version'",
        [],
        |r| r.get(0),
    )?;
    let libsimple_version = String::from_utf8(libsimple_version)?;
    Ok(libsimple_version == "v0.5.2")
}

// 将 libsimple 的版本信息写入到数据中
pub fn update_libsimple(conn: &Connection) -> anyhow::Result<()> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    conn.execute(
        "update config set value = cast('v0.5.2' as blob), updated_at = (?1) where name = 'libsimple_version'",
        (&now,),
    )?;
    Ok(())
}
