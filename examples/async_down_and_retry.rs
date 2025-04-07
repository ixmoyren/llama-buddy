use http_extra::{
    download,
    download::{DownloadParam, DownloadStatus},
    retry,
    retry::strategy::FibonacciBackoff,
};
use reqwest::{Client, Url};
use std::{str::FromStr, time::Duration};
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    let client = Client::builder()
        .pool_max_idle_per_host(32)
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    // 需要下载的文件
    let filename = "7z2409-linux-x64.tar.gz";
    // 创建一个临时文件夹用来保存文件
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path();
    // 重试策略，使用 Fibonacci，并且重试 5 次
    let fibonacci_backoff = FibonacciBackoff::from_millis(1000).take(5);
    let summary = retry::spawn(fibonacci_backoff, async || {
        let url = Url::from_str("https://www.7-zip.org/a/7z2409-linux-x64.tar.xz").unwrap();
        let param = DownloadParam::try_new(url, filename, dir_path).unwrap();
        download::spawn(client.clone(), param).await
    })
    .await
    .unwrap();
    assert_eq!(summary.status(), DownloadStatus::Success);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .open(dir_path.join(filename))
        .unwrap();
    let file_len = file.metadata().unwrap().len();
    assert_eq!(summary.connet_length(), file_len);
}
