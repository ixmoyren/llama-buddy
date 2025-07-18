use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::error;
use uuid::Uuid;

// 插入 model 信息
#[derive(Eq, PartialEq, Clone, Default, Debug)]
pub(crate) struct ModelInfo {
    // 模型名字
    pub(crate) title: String,
    // 获取这个模型详细介绍的 url
    pub(crate) href: String,
    // 模型简介
    pub(crate) introduction: String,
    // 模型总结
    pub(crate) summary: String,
    // 不同参数的模型
    pub(crate) models: Vec<Model>,
    // 模型详细介绍
    pub(crate) readme: String,
    // 拉取的数量
    pub(crate) pull_count: String,
    // 模型的规格
    pub(crate) tag_count: String,
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

pub fn insert_model_info(
    conn: &mut Connection,
    model_infos: impl IntoIterator<Item = ModelInfo>,
) -> anyhow::Result<bool> {
    // 开启一个事务
    let tx = conn.transaction()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let mut is_failed = false;
    'i: for info in model_infos.into_iter() {
        let model_id = Uuid::now_v7();
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
                                    &model_id,
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
        for model in info.models {
            let id = Uuid::now_v7();
            let result = tx.execute(
                r#"
insert into model (id, name, href, size, context, input, hash, model_id, updated_at)
values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
on conflict (id) do update set name       = excluded.name,
                               href       = excluded.href,
                               size       = excluded.size,
                               context    = excluded.context,
                               input      = excluded.input,
                               hash       = excluded.hash,
                               model_id   = excluded.model_id,
                               updated_at = strftime('%s', 'now');"#,
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
                error!("Insert model failed, err is {err}");
                is_failed = true;
                break 'i;
            }
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
