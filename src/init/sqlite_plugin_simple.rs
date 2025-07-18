use http_extra::{download, download::DownloadParam, retry};
use reqwest::Client;
use std::{
    fs,
    fs::{File, create_dir_all},
    path::Path,
    str::FromStr,
    time::Duration,
};
use sys_extra::target::TargetTriple;
use tracing::debug;
use url::Url;
use zip::ZipArchive;

pub(crate) async fn download_sqlite_plugin(
    client: Client,
    back_off: impl IntoIterator<Item = Duration>,
    chunk_timeout: Option<u64>,
    sqlite_plugin_dir: &Path,
) -> anyhow::Result<()> {
    let (simple_url, simple_name) = get_simple_url();
    let param = DownloadParam::try_new(simple_url, simple_name, sqlite_plugin_dir)?
        .with_chunk_timeout(chunk_timeout);
    let summary = retry::spawn(back_off, async || {
        download::spawn(client.clone(), param.clone()).await
    })
    .await?;
    debug!("{summary:?}");
    let simple_path = sqlite_plugin_dir.join(simple_name);
    let simple_dir = sqlite_plugin_dir.join("libsimple");
    if !simple_dir.exists() {
        create_dir_all(simple_dir.as_path())?;
    }
    let simple_file = File::open(simple_path.as_path())?;
    let mut archive = ZipArchive::new(simple_file)?;
    archive.extract_unwrapped_root_dir(simple_dir.as_path(), |_path| true)?;
    fs::remove_file(simple_path.as_path())?;
    Ok(())
}

fn get_simple_url<'a>() -> (Url, &'a str) {
    let triple = TargetTriple::default();
    let simple_name = if triple.is_apple_darwin() {
        "libsimple-osx-x64.zip"
    } else if triple.is_x86_64_windows() {
        "libsimple-windows-x64.zip"
    } else if triple.is_i686_windows() {
        "libsimple-windows-x86.zip"
    } else if triple.is_aarch64_windows() {
        "libsimple-windows-arm64.zip"
    } else if triple.is_aarch64_linux() {
        "libsimple-linux-ubuntu-24.04-arm.zip"
    } else {
        "libsimple-linux-ubuntu-latest.zip"
    };
    let simple_url = Url::from_str(
        format!("https://github.com/wangfenjin/simple/releases/download/v0.5.2/{simple_name}")
            .as_str(),
    )
    .unwrap();
    (simple_url, simple_name)
}
