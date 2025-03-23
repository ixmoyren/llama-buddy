use crate::error::HttpExtraError;
use dir_extra::UserDirs;
use reqwest::Url;
use std::path::{Path, PathBuf};

#[derive(Debug, Eq, PartialEq)]
pub struct Download {
    // 下载路径
    fetch_from: Url,
    // 保存的文件名
    file_name: String,
    // 保存的路径
    save_to: PathBuf,
    // 重试次数
    retries: usize,
    // 是否断点续传
    resumable: bool,
}

impl Download {
    pub fn new(url: Url, file_name: impl AsRef<str>, save_to: impl AsRef<Path>) -> Self {
        Self {
            fetch_from: url,
            file_name: file_name.as_ref().to_owned(),
            save_to: save_to.as_ref().to_owned(),
            retries: 0,
            resumable: false,
        }
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
        Ok(Self {
            fetch_from: url,
            file_name,
            save_to,
            retries: 0_usize,
            resumable: false,
        })
    }

    pub fn with_retries(mut self, retries: usize) -> Self {
        self.retries = retries;
        self
    }

    pub fn with_resumable(mut self, resumable: bool) -> Self {
        self.resumable = resumable;
        self
    }
}

impl TryFrom<Url> for Download {
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
        Download::try_new_default_download_dir(url, filename)
    }
}

impl TryFrom<&str> for Download {
    type Error = HttpExtraError;
    fn try_from(url: &str) -> Result<Self, Self::Error> {
        let url = Url::parse(url).map_err(|_| HttpExtraError::InvalidUrl(url.to_string()))?;
        Download::try_from(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_from_str() {
        let url = "https://www.rust-lang.org/";
        let download = Download::try_from(url).unwrap();
        assert_eq!(download.file_name, "");
        let url = "https://www.rust-lang.org/test.html";
        let download = Download::try_from(url).unwrap();
        assert_eq!(download.file_name, "test.html");
        let url = "https://www.rust-lang.org/file=test.html";
        let download = Download::try_from(url).unwrap();
        assert_eq!(download.file_name, "file_test.html");
        let url = "https://www.rust-lang.org/file1=test1.html&file2=test2.html";
        let download = Download::try_from(url).unwrap();
        assert_eq!(download.file_name, "file1_test1.html_file2_test2.html");
    }
}
