//! 初始化本地注册表

mod model;

use crate::{
    config::{Config, Data, HttpClient as HttpClientConfig, Registry},
    db,
    db::{
        CompletedStatus, check_init_completed, check_insert_model_info_completed, completed_init,
        insert_config, insert_model_info,
    },
    init::model::fetch_model_info,
};
use clap::Args;
use std::{fs, path::PathBuf};
use tracing::info;
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
    let back_off = client_config.build_back_off();
    let chunk_timeout = client_config.build_chunk_timout();
    if force {
        fs::remove_dir_all(data_path.as_path())?
    }
    // 拉取 sqlite 的 simple 插件和词库，用于数据库检索
    let sqlite_dir = data_path.join("sqlite");
    let sqlite_plugin_dir = sqlite_dir.join("plugin");
    // 创建数据库并且创建配置表、模型信息表
    let conn = db::open(sqlite_dir, "llama-buddy.sqlite")?;
    // 检查一下有没有完成初始化
    if check_init_completed(&conn)? {
        info!("Initialization completed");
        return Ok(());
    }
    let mut conn = conn;
    if !check_insert_model_info_completed(&conn)? {
        // 拉取远程服务器上的数据，并且保存到模型信息表中
        let (html, model_infos) = fetch_model_info(client.clone(), remote.clone()).await?;
        let html = html.as_bytes();
        let html_sha256 = http_extra::sha256::digest(html);
        insert_config(
            &conn,
            "model_library_html_digest".to_owned(),
            html_sha256.as_bytes().to_vec(),
        )?;
        insert_config(&conn, "model_library_html_data".to_owned(), html.to_vec())?;
        if insert_model_info(&mut conn, model_infos)? {
            // 完成初始化
            completed_init(&conn, CompletedStatus::Completed)?;
        } else {
            completed_init(&conn, CompletedStatus::InProgress)?;
        }
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
