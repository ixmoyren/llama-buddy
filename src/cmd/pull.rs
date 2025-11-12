//！从远程仓库中拉取模型

use crate::{
    config::{
        Config as LLamaBuddyConfig, Data, HttpClient as HttpClientConfig, HttpClient, Model,
        Registry,
    },
    db,
    db::CompletedStatus,
};
use clap::Args;
use http_extra::{download, download::DownloadParam, retry, sha256::checksum};
use reqwest::Client;
use rusqlite::Connection;
use serde::Deserialize;
use std::path::PathBuf;
use tracing::{debug, info};
use url::Url;

pub async fn pull_model_from_registry(args: PullArgs) {
    let PullArgs {
        name,
        category,
        client: http_client_config,
        saved,
        ..
    } = args;
    // 获取配置
    let (
        LLamaBuddyConfig {
            data: Data { path: data_path },
            registry:
                Registry {
                    remote,
                    client: registry_http_client_config,
                },
            model:
                Model {
                    client: model_http_client_config,
                    ..
                },
        },
        config_path,
    ) = LLamaBuddyConfig::try_config_path().expect("Couldn't get the config");
    let sqlite_dir = data_path.join("sqlite");
    let conn = db::open(sqlite_dir, "llama-buddy.sqlite").expect("Couldn't open sqlite file");
    let (model_name, category) = final_name_and_category(&conn, &name, category);
    // 如果没有提供保存目录，那么使用默认目录
    let dir = data_path.join("model").join(&model_name);
    // 获取下载 Model 时 HTTP client 的配置
    let client_config = if let Some(new) = http_client_config {
        model_http_client_config.merge(new)
    } else {
        model_http_client_config
    };
    let client = client_config
        .build_client()
        .expect("Couldn't build the reqwest client");
    let manifest_url = format!("/v2/library/{name}/manifests/{category}");
    let manifest_url = remote.join(manifest_url.as_str()).unwrap();
    let response = client.get(manifest_url).send().await.unwrap();
    let response_text = response.text().await.unwrap();
    let manifest: Manifest = serde_json::from_str(&response_text).unwrap();
    // 判断当前的 Manifest 的 schema_version 和 media_type 是不是和注册表中的一致，如果不一致，那么需要退出，并且重新适配
    if !db::check_manifest_schema_version_and_media_type(
        &conn,
        manifest.schema_version,
        &manifest.media_type,
    )
    .expect("Failed to check manifest schema version and media type")
    {
        panic!(
            "The manifest schema_version or media_type does not match. Please re-adapt the remote registry."
        );
    }
    // 获取重试时超时设置
    let chunk_timeout = client_config.build_chunk_timeout();
    for layer in manifest.layers {
        let Layer {
            media_type,
            digest,
            size,
        } = layer;
        save_res_to_local(
            &conn,
            &client_config,
            chunk_timeout,
            &remote,
            client.clone(),
            &name,
            &model_name,
            media_type,
            digest,
            size,
            &dir,
        )
        .await;
    }
    let Config {
        media_type,
        digest,
        size,
    } = manifest.config;
    save_res_to_local(
        &conn,
        &client_config,
        chunk_timeout,
        &remote,
        client.clone(),
        &name,
        &model_name,
        media_type,
        digest,
        size,
        &dir,
    )
    .await;
    // 保存一个拉取状态，完成拉取，用来标识全部的资源都已经拉取完成
    db::set_model_pull_status(&conn, &model_name, CompletedStatus::Completed)
        .expect("Couldn't to set model pull status");
    if saved {
        let config = LLamaBuddyConfig {
            data: Data { path: data_path },
            registry: Registry {
                remote,
                client: registry_http_client_config,
            },
            model: Model {
                category,
                client: client_config,
            },
        };
        config
            .write_to_toml(config_path.as_path())
            .expect("Failed to write all configs to file");
    }
    info!("Pull completed");
}

async fn save_res_to_local(
    conn: &Connection,
    client_config: &HttpClient,
    chunk_timeout: Option<u64>,
    remote: &Url,
    client: Client,
    name: &String,
    model_name: &String,
    media_type: String,
    digest: String,
    size: usize,
    dir: &PathBuf,
) {
    let Some((filename, media_type)) = file_name(conn, &media_type, digest.replace("sha256:", ""))
    else {
        return;
    };
    let filepath = dir.join(&filename);
    // 判断文件是否需要重新下载
    if need_retry_download(&filepath, &digest) {
        // 获取重试策略
        let backoff = client_config.build_back_off();
        let blob_url = format!("/v2/library/{name}/blobs/{}", digest.replace(":", "-"));
        let blob_url = remote.join(blob_url.as_str()).unwrap();
        let param = DownloadParam::try_new(blob_url, filename, dir.as_path())
            .expect("Couldn't build a download param.")
            .with_chunk_timeout(chunk_timeout);
        let summary = retry::spawn(backoff, async || {
            download::spawn(client.clone(), param.clone()).await
        })
        .await
        .expect("Couldn't download the resources");
        debug!("{summary:?}");
        let checksum = checksum(&filepath, digest.replace("sha256:", ""))
            .expect("There is no way to obtain the digest of the file");
        if !checksum {
            panic!("{digest}: checksum failed");
        }
    }
    // 将这个目录保存在注册表中
    db::save_model_file_path(&conn, &model_name, &filepath, size, &media_type)
        .expect("Couldn't save model file path and size");
}

fn need_retry_download(filepath: &PathBuf, digest: &String) -> bool {
    // 文件不存在，重新下载
    if !filepath.exists() {
        return true;
    }
    // 文件存在，判断一下文件的摘要，摘要不一样，重新下载
    let Ok(checksum) = checksum(&filepath, digest.replace("sha256:", "")) else {
        // 没有办法校验摘要，那么重新下载
        return true;
    };
    !checksum
}

fn final_name_and_category(
    conn: &Connection,
    name: impl AsRef<str> + std::fmt::Display,
    category: Option<String>,
) -> (String, String) {
    match category {
        None => {
            let model_name = db::get_first_model_name(conn, name).unwrap();
            if let Some(category) = model_name.clone().rsplit(":").next() {
                (model_name, category.to_owned())
            } else {
                panic!("The category cannot be obtained from the local registry.")
            }
        }
        Some(category) => {
            // 用户有提供 category，那么检查这个 name:category 是否在本地注册表中存在
            let model_name = format!("{name}:{category}");
            if !db::check_model_name(&conn, &model_name) {
                panic!(
                    "The provided model name is not in the local registry. Please check the model name or try to update the local registry."
                );
            }
            (model_name, category)
        }
    }
}

fn file_name(
    conn: &Connection,
    media_type: impl AsRef<str>,
    digest: impl AsRef<str>,
) -> Option<(String, String)> {
    let digest = digest.as_ref();
    let media_type = media_type.as_ref();
    let Some((media, file_type)) = db::get_media_type(conn, media_type).expect("No media type")
    else {
        return None;
    };
    Some((format!("{media}-{digest}.{file_type}"), media))
}

#[derive(Args)]
pub struct PullArgs {
    #[arg(short = 'n', long = "name", help = "The name of mode")]
    pub name: String,
    #[arg(
        short = 'c',
        long = "category",
        help = "The category of mode, If the version of the mode is not provided, the default value is obtained from the local registry"
    )]
    pub category: Option<String>,
    #[arg(
        short = 's',
        long = "save",
        help = "Save the options provided in the command line to a configuration file"
    )]
    pub saved: bool,
    #[command(flatten)]
    pub client: Option<HttpClientConfig>,
}

#[derive(Debug, Deserialize)]
struct Layer {
    #[serde(rename(deserialize = "mediaType"))]
    media_type: String,
    digest: String,
    size: usize,
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(rename(deserialize = "mediaType"))]
    media_type: String,
    digest: String,
    size: usize,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(rename(deserialize = "schemaVersion"))]
    schema_version: u32,
    #[serde(rename(deserialize = "mediaType"))]
    media_type: String,
    config: Config,
    layers: Vec<Layer>,
}
