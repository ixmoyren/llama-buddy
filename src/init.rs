//! 初始化本地注册表

use clap::Args;
use reqwest::{Client, Proxy};
use std::{path::PathBuf, thread};
use sys_extra::dir::BaseDirs;

use url::Url;

pub async fn init_local_registry(args: InitArgs) {
    let InitArgs {
        remote_registry: remote,
        path,
        proxy,
        ..
    } = args;
    // 如果没有提供本地注册表所在目录，那么使用默认值
    let dir = if let Some(dir) = path {
        dir
    } else {
        let dirs = BaseDirs::new().unwrap();
        dirs.data_dir().join("llama-buddy")
    };
    let client_build = Client::builder()
        .pool_max_idle_per_host(thread::available_parallelism().map_or(1, |p| p.get()));
    let client_build = if let Some(proxy) = proxy {
        let proxy = Proxy::all(proxy).unwrap();
        client_build.proxy(proxy)
    } else {
        client_build
    };
    let client = client_build.build().unwrap();
    // 拉取数据库插件和词库，用于数据库检索
    let sqlite_dir = dir.join("sqlite");
    let sqlite_plugin_dir = sqlite_dir.join("plugin");
    download_sqlite_plugin(client.clone(), sqlite_plugin_dir).await;
    // 创建数据库
    // 创建数据库表，配置表，模型信息表
    // 拉取远程服务器上的数据，并且保存到模型信息表中
}

async fn download_sqlite_plugin(client: Client, sqlite_plugin_dir: PathBuf) {}

#[derive(Args)]
pub struct InitArgs {
    #[arg(
        short = 'r',
        long = "remote",
        default_value = "https://registry.ollama.ai/"
    )]
    pub remote_registry: Url,
    #[arg(
        short = 'p',
        long = "path",
        help = "The path where the local registry is located, the default path is `~/.local/share/llama-buddy`"
    )]
    pub path: Option<PathBuf>,
    #[arg(short = 'p', long = "proxy", help = "Proxy address")]
    pub proxy: Option<String>,
}
