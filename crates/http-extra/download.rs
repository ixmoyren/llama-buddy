use crate::error::HttpExtraError;
use dir_extra::UserDirs;
use reqwest::Url;
use std::path::{Path, PathBuf};

pub trait Download {
    /// 获取 content-length 和 accept-ranges
    async fn get_content_length_and_accept_ranges(
        &self,
        url: Url,
    ) -> Result<(Option<u64>, Option<String>), HttpExtraError>;

    /// 获取文件
    async fn fetch_file(&self, download: DownloadParam) -> Result<DownloadSummary, HttpExtraError>;
}

pub async fn spawn<D>(client: D, param: DownloadParam) -> Result<DownloadSummary, HttpExtraError>
where
    D: Download,
{
    client.fetch_file(param).await
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadParam {
    // 下载路径
    pub(crate) fetch_from: Url,
    // 保存的文件名
    pub(crate) file_name: String,
    // 保存的路径
    pub(crate) save_to: PathBuf,
    // 重试次数
    pub(crate) retries: usize,
    // 读写文件片允许超时时间
    pub(crate) chunk_timeout: u64,
}

impl DownloadParam {
    pub fn try_new(
        url: Url,
        file_name: impl AsRef<str>,
        save_to: impl AsRef<Path>,
    ) -> Result<Self, HttpExtraError> {
        let save_to = save_to.as_ref();
        // 保存文件的路径存在，但是不是目录，那么直接返回错误
        if save_to.try_exists()? && !save_to.is_dir() {
            return Err(HttpExtraError::PathNotDirectory);
        }
        Ok(Self {
            fetch_from: url,
            file_name: file_name.as_ref().to_owned(),
            save_to: save_to.to_owned(),
            retries: 0,
            chunk_timeout: 60,
        })
    }

    pub fn try_new_default_download_dir(
        url: Url,
        file_name: impl AsRef<str>,
    ) -> Result<Self, HttpExtraError> {
        let file_name = file_name.as_ref().to_owned();
        let save_to = UserDirs::new()?
            .download_dir()
            .ok_or(HttpExtraError::NoDownloadDir)?
            .to_owned();
        Self::try_new(url, file_name, save_to)
    }

    pub fn with_retries(mut self, retries: usize) -> Self {
        self.retries = retries;
        self
    }

    pub fn with_chunk_timeout(mut self, chunk_timeout: u64) -> Self {
        self.chunk_timeout = chunk_timeout;
        self
    }
}

impl TryFrom<Url> for DownloadParam {
    type Error = HttpExtraError;
    fn try_from(url: Url) -> Result<Self, Self::Error> {
        let filename = url
            .path_segments()
            .ok_or_else(|| {
                HttpExtraError::InvalidUrl(format!("The URL({url}) doesn't contain a valid path"))
            })?
            .last()
            .map(|file_name| {
                // 判断 url 最后一个部分的格式，尝试解码
                // 如果是 name 形式，那么就是 name
                // 如果是 name=value 形式，那么 name_value
                // 如果是 name1=value1&name2=value2 形式，那么 name1_value1_name2_value2
                let vec = form_urlencoded::parse(file_name.as_bytes()).fold(
                    Vec::new(),
                    |mut vec, (name, value)| {
                        if !name.is_empty() {
                            vec.push(name);
                        }
                        if !value.is_empty() {
                            vec.push(value);
                        }
                        vec
                    },
                );
                if vec.is_empty() {
                    return "".to_owned();
                }
                if vec.len() == 1 {
                    return vec.concat();
                }
                vec.join("_")
            })
            .ok_or(HttpExtraError::InvalidUrl(format!(
                "The URL({url}) doesn't contain a valid path, file name is null"
            )))?;
        DownloadParam::try_new_default_download_dir(url, filename)
    }
}

impl TryFrom<&str> for DownloadParam {
    type Error = HttpExtraError;
    fn try_from(url: &str) -> Result<Self, Self::Error> {
        let url = Url::parse(url).map_err(|_| HttpExtraError::InvalidUrl(url.to_string()))?;
        DownloadParam::try_from(url)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DownloadStatus {
    // 下载还没有开始
    NotStarted,
    // 下载成功
    Success,
    // 下载失败
    Failed(String),
    // 跳过
    Skipped(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadSummary {
    param: DownloadParam,
    status: DownloadStatus,
    connet_length: u64,
    resumable: bool,
}

impl DownloadSummary {
    pub fn new(param: DownloadParam) -> Self {
        Self {
            param,
            status: DownloadStatus::NotStarted,
            connet_length: 0_u64,
            resumable: false,
        }
    }

    pub fn with_status(mut self, status: DownloadStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_connet_length(mut self, connet_length: u64) -> Self {
        self.connet_length = connet_length;
        self
    }

    pub fn with_resumable(mut self, resumable: bool) -> Self {
        self.resumable = resumable;
        self
    }

    pub fn status(&self) -> DownloadStatus {
        self.status.clone()
    }

    pub fn connet_length(&self) -> u64 {
        self.connet_length
    }

    pub fn resumable(&self) -> bool {
        self.resumable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_from_str() {
        let url = "https://www.rust-lang.org/";
        let download = DownloadParam::try_from(url).unwrap();
        assert_eq!(download.file_name, "");
        let url = "https://www.rust-lang.org/test.html";
        let download = DownloadParam::try_from(url).unwrap();
        assert_eq!(download.file_name, "test.html");
        let url = "https://www.rust-lang.org/file=test.html";
        let download = DownloadParam::try_from(url).unwrap();
        assert_eq!(download.file_name, "file_test.html");
        let url = "https://www.rust-lang.org/file1=test1.html&file2=test2.html";
        let download = DownloadParam::try_from(url).unwrap();
        assert_eq!(download.file_name, "file1_test1.html_file2_test2.html");
    }
}
