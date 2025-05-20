//！从远程仓库中拉取模型

use crate::config::{
    Config as LLamaBuddyConfig, Data, HttpClient as HttpClientConfig, Model, Registry,
};
use anyhow::anyhow;
use clap::Args;
use http_extra::{download, download::DownloadParam, retry};
use serde::Deserialize;
use serde_json::from_str;
use tracing::debug;

pub async fn pull_model_from_registry(args: PullArgs) -> anyhow::Result<()> {
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
                    category: category_default,
                    client: model_http_client_config,
                },
        },
        config_path,
    ) = LLamaBuddyConfig::try_config_path()?;
    // 如果没有提供模型的版本，使用配置中的默认值
    let category = category.unwrap_or(category_default);
    let model_prefix = format!("{name}_{category}");
    // 如果没有提供保存目录，那么使用默认目录
    let dir = data_path.join("model").join(model_prefix);
    // 获取下载 Model 时 HTTP client 的配置
    let client_config = if let Some(new) = http_client_config {
        model_http_client_config.merge(new)
    } else {
        model_http_client_config
    };
    let client = client_config.build_client()?;
    let manifest_url = format!("/v2/library/{name}/manifests/{category}");
    let manifest_url = remote.join(manifest_url.as_str())?;
    let response = client.get(manifest_url).send().await?;
    let response_text = response.text().await?;
    let manifest: Manifest = from_str(&response_text)?;
    // let backoff = Arc::new(backoff);
    // 获取重试时超时设置
    let chunk_timeout = client_config.build_chunk_timout();
    for layer in manifest.layers {
        let Layer {
            media_type, digest, ..
        } = layer;
        // 获取重试策略
        let backoff = client_config.build_back_off();
        let blob_url = format!("/v2/library/{name}/blobs/{}", digest.replace(":", "-"));
        let blob_url = remote.join(blob_url.as_str())?;
        let filename = file_name(media_type, digest.replace("sha256:", ""));
        let filepath = dir.join(&filename);
        let param = DownloadParam::try_new(blob_url, filename, dir.as_path())?
            .with_chunk_timeout(chunk_timeout);
        let summary = retry::spawn(backoff, async || {
            download::spawn(client.clone(), param.clone()).await
        })
        .await?;
        debug!("{summary:?}");
        let checksum = http_extra::sha256::checksum(filepath, digest.replace("sha256:", ""))?;
        if !checksum {
            return Err(anyhow!("{digest}: checksum failed"));
        }
    }
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
        config.write_to_toml(config_path.as_path())?;
    }
    Ok(())
}

fn file_name(media_type: impl AsRef<str>, digest: impl AsRef<str>) -> String {
    let digest = digest.as_ref();
    match media_type.as_ref() {
        "application/vnd.ollama.image.model" => format!("model-{digest}.gguf"),
        "application/vnd.ollama.image.template" => format!("template-{digest}.txt"),
        "application/vnd.ollama.image.license" => format!("license-{digest}.txt"),
        "application/vnd.ollama.image.params" => format!("params-{digest}.json"),
        media => {
            if let Some(file_type) = media.rsplit('.').next() {
                format!("{file_type}-{digest}.txt")
            } else {
                digest.to_owned()
            }
        }
    }
}

#[derive(Args)]
pub struct PullArgs {
    #[arg(short = 'n', long = "name", help = "The name of mode")]
    pub name: String,
    #[arg(
        short = 'c',
        long = "category",
        help = "The category of mode, If the version of the mode is not provided, the default value is latest"
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
