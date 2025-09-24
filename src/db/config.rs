use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

const SET_INIT_STATUS: &str =
    "update config set value = cast(?1 as blob), updated_at = (?2) where name = 'init_status'";

const QUERY_INSERT_MODEL_INFO_COMPLETED: &str =
    "select value from config where name = 'insert_model_info_completed'";

const INSERT_CONFIG_ITEM: &str = r#"insert into config (name, value) values (?1, ?2) on conflict (name) do update set value = excluded.value"#;

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
pub fn completed_init(conn: &Connection, completed_status: CompletedStatus) -> anyhow::Result<()> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let status = completed_status.as_ref();
    conn.execute(SET_INIT_STATUS, (status, &now))?;
    Ok(())
}

/// 检查模型信息是否全部插入到表中
pub fn check_insert_model_info_completed(conn: &Connection) -> anyhow::Result<bool> {
    let init_status = conn.query_row(QUERY_INSERT_MODEL_INFO_COMPLETED, [], |r| {
        r.get::<_, Vec<u8>>(0)
    })?;
    let init_status = String::from_utf8(init_status)?;
    Ok(init_status == "Completed")
}

/// 插入一个新的配置项，如果配置项已经存在，那么则更新这个配置项
pub fn insert_config(conn: &Connection, name: impl ToString, value: Vec<u8>) -> anyhow::Result<()> {
    let name = name.to_string();
    conn.execute(INSERT_CONFIG_ITEM, (&name, &value))?;
    Ok(())
}
