use rusqlite::Connection;
use std::{
    fs::create_dir_all,
    path::Path,
    process::exit,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::error;
use uuid::Uuid;

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
    check_schema(&conn)?;
    Ok(conn)
}

// 检查相关表结构有没有创建好
fn check_schema(conn: &Connection) -> anyhow::Result<()> {
    let user_version: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if user_version <= 0 {
        conn.execute_batch(INIT_DB_SQL)?;
    }
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

// 检查是否完成初始化
pub fn check_init_completed(conn: &Connection) -> anyhow::Result<bool> {
    let init_status: Vec<u8> = conn.query_row(
        "select value from config where name = 'init_status'",
        [],
        |r| r.get(0),
    )?;
    let init_status = String::from_utf8(init_status)?;
    Ok(init_status == "Completed")
}

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

// 插入 model 信息
pub struct ModelInfo {
    pub(crate) title: String,
    pub(crate) href: String,
    pub(crate) introduction: String,
    pub(crate) pull_count: String,
    pub(crate) tag_count: String,
    pub(crate) updated_time: String,
}

pub fn insert_model_info(
    conn: &mut Connection,
    model_infos: impl IntoIterator<Item = ModelInfo>,
) -> anyhow::Result<bool> {
    // 开启一个事务
    let tx = conn.transaction()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let mut is_failed = false;
    for info in model_infos.into_iter() {
        let id = Uuid::now_v7();
        let result = tx.execute(r#"
insert into model_info (id, title, href, introduction, pull_count, tag_count, updated_time, updated_at)
values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
on conflict (title, href) do update set title        = excluded.title,
                                        href         = excluded.href,
                                        introduction = excluded.introduction,
                                        pull_count   = excluded.pull_count,
                                        tag_count    = excluded.tag_count,
                                        updated_time = excluded.updated_at,
                                        updated_at   = strftime('%s', 'now');"#,
                                (
                                    &id,
                                    &info.title,
                                    &info.href,
                                    &info.introduction,
                                    &info.pull_count,
                                    &info.tag_count,
                                    &info.updated_time,
                                    &now
                                ),
        );
        if let Err(err) = result {
            error!("Insert model info failed, err is {err}");
            is_failed = true;
            break;
        }
    }
    // 插入一条失败就全部回退事务
    if is_failed {
        tx.rollback()?;
    } else {
        let result = tx.execute(
            "update config set value = cast('Completed' as blob), updated_at = (?1) where name = 'insert_model_info_completed'",
            (&now,),
        );
        match result {
            Ok(_) => tx.commit()?,
            Err(err) => {
                error!("Insert model info failed, err is {err}");
                tx.rollback()?
            }
        }
    }
    Ok(!is_failed)
}
