use http_extra::sha256::digest;
use rusqlite::{Connection, Transaction};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, info};
use uuid::Uuid;

const INSERT_INTO_MODEL_INFO: &str = r#"
insert into model_info (id, title, href, raw_digest, introduction, pull_count, tag_count, summary, readme, updated_time, updated_at)
values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
on conflict (title, href) do update set title        = excluded.title,
                                        href         = excluded.href,
                                        raw_digest   = excluded.raw_digest,
                                        introduction = excluded.introduction,
                                        pull_count   = excluded.pull_count,
                                        tag_count    = excluded.tag_count,
                                        summary      = excluded.summary,
                                        readme       = excluded.readme,
                                        updated_time = excluded.updated_time,
                                        updated_at   = strftime('%s', 'now');"#;

const INSERT_INTO_LIBRARY_RAW_DATA: &str = r#"
insert into library_raw_data (href, digest, raw_data, updated_at)
values (?1, ?2, ?3, ?4)
on conflict (href) do update set href       = excluded.href,
                                 digest     = excluded.digest,
                                 raw_data   = excluded.raw_data,
                                 updated_at = strftime('%s', 'now');"#;

const INSERT_INTO_MODEL: &str = r#"
insert into model (id, name, href, size, context, input, hash, model_id, updated_at)
values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
on conflict (name) do update set name       = excluded.name,
                                 href       = excluded.href,
                                 size       = excluded.size,
                                 context    = excluded.context,
                                 input      = excluded.input,
                                 hash       = excluded.hash,
                                 updated_at = strftime('%s', 'now');"#;

const QUERY_MODEL_TITLE_AND_RAW_DIGEST: &str = r#"
select title, raw_digest from model_info;
"#;

// 插入 model 信息
#[derive(Eq, PartialEq, Clone, Default, Debug)]
pub(crate) struct ModelInfo {
    // 模型名字
    pub(crate) title: String,
    // 获取这个模型详细介绍的 url
    pub(crate) href: String,
    // 原始数据的摘要
    pub(crate) raw_digest: String,
    // 模型简介
    pub(crate) introduction: String,
    // 拉取的数量
    pub(crate) pull_count: String,
    // 模型的规格
    pub(crate) tag_count: String,
    // 模型介绍摘要
    pub(crate) summary: String,
    // 不同参数的模型
    pub(crate) models: Vec<Model>,
    // 模型详细介绍
    pub(crate) readme: String,
    // 更新时间
    pub(crate) updated_time: String,
    // 模型原始详细的 html
    pub(crate) html_raw: String,
}

#[derive(Eq, PartialEq, Clone, Default, Debug)]
pub(crate) struct Model {
    // 模型名字
    pub(crate) name: String,
    // 路径
    pub(crate) href: String,
    // 模板
    pub(crate) template: String,
    // 许可
    pub(crate) license: String,
    // 参数
    pub(crate) params: String,
    // 大小
    pub(crate) size: String,
    // 上下文大小
    pub(crate) context: String,
    // 输入类型
    pub(crate) input: String,
    // 模型 hash
    pub(crate) hash: String,
}

pub fn save_library_to_library_raw_data(conn: &Connection, html: String) -> anyhow::Result<bool> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
    let digest = digest(html.as_bytes());
    let href = "/library?sort=newest";
    conn.execute(INSERT_INTO_LIBRARY_RAW_DATA, (&href, &digest, &html, &now))?;
    Ok(true)
}

pub fn query_model_title_and_model_info(
    conn: &Connection,
) -> anyhow::Result<HashMap<String, String>> {
    let mut map = HashMap::<String, String>::new();
    let mut statement = conn.prepare(QUERY_MODEL_TITLE_AND_RAW_DIGEST)?;
    let rows = statement.query_map([], |row| {
        let title = row.get::<_, String>(0)?;
        let raw_digest = row.get::<_, String>(1)?;
        Ok((title, raw_digest))
    })?;
    for row in rows {
        let (title, raw_digest) = row?;
        map.insert(title, raw_digest);
    }
    Ok(map)
}

pub fn insert_model_info(conn: &mut Connection, info: ModelInfo) -> anyhow::Result<bool> {
    // 开启一个事务
    let tx = conn.transaction()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
    let model_id = Uuid::now_v7();
    let result = tx.execute(
        INSERT_INTO_MODEL_INFO,
        (
            &model_id,
            &info.title,
            &info.href,
            &info.raw_digest,
            &info.introduction,
            &info.pull_count,
            &info.tag_count,
            &info.summary,
            &info.readme,
            &info.updated_time,
            &now,
        ),
    );
    if let Err(err) = result {
        error!(
            "Insert model info failed, err is {err}, model id is {model_id}, title is {}",
            info.title
        );
        return rollback_and_return(tx);
    }
    let digest = digest(&info.html_raw.as_bytes());
    let result = tx.execute(
        INSERT_INTO_LIBRARY_RAW_DATA,
        (&info.href, &digest, &info.html_raw, &now),
    );
    if let Err(err) = result {
        error!(
            "Insert model raw data failed, err is {err}, model id is {model_id}, title is {}, raw is {}",
            info.title, info.html_raw
        );
        return rollback_and_return(tx);
    }
    for model in info.models {
        let id = Uuid::now_v7();
        let result = tx.execute(
            INSERT_INTO_MODEL,
            (
                &id,
                &model.name,
                &model.href,
                &model.size,
                &model.context,
                &model.input,
                &model.hash,
                &model_id,
                &now,
            ),
        );
        if let Err(err) = result {
            error!("Insert model failed, err is {err}, id is {id}, model is {model:?}");
            return rollback_and_return(tx);
        }
    }
    let result = tx.execute(
            "update config set value = cast('Completed' as blob), updated_at = (?1) where name = 'insert_model_info_completed'",
            (&now,),
        );
    match result {
        Ok(_) => tx.commit()?,
        Err(err) => {
            error!("Update config set 'insert_model_info_completed' failed, err is {err}");
            return rollback_and_return(tx);
        }
    }
    info!("Insert model info success, title is {}", info.title);
    Ok(true)
}

fn rollback_and_return(tx: Transaction) -> anyhow::Result<bool> {
    tx.rollback()?;
    Ok(false)
}
