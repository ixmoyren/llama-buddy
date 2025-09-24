//! 初始化本地注册表

mod config;
mod model;

use crate::{
    config::{Config, Data, HttpClient as HttpClientConfig, Registry},
    db,
    db::{check_init_completed, check_insert_model_info_completed},
    init::{
        config::save_library_to_config,
        model::{fetch_library_html, save_model_info},
    },
};
use clap::Args;
use rusqlite::Connection;
use std::{fs, path::PathBuf, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Mutex, RwLock, broadcast::error::RecvError},
};
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
        Config {
            data: Data { path },
            registry:
                Registry {
                    remote,
                    client: client_config,
                },
            model,
        },
        config_path,
    ) = Config::try_config_path()?;
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
        // 创建一个单生产者多消费者的队列
        let (sender, mut receiver_one) = tokio::sync::broadcast::channel::<String>(16);
        let mut receiver_two = sender.subscribe();
        // 生产者为从 ollama.com 中获取的全部模型列表的数据
        let remote_registry = remote.clone();
        let send_job = tokio::spawn(async move {
            let library_html = fetch_library_html(client.clone(), remote_registry)
                .await
                .unwrap_or_else(|error| {
                    error!("fetch library html failed, {error}");
                    "".to_owned()
                });
            sender
                .send(library_html)
                .expect("send library html to channel failed!");
        });
        let conn = Arc::new(Mutex::new(conn));
        let conn_one = Arc::clone(&conn);
        let receive_job_one = tokio::spawn(async move {
            match receiver_one.recv().await {
                Ok(html) => {
                    let conn = conn_one.lock().await;
                    save_library_to_config(html, &conn);
                }
                Err(err) => {
                    error!("receiver one get the library html from channel failed, {err}");
                }
            }
        });
        let conn_two = Arc::clone(&conn);
        let receive_job_two = tokio::spawn(async move {
            match receiver_two.recv().await {
                Ok(html) => {
                    let mut conn = conn_two.lock().await;
                    save_model_info(html, &mut conn)
                }
                Err(err) => {
                    error!("receiver two get the library html from channel failed, {err}");
                }
            }
        });
        let _ = tokio::join!(send_job, receive_job_one, receive_job_two);
    }
    // 保存 cli 传入的参数到配置文件中
    if saved {
        let config = Config {
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
