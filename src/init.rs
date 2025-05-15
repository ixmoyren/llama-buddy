//! 初始化本地注册表

use crate::{
    config::{Config, Data, HttpClient as HttpClientConfig, Registry},
    db,
    db::{
        CompletedStatus, ModelInfo, check_init_completed, check_insert_model_info_completed,
        check_libsimple, completed_init, insert_config, insert_model_info, update_libsimple,
    },
};
use anyhow::anyhow;
use clap::Args;
use http_extra::{download, download::DownloadParam, retry};
use reqwest::Client;
use scraper::{ElementRef, Html, Selector};
use std::{
    collections::VecDeque,
    env,
    env::VarError,
    fs,
    fs::{File, create_dir_all},
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};
use sys_extra::{dir::BaseDirs, target::TargetTriple};
use tracing::{debug, info};
use url::Url;
use zip::ZipArchive;

pub async fn init_local_registry(args: InitArgs) -> anyhow::Result<()> {
    let InitArgs {
        remote_registry: new_remote,
        path: new_data_path,
        client: http_client_config,
        saved,
        ..
    } = args;
    let (
        Config {
            data: Data { path },
            registry:
                Registry {
                    client: client_config,
                    remote,
                },
            model,
        },
        config_path,
    ) = get_config_path()?;
    let data_path = if let Some(data_path) = new_data_path {
        data_path
    } else {
        path
    };
    let client_config = if let Some(new) = http_client_config {
        client_config.merge(new)
    } else {
        client_config
    };
    let remote = if let Some(remote) = new_remote {
        remote
    } else {
        remote
    };
    let client = client_config.build_client()?;
    let back_off = client_config.build_back_off();
    let chunk_timeout = client_config.build_chunk_timout();
    // 拉取数据库插件和词库，用于数据库检索
    let sqlite_dir = data_path.join("sqlite");
    let sqlite_plugin_dir = sqlite_dir.join("plugin");
    // 创建数据库，配置表，模型信息表
    let conn = db::open(sqlite_dir, "llama-buddy.sqlite")?;
    // 检查一下有没有完成初始化
    if check_init_completed(&conn)? {
        info!("Initialization completed");
        return Ok(());
    }
    // 检查一下有没有从服务器上拉取 libsimple 插件
    if !check_libsimple(&conn)? {
        download_sqlite_plugin(
            client.clone(),
            back_off,
            chunk_timeout,
            sqlite_plugin_dir.as_path(),
        )
        .await?;
        update_libsimple(&conn)?;
    }
    let mut conn = conn;
    if !check_insert_model_info_completed(&conn)? {
        // 拉取远程服务器上的数据，并且保存到模型信息表中
        let (html, model_infos) = fetch_model_info(client.clone(), remote.clone()).await?;
        let html = html.as_bytes();
        let html_sha256 = http_extra::sha256::digest(html);
        insert_config(
            &conn,
            "model_library_html_digest".to_owned(),
            html_sha256.as_bytes().to_vec(),
        )?;
        insert_config(&conn, "model_library_html_data".to_owned(), html.to_vec())?;
        if insert_model_info(&mut conn, model_infos)? {
            // 完成初始化
            completed_init(&conn, CompletedStatus::Completed)?;
        } else {
            completed_init(&conn, CompletedStatus::InProgress)?;
        }
    }
    // 保存 cli 传入的参数到配置文件中
    if saved {
        let config = Config {
            data: Data { path: data_path },
            registry: Registry {
                client: client_config,
                remote,
            },
            model,
        };
        config.write_to_toml(config_path.as_path())?;
    }
    info!("Initialization completed");
    Ok(())
}

// 从环境变量中获取配置文件路径，并且转换成 Config
// 1. 如果没有这个变量，那么使用默认的配置变量
// 2. 如果有提供这个变量，则使用这个变量的路径
fn get_config_path() -> anyhow::Result<(Config, PathBuf)> {
    let key = "LLAMA_BUDDY_CONFIG_PATH";
    match env::var(key) {
        Ok(val) => {
            if val.is_empty() {
                return Err(anyhow!("Empty string isn't allowed"));
            }
            let path = PathBuf::from(val);
            if path.exists() && path.is_dir() {
                return Err(anyhow!("Dir path isn't allowed"));
            }
            let config = Config::read_from_toml(&path)?;
            Ok((config, path))
        }
        Err(VarError::NotPresent) => {
            let base = BaseDirs::new()?;
            let config_dir = base.config_dir().join("llama-buddy");
            if !config_dir.exists() {
                create_dir_all(config_dir.as_path())?;
            }
            let path = config_dir.join("llama-buddy.toml");
            let config = if !path.exists() {
                let data_path = base.data_dir().join("llama-buddy");
                let mut config = Config::default();
                config.data.path = data_path;
                config.write_to_toml(path.as_path())?;
                config
            } else {
                Config::read_from_toml(path.as_path())?
            };
            Ok((config, path))
        }
        Err(e) => Err(anyhow!("Couldn't interpret {key}: {e}")),
    }
}

async fn download_sqlite_plugin(
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

async fn fetch_model_info(
    client: Client,
    remote_registry: Url,
) -> anyhow::Result<(String, impl IntoIterator<Item = ModelInfo>)> {
    let library = remote_registry.join("/library?sort=newest")?;
    debug!("Fetching model information from {library:?}");
    let response = client.get(library).send().await?;
    let library_html = response.text().await?;
    let models = convert_to_model_infos(library_html.clone())?;
    Ok((library_html, models))
}

fn convert_to_model_infos(
    html: impl AsRef<str>,
) -> anyhow::Result<impl IntoIterator<Item = ModelInfo>> {
    let html = Html::parse_document(html.as_ref());
    let li_selector = get_selector("div#repo > ul li a")?;
    let title_selector = get_selector("div [x-test-model-title]")?;
    let introduction_selector = get_selector("p")?;
    let pull_count_selector = get_selector("span [x-test-pull-count]")?;
    let tag_count_selector = get_selector("span [x-test-tag-count]")?;
    let updated_time_selector = get_selector("span [x-test-updated]")?;
    let mut models = VecDeque::<ModelInfo>::new();

    for el in html.select(&li_selector) {
        let href = if let Some(href) = el.attr("href") {
            href.to_owned()
        } else {
            "".to_owned()
        };
        let Some(title_el) = el.select(&title_selector).next() else {
            continue;
        };
        let Some(title) = title_el.attr("title") else {
            continue;
        };
        let introduction = extract_text(&title_el, &introduction_selector);
        let pull_count = extract_text(&el, &pull_count_selector);
        let tag_count = extract_text(&el, &tag_count_selector);
        let updated_time = extract_text(&el, &updated_time_selector);
        let (Some(introduction), Some(pull_count), Some(tag_count), Some(updated_time)) =
            (introduction, pull_count, tag_count, updated_time)
        else {
            continue;
        };
        let model_info = ModelInfo {
            title: title.to_owned(),
            href,
            introduction,
            pull_count,
            tag_count,
            updated_time,
        };
        models.push_front(model_info);
    }
    Ok(models)
}

fn get_selector(str: &str) -> anyhow::Result<Selector> {
    Selector::parse(str).map_err(|err| anyhow!("Failed to create the selector, err: {err}"))
}

fn extract_text(el: &ElementRef, selector: &Selector) -> Option<String> {
    el.select(selector)
        .next()
        .map(|el| el.text().collect::<String>())
}

#[derive(Args)]
pub struct InitArgs {
    #[arg(
        short = 'r',
        long = "remote",
        help = "The remote registry address, the default value is `https://registry.ollama.com/`"
    )]
    pub remote_registry: Option<Url>,
    #[arg(
        short = 'p',
        long = "path",
        help = "The path where the local registry is located, the default path is `$DATA$/llama-buddy`"
    )]
    pub path: Option<PathBuf>,
    #[command(flatten)]
    pub client: Option<HttpClientConfig>,
    #[arg(
        short = 's',
        long = "save",
        help = "Save the options provided in the command line to a configuration file"
    )]
    pub saved: bool,
}
