use crate::error::ConfigError;
use clap::{Args, ValueEnum};
use http_extra::retry::strategy::{ExponentialBackoff, FibonacciBackoff, FixedInterval};
use reqwest::{Client as ReqwestClient, Proxy};
use serde::{Deserialize, Serialize};
use std::{
    env,
    env::VarError,
    fs::{File, OpenOptions, create_dir_all},
    io::{Read, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use sys_extra::dir::BaseDirs;
use tracing::info;
use url::Url;

const LLAMA_BUDDY_CONFIG: &str = include_str!("llama-buddy.toml");

pub async fn output() -> anyhow::Result<()> {
    let mut config = Config::default();
    let base = BaseDirs::new()?;
    let data_path = base.data_dir().join("llama-buddy");
    config.data.path = data_path;
    let config_toml = toml::to_string(&config)?;
    info!("{config_toml}");
    Ok(())
}

/// 配置
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub data: Data,
    pub registry: Registry,
    pub model: Model,
}

impl Default for Config {
    fn default() -> Self {
        let config = toml::from_str::<Config>(LLAMA_BUDDY_CONFIG);
        config.expect("The default configuration doesn't meet the requirements")
    }
}

impl Config {
    pub fn update_data(mut self, new: Data) -> Self {
        self.data = new;
        self
    }

    pub fn update_registry(mut self, new: Registry) -> Self {
        self.registry = new;
        self
    }

    pub fn update_model(mut self, new: Model) -> Self {
        self.model = new;
        self
    }

    pub fn read_from_toml(path: &Path) -> Result<Config, ConfigError> {
        let mut file = File::open(path)?;
        let mut config = String::new();
        file.read_to_string(&mut config)?;

        let config = toml::from_str::<Config>(config.as_str())?;
        Ok(config)
    }

    pub fn write_to_toml(&self, path: &Path) -> Result<(), ConfigError> {
        if path.exists() && path.is_dir() {
            return Err(ConfigError::NotDir);
        }
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        let config_toml = toml::to_string(self)?;
        file.write_all(config_toml.as_bytes())?;
        Ok(())
    }

    // 从环境变量中获取配置文件路径，并且转换成 Config
    // 1. 如果没有这个变量，那么使用默认的配置变量
    // 2. 如果有提供这个变量，则使用这个变量的路径
    pub fn try_config_path() -> Result<(Config, PathBuf), ConfigError> {
        let key = "LLAMA_BUDDY_CONFIG_PATH";
        match env::var(key) {
            Ok(val) => {
                if val.is_empty() {
                    return Err(ConfigError::NotAllowedEmptyStr);
                }
                let path = PathBuf::from(val);
                if path.exists() && path.is_dir() {
                    return Err(ConfigError::NotDir);
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
            Err(error) => Err(ConfigError::NotInterpret {
                key: key.to_owned(),
                error,
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// 注册表所在位置
    pub path: PathBuf,
}

impl Data {
    pub fn path(mut self, new: PathBuf) -> Self {
        self.path = new;
        self
    }
}

/// 注册表配置项
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Registry {
    /// 远程注册表路径
    pub remote: Url,
    /// 客户端配置
    pub client: HttpClient,
}

impl Registry {
    pub fn remote(mut self, new: Url) -> Self {
        self.remote = new;
        self
    }

    pub fn client(mut self, new: HttpClient) -> Self {
        self.client = new;
        self
    }
}

/// 模型配置
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// 模型版本
    pub category: String,
    /// 客户端配置
    pub client: HttpClient,
}

impl Model {
    pub fn update_category(mut self, new: String) -> Self {
        self.category = new;
        self
    }

    pub fn update_client(mut self, new: HttpClient) -> Self {
        self.client = new;
        self
    }
}

/// HTTP 客户端配置
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Args)]
pub struct HttpClient {
    /// 代理
    #[arg(long = "proxy", help = "Proxy address", required = false)]
    pub proxy: Option<String>,
    /// 请求超时，单位为秒
    #[arg(
        short = 't',
        long = "timeout",
        help = "Total timeout, specified in seconds",
        required = false
    )]
    pub timeout: Option<u64>,
    /// 文件写入超时，单位为秒
    #[arg(
        long = "chunk-timeout",
        help = "Chunk timeout, specified in seconds",
        required = false
    )]
    pub chunk_timeout: Option<u64>,
    /// 重试次数
    #[arg(long = "retry", help = "Retry times", required = false)]
    pub retry: Option<usize>,
    /// 重试回退策略
    #[arg(
        value_enum,
        long = "back-off-strategy",
        help = "Back off strategy",
        required = false
    )]
    pub back_off_strategy: Option<BackOffStrategy>,
    /// 回退时提供的延迟时间，单位为秒
    #[arg(
        long = "back_off_time",
        help = "Back off time, specified in seconds",
        required = false
    )]
    pub back_off_time: Option<u64>,
}

/// 重试回退策略
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, ValueEnum)]
pub enum BackOffStrategy {
    /// 斐波那契回退策略，每次重试等待的延迟时间，都是前两次的延迟时间的和
    #[value(
        help = "Fibonacci backoff strategy, where each retry delay is the sum of the previous two delays"
    )]
    Fibonacci,
    /// 指数回退，由重试次数决定指数
    #[value(
        help = "Exponential backoff, where the exponent is determined by the number of retry attempts"
    )]
    Exponential,
    /// 固定延迟时间回退
    #[value(help = "A backoff strategy with a fixed delay between retries")]
    Fixed,
}

impl HttpClient {
    pub fn merge(
        mut self,
        HttpClient {
            proxy,
            timeout,
            chunk_timeout,
            retry,
            back_off_strategy,
            back_off_time,
        }: HttpClient,
    ) -> Self {
        if proxy.is_some() {
            self.proxy = proxy;
        }
        if timeout.is_some() {
            self.timeout = timeout;
        }
        if chunk_timeout.is_some() {
            self.chunk_timeout = chunk_timeout;
        }
        if retry.is_some() {
            self.retry = retry;
        }
        if back_off_strategy.is_some() {
            self.back_off_strategy = back_off_strategy;
        }
        if back_off_time.is_some() {
            self.back_off_time = back_off_time;
        }
        self
    }

    pub fn build_client(&self) -> Result<ReqwestClient, ConfigError> {
        let client_build = ReqwestClient::builder()
            .pool_max_idle_per_host(thread::available_parallelism().map_or(1, |p| p.get()));
        let client_build = if let Some(p) = self.proxy.clone()
            && !p.is_empty()
        {
            let p = Proxy::all(p)?;
            client_build.proxy(p)
        } else {
            client_build
        };
        let client_build = if let Some(timeout) = self.timeout {
            client_build.timeout(Duration::from_secs(timeout))
        } else {
            client_build
        };
        let client = client_build.build()?;
        Ok(client)
    }

    pub fn build_back_off(&self) -> Box<dyn Iterator<Item = Duration>> {
        use BackOffStrategy::*;
        let retry = self.retry.unwrap_or(5);
        let time_out = self.back_off_time.unwrap_or(10000);
        match self.back_off_strategy {
            Some(Fixed) => Box::new(FixedInterval::from_millis(time_out).take(retry)),
            Some(Exponential) => Box::new(ExponentialBackoff::from_millis(time_out).take(retry)),
            Some(Fibonacci) | None => Box::new(FibonacciBackoff::from_millis(time_out).take(retry)),
        }
    }

    pub fn build_chunk_timout(&self) -> Option<u64> {
        self.chunk_timeout
    }
}

#[cfg(test)]
mod tests {
    use super::{BackOffStrategy, Config};
    use url::Url;

    #[test]
    fn get_default_config() {
        let config = Config::default();
        assert_eq!(
            config.registry.remote,
            Url::parse("https://registry.ollama.com").unwrap()
        );
        assert_eq!(
            config.registry.client.back_off_strategy,
            Some(BackOffStrategy::Fibonacci)
        );
        assert_eq!(config.model.client.back_off_time, Some(10000))
    }

    #[test]
    fn write_config_to_file() {
        // 创建一个临时文件夹用来保存文件
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        let file_path = dir.path().join("llama-buddy.toml");
        config.model.client.back_off_strategy = Some(BackOffStrategy::Exponential);
        config.write_to_toml(file_path.as_path()).unwrap();
        let config_form_file = Config::read_from_toml(file_path.as_path()).unwrap();
        assert_eq!(
            config_form_file.model.client.back_off_strategy,
            Some(BackOffStrategy::Exponential)
        );
        assert_eq!(
            config_form_file.registry.remote,
            Url::parse("https://registry.ollama.com").unwrap()
        );
    }
}
