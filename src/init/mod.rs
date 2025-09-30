//! 初始化本地注册表

mod model;

use crate::{
    config::{Config as LLamaBuddyConfig, Data, HttpClient as HttpClientConfig, Registry},
    db::{
        self, CompletedStatus, check_init_completed, check_insert_model_info_completed,
        completed_init, insert_model_info, query_model_title_and_model_info,
        save_library_to_library_raw_data,
    },
    init::model::{convert_to_model_infos, fetch_library_html, fetch_model_more_info},
};
use clap::Args;
use std::{collections::VecDeque, fs, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use tracing::{error, info};
use url::Url;

pub async fn init_local_registry(args: InitArgs) -> anyhow::Result<()> {
    let InitArgs {
        remote_registry: new_remote,
        path: new_data_path,
        client: http_client_config,
        saved,
        force,
        ..
    } = args;
    let (
        LLamaBuddyConfig {
            data: Data { path },
            registry:
                Registry {
                    remote,
                    client: client_config,
                },
            model,
        },
        config_path,
    ) = LLamaBuddyConfig::try_config_path()?;
    let data_path = new_data_path.unwrap_or(path);
    let client_config = if let Some(new) = http_client_config {
        client_config.merge(new)
    } else {
        client_config
    };
    let remote = new_remote.unwrap_or(remote);
    let client = client_config.build_client()?;
    if force {
        fs::remove_dir_all(data_path.as_path())?
    }
    // 打开数据库文件，创建数据库并且创建配置表、模型信息表
    let sqlite_dir = data_path.join("sqlite");
    let conn = db::open(sqlite_dir, "llama-buddy.sqlite")?;
    // 检查一下有没有完成初始化，初始化已经完成，那么直接退出
    if check_init_completed(&conn)? {
        info!("Initialization completed");
        return Ok(());
    }
    if !check_insert_model_info_completed(&conn)? {
        let mut old_model_raw_digest_map = query_model_title_and_model_info(&conn)?;
        // 创建一个单生产者单消费者的 channel，用来传递 library_html
        let (library_html_sender, library_html_receiver) =
            tokio::sync::oneshot::channel::<String>();
        let (model_info_sender, mut model_info_receiver) = tokio::sync::mpsc::channel(256);
        // 生产者为从 ollama.com 中获取的全部模型列表的数据
        let remote_registry = remote.clone();
        let send_job = tokio::spawn(async move {
            let library_html = fetch_library_html(client.clone(), remote_registry.clone())
                .await
                .unwrap_or_else(|error| {
                    error!("fetch library html failed, {error}");
                    "".to_owned()
                });
            let library_html_str = library_html.as_str();
            let mut model_infos =
                convert_to_model_infos(library_html_str).unwrap_or_else(|error| {
                    error!("convert to model info failed, {error}");
                    VecDeque::default()
                });
            library_html_sender
                .send(library_html)
                .expect("send library html to channel failed!");
            for model_info in model_infos.iter_mut() {
                if let Some(old_raw_digest) = old_model_raw_digest_map.get(&model_info.title) {
                    if old_raw_digest == model_info.raw_digest.as_str() {
                        continue;
                    }
                }
                let (summary, readme, html_raw, model_tag_vec) =
                    fetch_model_more_info(&model_info, client.clone(), remote_registry.clone())
                        .await
                        .expect("fetch model more info failed!");
                model_info.summary = summary;
                model_info.readme = readme;
                model_info.html_raw = html_raw;
                model_info.models = model_tag_vec;
                model_info_sender
                    .send(model_info.to_owned())
                    .await
                    .unwrap_or_else(|error| {
                        error!("send model info to channel failed, {error}");
                    });
            }
        });
        let conn = Arc::new(Mutex::new(conn));
        let conn_one = Arc::clone(&conn);
        // 将 library_html 保存到 config
        let receive_job_one = tokio::spawn(async move {
            match library_html_receiver.await {
                Ok(html) => {
                    let conn = conn_one.lock().await;
                    save_library_to_library_raw_data(&conn, html).unwrap();
                }
                Err(err) => {
                    error!("receiver one get the library html from channel failed, {err}");
                }
            }
        });
        let conn_one = Arc::clone(&conn);
        let receive_job_two = tokio::spawn(async move {
            let mut conn = conn_one.lock().await;
            let mut all_success = true;
            while let Some(model) = model_info_receiver.recv().await {
                if let Ok(is_success) = insert_model_info(&mut conn, model)
                    && !is_success
                {
                    all_success = false;
                }
            }
            if all_success {
                completed_init(&conn, CompletedStatus::Completed).unwrap();
            } else {
                completed_init(&conn, CompletedStatus::Failed).unwrap();
            }
        });
        let _ = tokio::join!(send_job, receive_job_one, receive_job_two);
    }
    // 保存 cli 传入的参数到配置文件中
    if saved {
        let config = LLamaBuddyConfig {
            data: Data { path: data_path },
            registry: Registry {
                client: client_config,
                remote,
            },
            model,
        };
        config.write_to_toml(config_path.as_path())?;
    }
    info!("Initialization completed");
    Ok(())
}

#[derive(Args)]
pub struct InitArgs {
    #[arg(
        short = 'r',
        long = "remote",
        help = "The remote registry address, the default value is `https://registry.ollama.com/`"
    )]
    pub remote_registry: Option<Url>,
    #[arg(
        short = 'p',
        long = "path",
        help = "The path where the local registry is located, the default path is `$DATA$/llama-buddy`"
    )]
    pub path: Option<PathBuf>,
    #[command(flatten)]
    pub client: Option<HttpClientConfig>,
    #[arg(
        short = 's',
        long = "save",
        help = "Save the options provided in the command line to a configuration file"
    )]
    pub saved: bool,
    #[arg(
        long = "force",
        help = "Force initialization will clear all information and rebuild the metadata of the registry"
    )]
    pub force: bool,
}
