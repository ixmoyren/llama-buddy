//! 初始化本地注册表

use clap::Args;
use std::path::PathBuf;
use url::Url;

pub async fn init_local_registry(args: InitArgs) {
    let InitArgs {
        remote_registry: remote,
        path,
        ..
    } = args;
    // 如果没有提供本地注册表所在目录，那么使用默认值
    let dir = if let Some(dir) = path {
        dir
    } else {
        let dirs = dir_extra::BaseDirs::new().unwrap();
        dirs.data_dir().join("llama-buddy")
    };
}

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
}
