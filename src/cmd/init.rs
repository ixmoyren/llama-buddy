//! 初始化本地注册表

use crate::{
    config::{Config as LLamaBuddyConfig, Data, HttpClient as HttpClientConfig, Registry},
    db::CompletedStatus,
    service,
};
use clap::Args;
use std::{fs, path::PathBuf, sync::Arc};
use tracing::{error, info};
use url::Url;

pub async fn init_local_registry(args: InitArgs) {
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
    ) = LLamaBuddyConfig::try_config_path().expect("Couldn't get the config");
    let data_path = new_data_path.unwrap_or(path);
    let client_config = if let Some(new) = http_client_config {
        client_config.merge(new)
    } else {
        client_config
    };
    let remote = new_remote.unwrap_or(remote);
    let client = client_config
        .build_client()
        .expect("Couldn't build reqwest client");
    // 强制初始化，清理全部的配置文件
    if force {
        fs::remove_dir_all(data_path.as_path())
            .expect("Couldn't remove all dir when force init repo")
    }
    // 打开数据库文件，创建数据库并且创建配置表、模型信息表
    let sqlite_dir = data_path.join("sqlite");
    let conn =
        service::connection(sqlite_dir, "llama-buddy.sqlite").expect("Couldn't open sqlite file");
    // 检查一下有没有完成初始化，初始化已经完成，那么直接退出
    if service::init::check_init_completed(Arc::clone(&conn))
        .await
        .expect("Couldn't check init whatever completed")
    {
        info!("Initialization completed");
    }
    match service::model::try_save_model_info(Arc::clone(&conn), client, remote.clone()).await {
        Ok(_) => {
            // 如果成功，那么将初始化状态设置成完成，后续的流程应该以这个状态为准
            service::init::completed_init(Arc::clone(&conn), CompletedStatus::Completed)
                .await
                .expect("Couldn't set init status to completed");
        }
        Err(error) => {
            error!("Failed to try to save model info, {error:?}");
            // 如果失败，将初始化状态设置为失败
            service::init::completed_init(Arc::clone(&conn), CompletedStatus::Failed)
                .await
                .expect("Couldn't set init status to failed");
        }
    };
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
        config
            .write_to_toml(config_path.as_path())
            .expect("Failed to write all configs to file");
    }
    info!("Initialization completed");
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
