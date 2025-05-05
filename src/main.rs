use clap::{
    builder::{styling::AnsiColor, Styles},
    Parser,
};
use http_extra::{download, download::DownloadParam, retry, retry::strategy::FibonacciBackoff};
use reqwest::{Client, Proxy};
use serde::Deserialize;
use serde_json::from_str;
use std::{path::PathBuf, thread, time::Duration};
use tracing::{debug, Level};
use url::Url;

const CLI_HELP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Blue.on_default().bold())
    .usage(AnsiColor::Blue.on_default().bold())
    .literal(AnsiColor::White.on_default())
    .placeholder(AnsiColor::Green.on_default());

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    let Args {
        name,
        category,
        registry,
        dest_dir,
        proxy,
        timeout,
        chunk_timeout,
        retry,
        ..
    } = Args::parse();
    // 如果没有提供模型的版本，那么默认 latest
    let category = if let Some(category) = category {
        category
    } else {
        "latest".to_owned()
    };
    let model_prefix = format!("{name}_{category}");
    // 如果没有提供保存目录，那么使用默认目录
    let dir = if let Some(dir) = dest_dir {
        dir
    } else {
        let dirs = dir_extra::BaseDirs::new().unwrap();
        dirs.data_dir()
            .join("ollama")
            .join("model")
            .join(&model_prefix)
    };
    let client_build = Client::builder()
        .pool_max_idle_per_host(thread::available_parallelism().map_or(1, |p| p.get()));
    let client_build = if let Some(timeout) = timeout {
        client_build.timeout(Duration::from_secs(timeout))
    } else {
        client_build
    };
    let client_build = if let Some(proxy) = proxy {
        let proxy = Proxy::all(proxy).unwrap();
        client_build.proxy(proxy)
    } else {
        client_build
    };
    let client = client_build.build().unwrap();
    let manifest_url = format!("/v2/library/{name}/manifests/{category}");
    let manifest_url = registry.join(manifest_url.as_str()).unwrap();
    let response = client.get(manifest_url).send().await.unwrap();
    let response_text = response.text().await.unwrap();
    let manifest: Manifest = from_str(&response_text).unwrap();
    // 重试策略，使用 Fibonacci，并且重试 5 次
    let fibonacci_backoff = FibonacciBackoff::from_millis(10000).take(retry);
    for layer in manifest.layers {
        let Layer {
            media_type, digest, ..
        } = layer;
        let blob_url = format!("/v2/library/{name}/blobs/{}", digest.replace(":", "-"));
        let blob_url = registry.join(blob_url.as_str()).unwrap();
        let filename = file_name(media_type, digest.replace("sha256:", ""));
        let filepath = dir.join(&filename);
        let param = DownloadParam::try_new(blob_url, filename, dir.as_path())
            .unwrap()
            .with_chunk_timeout(chunk_timeout);
        let summary = retry::spawn(fibonacci_backoff.clone(), async || {
            download::spawn(client.clone(), param.clone()).await
        })
        .await
        .unwrap();
        debug!("{summary:?}");
        let checksum =
            http_extra::sha256::checksum(filepath, digest.replace("sha256:", "")).unwrap();
        if !checksum {
            eprintln!("{digest}: checksum failed");
            std::process::exit(1);
        }
    }
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

#[derive(Parser)]
#[command(about = "llama-buddy cli interface for related operations")]
#[command(version, long_about = None, styles = CLI_HELP_STYLES)]
struct Args {
    #[arg(short = 'n', long = "name", help = "The name of mode")]
    name: String,
    #[arg(
        short = 'c',
        long = "category",
        help = "The category of mode, If the version of the mode is not provided, the default value is latest"
    )]
    category: Option<String>,
    #[arg(
        short = 'r',
        long = "registry",
        default_value = "https://registry.ollama.ai/"
    )]
    registry: Url,
    #[arg(
        short = 'd',
        long = "dest-dir",
        help = "The location where model is saved, the default path is `~/.local/share/ollama/model`"
    )]
    dest_dir: Option<PathBuf>,
    #[arg(short = 'p', long = "proxy", help = "Proxy address")]
    proxy: Option<String>,
    #[arg(short = 't', long = "timeout", help = "Timeout in seconds")]
    timeout: Option<u64>,
    #[arg(
        long = "chunk_timeout",
        help = "Controls the timeout period for file slice writes"
    )]
    chunk_timeout: Option<u64>,
    #[arg(long = "retry", default_value = "5", help = "Retry times")]
    retry: usize,
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
