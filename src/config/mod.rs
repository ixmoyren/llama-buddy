use crate::config::ConfigError::NotInterpret;
use clap::{Args, ValueEnum};
use http_extra::retry::strategy::{ExponentialBackoff, FibonacciBackoff, FixedInterval};
use llama_buddy_macro::IndexByField;
use reqwest::{Client as ReqwestClient, Proxy};
use serde::{Deserialize, Serialize};
use snafu::prelude::*;
use std::{
    collections::VecDeque,
    env,
    env::VarError,
    fs::{File, OpenOptions, create_dir_all},
    io::{Read, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use sys_extra::dir::BaseDirs;
use toml_edit::{DocumentMut, Table, value};
use url::Url;

const LLAMA_BUDDY_CONFIG: &str = include_str!("llama-buddy.toml");

/// 配置
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub data: Data,
    pub registry: Registry,
    pub model: Model,
}

#[derive(Debug, Snafu)]
pub enum ConfigError {
    #[snafu(display("{message}"))]
    IoOperation {
        message: String,
        source: std::io::Error,
    },
    #[snafu(display("Failed to deserialize file to config obj"))]
    Deserialize { source: toml_edit::de::Error },
    #[snafu(display("The path must be file"))]
    NotDir,
    #[snafu(display("Empty string isn't allowed"))]
    NotAllowedEmptyStr,
    #[snafu(display("Couldn't get the base dir from os"))]
    NotBaseDir { source: sys_extra::dir::Error },
    #[snafu(display("Couldn't interpret {key}"))]
    NotInterpret { key: String, source: VarError },
    #[snafu(display("Couldn't set proxy({proxy}) in reqwest client"))]
    ReqwestSetProxy {
        proxy: String,
        source: reqwest::Error,
    },
    #[snafu(display("Couldn't build reqwest client"))]
    ReqwestBuildClient { source: reqwest::Error },
}

impl Default for Config {
    fn default() -> Self {
        let config = toml_edit::de::from_str::<Config>(LLAMA_BUDDY_CONFIG);
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

    pub fn display(&self) -> Result<String, ConfigError> {
        let Self {
            data: Data { path },
            registry:
                Registry {
                    remote,
                    client: registry_client,
                },
            model:
                Model {
                    category,
                    client: model_client,
                },
        } = self;
        let mut doc = LLAMA_BUDDY_CONFIG
            .parse::<DocumentMut>()
            .expect("Invalid config");
        // 保存 data_path
        doc["data"]["path"] = value(path.to_str().unwrap_or(""));
        doc["registry"]["remote"] = value(remote.to_string());
        doc["model"]["category"] = value(category);
        if let Some(table) = doc["registry"]["client"].as_table_mut() {
            Self::client_table(table, registry_client);
            Self::sort_client_table(table);
        }

        if let Some(table) = doc["model"]["client"].as_table_mut() {
            Self::client_table(table, model_client);
            Self::sort_client_table(table);
        }
        Ok(doc.to_string())
    }

    fn client_table(table: &mut Table, client: &HttpClient) {
        let HttpClient {
            proxy,
            timeout,
            chunk_timeout,
            retry,
            back_off_strategy,
            back_off_time,
        } = client;
        if let Some(time) = back_off_time {
            let item = table
                .get_mut("back_off_time")
                .expect("Invalid config, no back_off_time item");
            *item = value(*time as i64);
        }

        if let Some(strategy) = back_off_strategy {
            let item = table
                .get_mut("back_off_strategy")
                .expect("Invalid config, no back_off_strategy item");
            *item = value(strategy.as_str());
        }

        if let Some(retry) = retry {
            let item = table
                .get_mut("retry")
                .expect("Invalid config, no retry item");
            *item = value(*retry as i64);
        }

        let mut has_chunk_timeout = false;
        if let Some(chunk_timeout) = chunk_timeout {
            let _ = table.insert("chunk_timeout", value(*chunk_timeout as i64));
            has_chunk_timeout = true;
        }

        let mut has_timeout = false;
        if let Some(timeout) = timeout {
            let _ = table.insert("timeout", value(*timeout as i64));
            has_timeout = true;
        }

        let mut has_proxy = false;
        if let Some(proxy) = proxy {
            let _ = table.insert("proxy", value(proxy));
            has_proxy = true;
        }

        let (retry_key, _) = table
            .get_key_value("retry")
            .expect("Default config doesn't have any retry_item");

        // 获取注释
        let retry_decor = retry_key.leaf_decor();
        let mut retry_decor_lines = retry_decor
            .prefix()
            .expect("Invalid config, no retry decor")
            .as_str()
            .unwrap_or_default()
            .split('\n')
            .map(ToOwned::to_owned)
            .collect::<VecDeque<String>>();

        if has_proxy {
            let proxy_decor = retry_decor_lines.pop_front().unwrap_or_default().to_owned() + "\n";
            let mut proxy_key = table
                .key_mut("proxy")
                .expect("Default config doesn't have any proxy_key");
            let proxy_key_decor = proxy_key.leaf_decor_mut();
            proxy_key_decor.set_prefix(proxy_decor);
        };

        if has_timeout {
            let timout_decor = if has_proxy {
                retry_decor_lines.pop_front().unwrap_or_default() + "\n"
            } else {
                let proxy_decor = retry_decor_lines.pop_front().unwrap_or_default();
                let timeout_decor = retry_decor_lines.pop_front().unwrap_or_default();
                format!("{proxy_decor}\n{timeout_decor}\n")
            };
            let mut timeout_key = table
                .key_mut("timeout")
                .expect("Default config doesn't have any timout_key");
            let timeout_key_decor = timeout_key.leaf_decor_mut();
            timeout_key_decor.set_prefix(timout_decor);
        }

        if has_chunk_timeout {
            let chunk_timout_decor = match (has_proxy, has_timeout) {
                (true, true) | (false, true) => {
                    retry_decor_lines.pop_front().unwrap_or_default() + "\n"
                }
                (true, false) => {
                    let timeout_decor = retry_decor_lines.pop_front().unwrap_or_default();
                    let chunk_timeout_decor = retry_decor_lines.pop_front().unwrap_or_default();
                    format!("{timeout_decor}\n{chunk_timeout_decor}\n")
                }
                (false, false) => {
                    let proxy_decor = retry_decor_lines.pop_front().unwrap_or_default();
                    let timeout_decor = retry_decor_lines.pop_front().unwrap_or_default();
                    let chunk_timeout_decor = retry_decor_lines.pop_front().unwrap_or_default();
                    format!("{proxy_decor}\n{timeout_decor}\n{chunk_timeout_decor}\n")
                }
            };
            let mut chunk_timeout_key = table
                .key_mut("chunk_timeout")
                .expect("Default config doesn't have any chunk_timeout_key");
            let timeout_key_decor = chunk_timeout_key.leaf_decor_mut();
            timeout_key_decor.set_prefix(chunk_timout_decor)
        }

        let new_retry_decor = retry_decor_lines.iter().fold(String::new(), |mut acc, x| {
            if !x.is_empty() {
                acc.push_str(x.as_str());
                acc.push_str("\n");
            }
            acc
        });
        let mut retry_key = table
            .key_mut("retry")
            .expect("Default config doesn't have any retry_key");
        let retry_decor = retry_key.leaf_decor_mut();
        retry_decor.set_prefix(new_retry_decor);
    }

    fn sort_client_table(table: &mut Table) {
        table.sort_values_by(|key1, _, key2, _| {
            let index1 = HttpClient::index_by_field(key1.get());
            let index2 = HttpClient::index_by_field(key2.get());
            index1.cmp(&index2)
        })
    }

    pub fn read_from_toml(path: &Path) -> Result<Config, ConfigError> {
        let mut file = File::open(path).context(IoOperationSnafu {
            message: format!(
                "Failed to open the config file, in the path({})",
                path.display()
            ),
        })?;
        let mut config = String::new();
        file.read_to_string(&mut config).context(IoOperationSnafu {
            message: "Failed to read the config file",
        })?;

        let config =
            toml_edit::de::from_str::<Config>(config.as_str()).context(DeserializeSnafu)?;
        Ok(config)
    }

    pub fn write_to_toml(&self, path: &Path) -> Result<(), ConfigError> {
        ensure!(path.exists() && path.is_file(), NotDirSnafu);
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(path)
            .context(IoOperationSnafu {
                message: format!(
                    "Failed to open the config file for changing, in the path({})?",
                    path.display()
                ),
            })?;
        let config_toml = self.display()?;
        file.write_all(config_toml.as_bytes())
            .context(IoOperationSnafu {
                message: "Failed to write all config to file".to_owned(),
            })
    }

    // 从环境变量中获取配置文件路径，并且转换成 Config
    // 1. 如果没有这个变量，那么使用默认的配置变量
    // 2. 如果有提供这个变量，则使用这个变量的路径
    pub fn try_config_path() -> Result<(Config, PathBuf), ConfigError> {
        let key = "LLAMA_BUDDY_CONFIG_PATH";
        match env::var(key) {
            Ok(val) => {
                ensure!(!val.is_empty(), NotAllowedEmptyStrSnafu);
                let path = PathBuf::from(val);
                ensure!(path.exists() && path.is_file(), NotDirSnafu);
                let config = Config::read_from_toml(&path)?;
                Ok((config, path))
            }
            Err(VarError::NotPresent) => {
                let base = BaseDirs::new().context(NotBaseDirSnafu)?;
                let config_dir = base.config_dir().join("llama-buddy");
                if !config_dir.exists() {
                    create_dir_all(config_dir.as_path()).context(IoOperationSnafu {
                        message: format!(
                            "Failed to create dir, in the path({})",
                            config_dir.display()
                        ),
                    })?;
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
            Err(error) => Err(NotInterpret {
                key: key.to_owned(),
                source: error,
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// 注册表所在位置
    pub path: PathBuf,
}

/// 注册表配置项
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Registry {
    /// 远程注册表路径
    pub remote: Url,
    /// 客户端配置
    pub client: HttpClient,
}

/// 模型配置
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// 模型版本
    pub category: String,
    /// 客户端配置
    pub client: HttpClient,
}

/// HTTP 客户端配置
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Args, IndexByField)]
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

impl BackOffStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackOffStrategy::Fibonacci => "Fibonacci",
            BackOffStrategy::Exponential => "Exponential",
            BackOffStrategy::Fixed => "Fixed",
        }
    }
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
            let p = Proxy::all(&p).context(ReqwestSetProxySnafu { proxy: p })?;
            client_build.proxy(p)
        } else {
            client_build
        };
        let client_build = if let Some(timeout) = self.timeout {
            client_build.timeout(Duration::from_secs(timeout))
        } else {
            client_build
        };
        let client = client_build.build().context(ReqwestBuildClientSnafu)?;
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

    pub fn build_chunk_timeout(&self) -> Option<u64> {
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

    #[test]
    fn display_config_add_proxy() {
        // 创建一个临时文件夹用来保存文件
        let mut config = Config::default();
        config.model.client.back_off_strategy = Some(BackOffStrategy::Exponential);
        config.model.client.proxy = Some(Url::parse("socket5://127.0.0.1:5555").unwrap().into());
        let config_str = r#"[data]
# 数据保存的位置
path = ""

[registry]
# 远程仓库的地址
remote = "https://registry.ollama.com/"

[registry.client]
# 访问代理设置 proxy = ""
# 请求超时设置， timeout = 10
# 块写入磁盘的超时设置，chunk_timeout = 5
# 重试次数
retry = 3
# 重试时使用的时间策略
back_off_strategy = "Fibonacci"
# 重试第一次的时间间隔，后续每次重试的时间间隔由上面的策略生成
back_off_time = 10000

[model]
# 模型默认提供的版本
category = "latest"

[model.client]
# 访问代理设置 proxy = ""
proxy = "socket5://127.0.0.1:5555"
# 请求超时设置， timeout = 10
# 块写入磁盘的超时设置，chunk_timeout = 5
# 重试次数
retry = 5
# 重试时使用的时间策略
back_off_strategy = "Exponential"
# 重试第一次的时间间隔，后续每次重试的时间间隔由上面的策略生成
back_off_time = 10000
"#;
        assert_eq!(config_str, config.display().unwrap());
    }

    #[test]
    fn display_config_add_timeout() {
        // 创建一个临时文件夹用来保存文件
        let mut config = Config::default();
        config.model.client.back_off_strategy = Some(BackOffStrategy::Exponential);
        config.model.client.timeout = Some(10);
        let config_str = r#"[data]
# 数据保存的位置
path = ""

[registry]
# 远程仓库的地址
remote = "https://registry.ollama.com/"

[registry.client]
# 访问代理设置 proxy = ""
# 请求超时设置， timeout = 10
# 块写入磁盘的超时设置，chunk_timeout = 5
# 重试次数
retry = 3
# 重试时使用的时间策略
back_off_strategy = "Fibonacci"
# 重试第一次的时间间隔，后续每次重试的时间间隔由上面的策略生成
back_off_time = 10000

[model]
# 模型默认提供的版本
category = "latest"

[model.client]
# 访问代理设置 proxy = ""
# 请求超时设置， timeout = 10
timeout = 10
# 块写入磁盘的超时设置，chunk_timeout = 5
# 重试次数
retry = 5
# 重试时使用的时间策略
back_off_strategy = "Exponential"
# 重试第一次的时间间隔，后续每次重试的时间间隔由上面的策略生成
back_off_time = 10000
"#;
        assert_eq!(config_str, config.display().unwrap());
    }

    #[test]
    fn display_config_add_chunk_timeout() {
        // 创建一个临时文件夹用来保存文件
        let mut config = Config::default();
        config.model.client.back_off_strategy = Some(BackOffStrategy::Exponential);
        config.model.client.chunk_timeout = Some(5);
        let config_str = r#"[data]
# 数据保存的位置
path = ""

[registry]
# 远程仓库的地址
remote = "https://registry.ollama.com/"

[registry.client]
# 访问代理设置 proxy = ""
# 请求超时设置， timeout = 10
# 块写入磁盘的超时设置，chunk_timeout = 5
# 重试次数
retry = 3
# 重试时使用的时间策略
back_off_strategy = "Fibonacci"
# 重试第一次的时间间隔，后续每次重试的时间间隔由上面的策略生成
back_off_time = 10000

[model]
# 模型默认提供的版本
category = "latest"

[model.client]
# 访问代理设置 proxy = ""
# 请求超时设置， timeout = 10
# 块写入磁盘的超时设置，chunk_timeout = 5
chunk_timeout = 5
# 重试次数
retry = 5
# 重试时使用的时间策略
back_off_strategy = "Exponential"
# 重试第一次的时间间隔，后续每次重试的时间间隔由上面的策略生成
back_off_time = 10000
"#;
        assert_eq!(config_str, config.display().unwrap());
    }

    #[test]
    fn display_config_add_chunk_timeout_and_proxy() {
        // 创建一个临时文件夹用来保存文件
        let mut config = Config::default();
        config.model.client.back_off_strategy = Some(BackOffStrategy::Exponential);
        config.model.client.proxy = Some(Url::parse("socket5://127.0.0.1:5555").unwrap().into());
        config.model.client.chunk_timeout = Some(5);
        config.registry.client.proxy = Some(Url::parse("socket5://127.0.0.1:5555").unwrap().into());
        config.registry.client.timeout = Some(10);
        config.registry.client.chunk_timeout = Some(5);
        let config_str = r#"[data]
# 数据保存的位置
path = ""

[registry]
# 远程仓库的地址
remote = "https://registry.ollama.com/"

[registry.client]
# 访问代理设置 proxy = ""
proxy = "socket5://127.0.0.1:5555"
# 请求超时设置， timeout = 10
timeout = 10
# 块写入磁盘的超时设置，chunk_timeout = 5
chunk_timeout = 5
# 重试次数
retry = 3
# 重试时使用的时间策略
back_off_strategy = "Fibonacci"
# 重试第一次的时间间隔，后续每次重试的时间间隔由上面的策略生成
back_off_time = 10000

[model]
# 模型默认提供的版本
category = "latest"

[model.client]
# 访问代理设置 proxy = ""
proxy = "socket5://127.0.0.1:5555"
# 请求超时设置， timeout = 10
# 块写入磁盘的超时设置，chunk_timeout = 5
chunk_timeout = 5
# 重试次数
retry = 5
# 重试时使用的时间策略
back_off_strategy = "Exponential"
# 重试第一次的时间间隔，后续每次重试的时间间隔由上面的策略生成
back_off_time = 10000
"#;
        assert_eq!(config_str, config.display().unwrap());
    }
}
