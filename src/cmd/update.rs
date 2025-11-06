//! 更新

use crate::{
    config::{Config as LLamaBuddyConfig, Data, HttpClient as HttpClientConfig, Registry},
    db::{self, CompletedStatus},
    service,
};
use clap::Args;
use std::sync::Arc;
use tracing::info;
use url::Url;

pub async fn update_local_registry(args: UpdateArgs) {
    let UpdateArgs {
        remote_registry: new_remote,
        client: http_client_config,
        saved,
        registry,
        ..
    } = args;
    let (
        LLamaBuddyConfig {
            data: Data { path: data_path },
            registry:
                Registry {
                    remote,
                    client: client_config,
                },
            model,
        },
        config_path,
    ) = LLamaBuddyConfig::try_config_path().expect("Couldn't get the config");
    let client_config = if let Some(new) = http_client_config {
        client_config.merge(new)
    } else {
        client_config
    };
    let remote = new_remote.unwrap_or(remote);
    let client = client_config
        .build_client()
        .expect("Couldn't build reqwest client");
    // 打开数据库文件，创建数据库并且创建配置表、模型信息表
    let sqlite_dir = data_path.join("sqlite");
    let conn =
        service::connection(sqlite_dir, "llama-buddy.sqlite").expect("Couldn't open sqlite file");
    // 检查一下有没有完成初始化，没有完成初始化，那么应该在完成初始化之后才能够更新
    if !service::init::check_init_completed(Arc::clone(&conn))
        .await
        .expect("Couldn't check init whatever completed")
    {
        info!("Initialization should be ensured to be completed");
    } else {
        // 更新注册表
        if registry {
            service::model::try_update_model_info(Arc::clone(&conn), client, remote.clone())
                .await
                .expect("Couldn't update model info");
        }
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
        config
            .write_to_toml(config_path.as_path())
            .expect("Failed to write all configs to file");
    }
    info!("Update completed");
}

#[derive(Args)]
pub struct UpdateArgs {
    #[arg(
        short = 'r',
        long = "remote",
        help = "The remote registry address, the default value is `https://registry.ollama.com/`"
    )]
    pub remote_registry: Option<Url>,
    #[command(flatten)]
    pub client: Option<HttpClientConfig>,
    #[arg(
        short = 's',
        long = "save",
        help = "Save the options provided in the command line to a configuration file"
    )]
    pub saved: bool,
    #[arg(long = "registry", help = "Update the local registry")]
    pub registry: bool,
}
