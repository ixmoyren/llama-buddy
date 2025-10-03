use crate::{
    download::{Download, DownloadParam, DownloadStatus, DownloadSummary}, FetchHeadSnafu, FetchResourcesSnafu, GetChunkSnafu, IoOperationSnafu, Result,
    SetTimeoutSnafu,
};
use reqwest::{
    header::{HeaderMap, ACCEPT_RANGES, CONTENT_LENGTH, RANGE}, Client,
    Url,
};
use snafu::ResultExt;
use std::path::{Path, PathBuf};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    time::{timeout, Duration},
};
use tracing::{debug, error};

impl Download for Client {
    async fn get_content_length_and_accept_ranges(
        &self,
        url: Url,
    ) -> Result<(Option<u64>, Option<String>)> {
        let response = self
            .head(url.clone())
            .send()
            .await
            .context(FetchHeadSnafu)?;
        let headers = response.headers();
        if let Some(0) = content_length_value(headers) {
            // 使用 get 请求再尝试一次
            let response = self.get(url.clone()).send().await.context(FetchHeadSnafu)?;
            let headers = response.headers();
            Ok((content_length_value(headers), accept_ranges_value(headers)))
        } else {
            Ok((content_length_value(headers), accept_ranges_value(headers)))
        }
    }

    async fn fetch_file(&self, download: DownloadParam) -> Result<DownloadSummary> {
        let chunk_timeout = download.chunk_timeout;
        let url = download.fetch_from.clone();
        // 对下载目录和文件做预处理
        let dir = download.save_to.as_path();
        let file_name = download.file_name.as_str();
        let PreconditionFile {
            path,
            mut temp,
            temp_path,
        } = download_dir_precondition(dir, file_name).await?;

        let mut summary = DownloadSummary::new(download);

        let mut request = self.get(url.clone());
        let content_length_and_accept_ranges =
            self.get_content_length_and_accept_ranges(url).await?;
        let temp_len = temp
            .metadata()
            .await
            .context(IoOperationSnafu {
                message: "Failed to read metadata from temp file".to_owned(),
            })?
            .len();
        if let (Some(content_length), Some(accept_ranges)) = content_length_and_accept_ranges {
            let resumable = accept_ranges == "bytes";
            summary = summary
                .with_resumable(resumable)
                .with_connet_length(content_length);
            if !resumable {
                // 服务器不支持断点续传
                temp.set_len(0).await.context(IoOperationSnafu {
                    message: "Failed to clear the temp file".to_owned(),
                })?;
            }
            if content_length == temp_len {
                debug!(
                    "The size of the temporary file is the same as the size of the remote file. Just only need to do some post-processing related to the file."
                );
                download_dir_after_treatment(path, temp_path).await?;
                return Ok(summary.with_status(DownloadStatus::Success));
            }
            if resumable && temp_len > 0 {
                request = request.header(RANGE, format!("bytes={temp_len}-{content_length}"));
            }
        }
        let mut response = request.send().await.context(FetchResourcesSnafu)?;
        if !response.status().is_success() {
            error!("The response was abnormal during byte transmission.");
            return Ok(summary.with_status(DownloadStatus::Failed("Response exception".to_owned())));
        }
        if let Some(chunk_timeout) = chunk_timeout {
            let chunk_timeout = Duration::from_secs(chunk_timeout);
            while let Some(chunk) = timeout(chunk_timeout, response.chunk())
                .await
                .context(SetTimeoutSnafu)?
                .context(GetChunkSnafu)?
            {
                temp.write_all(&chunk).await.context(IoOperationSnafu {
                    message: "Failed to write to temp file with chunk timeout".to_owned(),
                })?;
                temp.flush().await.context(IoOperationSnafu {
                    message: "Failed to flush the temp file with chunk timeout".to_owned(),
                })?;
            }
        } else {
            while let Some(chunk) = response.chunk().await.context(GetChunkSnafu)? {
                temp.write_all(&chunk).await.context(IoOperationSnafu {
                    message: "Failed to write to temp file".to_owned(),
                })?;
                temp.flush().await.context(IoOperationSnafu {
                    message: "Failed to flush the temp file".to_owned(),
                })?;
            }
        }

        // 对保存的文件做后处理
        download_dir_after_treatment(path, temp_path).await?;
        Ok(summary.with_status(DownloadStatus::Success))
    }
}

struct PreconditionFile {
    // 路径
    path: PathBuf,
    // 暂存的文件
    temp: File,
    // 暂存的文件
    temp_path: PathBuf,
}

/// 文件保存前的预处理
/// 如果保存文件的目录不存在，那么创建目录，创建文件和暂存文件，并提供当前文件名
/// 判断当前文件夹下是否已经存在相同的名字的文件，有的重名，那么就提供新的文件名称
async fn download_dir_precondition(dir: &Path, file_name: &str) -> Result<PreconditionFile> {
    let (file_name, need_truncate) = if !dir.try_exists().context(IoOperationSnafu {
        message: format!(
            "Didn't determine whether this path({}) exists",
            dir.display()
        ),
    })? {
        // 保存文件的目录不存在，则创建
        tokio::fs::create_dir_all(dir)
            .await
            .context(IoOperationSnafu {
                message: format!("Failed to create a new directory({})", dir.display(),),
            })?;
        (file_name.to_owned(), true)
    } else {
        // 保存文件的目录存在，则判断文件是否有重名
        let mut entries = tokio::fs::read_dir(dir).await.context(IoOperationSnafu {
            message: format!("Failed to read directory({})", dir.display()),
        })?;
        let mut count = 0;
        while let Some(entry) = entries.next_entry().await.context(IoOperationSnafu {
            message: format!("Failed to read next entry in directory({})", dir.display()),
        })? {
            if entry.path().is_file() {
                let name = entry.file_name();
                if name == file_name {
                    count += 1;
                }
            }
        }
        if count > 0 {
            // 有重名，需要判断一下占位文件大小
            // 如果占位的文件大小为 0，那么可以认为是上一次中断，这个时候不需要重命名，继续上一次
            let file = dir.join(file_name);
            let file = tokio::fs::OpenOptions::new()
                .read(true)
                .open(&file)
                .await
                .context(IoOperationSnafu {
                    message: format!("Failed to open the file({})", file.display()),
                })?;
            let file_len = file
                .metadata()
                .await
                .context(IoOperationSnafu {
                    message: "Failed to get metadata".to_owned(),
                })?
                .len();
            if file_len == 0 {
                debug!("The file is not downloaded and does not need to be truncated.");
                (file_name.to_owned(), false)
            } else if let Some(index) = file_name.rfind(".") {
                let (left, right) = file_name.split_at(index);
                (format!("{left}_({count}){right}"), true)
            } else {
                (format!("{file_name}_({count})"), true)
            }
        } else {
            // 没有重名
            (file_name.to_owned(), true)
        }
    };

    let path = dir.join(PathBuf::from(&file_name));
    // 占位
    let _ = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(need_truncate)
        .open(&path)
        .await
        .context(IoOperationSnafu {
            message: format!("Failed to create a new file({})", path.display()),
        })?;
    let temp_name = format!("{file_name}.part");
    let temp_path = dir.join(PathBuf::from(&temp_name));
    let temp = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(need_truncate)
        .open(&temp_path)
        .await
        .context(IoOperationSnafu {
            message: format!("Failed to create a new temp file({})", temp_path.display()),
        })?;
    Ok(PreconditionFile {
        path,
        temp,
        temp_path,
    })
}

/// 下载完成后对文件进行后处理
async fn download_dir_after_treatment(file: PathBuf, temp: PathBuf) -> Result<()> {
    // 删除 file
    tokio::fs::remove_file(&file)
        .await
        .context(IoOperationSnafu {
            message: format!("Failed to remove file(\"{}\")", file.display()),
        })?;
    // 重名 temp
    tokio::fs::rename(&temp, &file)
        .await
        .context(IoOperationSnafu {
            message: format!(
                "Failed to rename file(\"{}\") to the new(\"{}\")",
                file.display(),
                temp.display(),
            ),
        })?;
    Ok(())
}

/// 从响应头中获取到 content-length
fn content_length_value(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|str| str.parse().ok())
}

/// 从响应头中获取 accept-ranges
fn accept_ranges_value(headers: &HeaderMap) -> Option<String> {
    headers
        .get(ACCEPT_RANGES)
        .and_then(|value| value.to_str().ok())
        .map(|str| str.to_owned())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::download::DownloadParam;
    use std::{str::FromStr, sync::LazyLock, time::Duration};

    pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
        let builder = Client::builder()
            .pool_max_idle_per_host(32)
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10));
        builder.build().expect("Couldn't build client")
    });

    #[tokio::test]
    async fn test_download() {
        let dir = tempfile::tempdir().unwrap();
        let url = Url::from_str("https://www.7-zip.org/a/7z2409-linux-x64.tar.xz").unwrap();
        let filename = "7z2409-linux-x64.tar.gz";
        let file_path = dir.path().join(filename);
        let download_param = DownloadParam::try_new(url, filename, dir.keep()).unwrap();
        let summary = CLIENT.fetch_file(download_param).await.unwrap();
        let file = tokio::fs::OpenOptions::new()
            .read(true)
            .open(&file_path)
            .await
            .unwrap();
        let file_len = file.metadata().await.unwrap().len();
        assert_eq!(summary.connet_length(), file_len);
    }
}
