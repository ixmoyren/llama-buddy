use http_extra::{download, download::DownloadParam, retry, retry::strategy::FibonacciBackoff};
use reqwest::{Client, Url};
use std::{str::FromStr, time::Duration};

#[tokio::main]
async fn main() {
    let client = Client::builder()
        .pool_max_idle_per_host(32)
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let url = Url::from_str("https://www.7-zip.org/a/7z2409-linux-x64.tar.xz").unwrap();
    let filename = "7z2409-linux-x64.tar.gz";
    let file_path = dir.path().join(filename);
    let param = DownloadParam::try_new(url, filename, dir.into_path()).unwrap();
    // 重试策略，使用 Fibonacci，并且重试 5 次
    let fibonacci_backoff = FibonacciBackoff::from_millis(1000).take(5);
    let summary = retry::spawn(fibonacci_backoff, async || {
        download::spawn(client.clone(), param.clone()).await
    })
    .await
    .unwrap();
    let file = tokio::fs::OpenOptions::new()
        .read(true)
        .open(&file_path)
        .await
        .unwrap();
    let file_len = file.metadata().await.unwrap().len();
    assert_eq!(summary.connet_length(), file_len);
}
