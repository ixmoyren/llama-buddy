use crate::error::Whatever;
use rusqlite::Connection;
use snafu::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

const SET_INIT_STATUS: &str =
    "update config set value = cast(?1 as blob), updated_at = (?2) where name = 'init_status'";

const QUERY_INSERT_MODEL_INFO_COMPLETED: &str =
    "select value from config where name = 'insert_model_info_completed'";

const SET_INSERT_MODEL_INFO_COMPLETED: &str = "update config set value = cast(?1 as blob), updated_at = (?2) where name = 'insert_model_info_completed'";

const INSERT_CONFIG_ITEM: &str = r#"insert into config (name, value) values (?1, ?2) on conflict (name) do update set value = excluded.value, updated_at = strftime('%s', 'now')"#;

const QUERY_MANIFEST_SCHEMA_VERSION: &str =
    r#"select cast(value as integer) from config where name = 'manifest_schema_version'"#;

const QUERY_MANIFEST_MEDIA_TYPE: &str =
    r#"select value from config where name = 'manifest_media_type'"#;

const QUERY_MEDIA_TYPE: &str = r#"select name from config where value = cast(?1 as blob)"#;

const QUERY_MEDIA_FILE_TYPE: &str = r#"select value from config where name = ?1"#;

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

/// 完成初始化
pub fn completed_init(
    conn: &Connection,
    completed_status: CompletedStatus,
) -> Result<(), Whatever> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .with_whatever_context(|_| "Failed to get system time when set init status to completed")?
        .as_secs();
    let status = completed_status.as_ref();
    conn.execute(SET_INIT_STATUS, (status, &now))
        .with_whatever_context(|_| "Failed to set init status to completed")?;
    Ok(())
}

/// 检查模型信息是否全部插入到表中
pub fn check_insert_model_info_completed(conn: &Connection) -> Result<bool, Whatever> {
    let init_status = conn
        .query_row(QUERY_INSERT_MODEL_INFO_COMPLETED, [], |r| {
            r.get::<_, Vec<u8>>(0)
        })
        .with_whatever_context(|_| "Failed to get init status")?;
    let init_status = String::from_utf8(init_status)
        .with_whatever_context(|_| "Couldn't convert init_status to string")?;
    Ok(init_status == "Completed")
}

pub fn completed_insert_model_info(
    conn: &Connection,
    completed_status: CompletedStatus,
) -> Result<(), Whatever> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .with_whatever_context(|_| "Failed to get system time when set init status to completed")?
        .as_secs();
    let status = completed_status.as_ref();
    conn.execute(SET_INSERT_MODEL_INFO_COMPLETED, (status, &now))
        .with_whatever_context(|_| "Failed to set init status to completed")?;
    Ok(())
}

pub fn completed_update_model_info(
    conn: &Connection,
    completed_status: CompletedStatus,
) -> Result<(), Whatever> {
    let status = completed_status.as_ref();
    insert_config(
        conn,
        "update_model_info_completed",
        status.as_bytes().to_vec(),
    )
    .with_whatever_context(|_| "Failed to set init status to completed")?;
    Ok(())
}

/// 插入一个新的配置项，如果配置项已经存在，那么则更新这个配置项
pub fn insert_config(
    conn: &Connection,
    name: impl AsRef<str>,
    value: Vec<u8>,
) -> Result<(), Whatever> {
    let name = name.as_ref();
    conn.execute(INSERT_CONFIG_ITEM, (name, &value))
        .with_whatever_context(|_| "Failed to insert config")?;
    Ok(())
}

pub fn check_manifest_schema_version_and_media_type(
    conn: &Connection,
    schema_version: u32,
    media_type: impl AsRef<str>,
) -> Result<bool, Whatever> {
    let manifest_scheme_version = conn
        .query_row(QUERY_MANIFEST_SCHEMA_VERSION, [], |r| r.get::<_, u32>(0))
        .with_whatever_context(|_| "Failed to get manifest scheme version")?;
    if schema_version != manifest_scheme_version {
        return Ok(false);
    }
    let manifest_media_type = conn
        .query_row(QUERY_MANIFEST_MEDIA_TYPE, [], |r| r.get::<_, Vec<u8>>(0))
        .with_whatever_context(|_| "Failed to get manifest media type")?;
    let manifest_media_type = String::from_utf8(manifest_media_type)
        .with_whatever_context(|_| "Couldn't convert manifest_media_type to string")?;
    let media_type = media_type.as_ref();
    Ok(manifest_media_type == media_type)
}

pub fn get_media_type(
    conn: &Connection,
    media_type: impl AsRef<str>,
) -> Result<Option<(String, String)>, Whatever> {
    let media_type = media_type.as_ref();
    let media_type = conn
        .query_row(QUERY_MEDIA_TYPE, [media_type], |r| r.get::<_, String>(0))
        .with_whatever_context(|_| "Failed to get media type")?;
    if media_type.is_empty() || media_type == "" {
        return Ok(None);
    }
    let media = media_type.replace("_media_type", "");
    let file_type = conn
        .query_row(QUERY_MEDIA_FILE_TYPE, [&media], |r| r.get::<_, Vec<u8>>(0))
        .with_whatever_context(|_| "Failed to get media file type")?;
    let file_type = String::from_utf8(file_type)
        .with_whatever_context(|_| "Couldn't convert media file type to string")?;
    Ok(Some((media, file_type)))
}
