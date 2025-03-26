#![feature(return_type_notation)]

pub mod client;
mod download;
mod error;

pub use download::*;
pub use error::*;
use reqwest::Url;

pub trait Download {
    /// 获取 content-length 和 accept-ranges
    async fn get_content_length_and_accept_ranges(
        &self,
        url: Url,
    ) -> Result<(Option<u64>, Option<String>), HttpExtraError>;

    /// 获取文件
    async fn fetch_file(&self, download: DownloadParam) -> Result<DownloadSummary, HttpExtraError>;
}

pub async fn download<D>(client: D, param: DownloadParam) -> Result<DownloadSummary, HttpExtraError>
where
    D: Download,
{
    client.fetch_file(param).await
}
