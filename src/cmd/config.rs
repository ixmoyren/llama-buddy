use crate::config::Config;
use sys_extra::dir::BaseDirs;

pub async fn output() {
    let mut config = Config::default();
    let base = BaseDirs::new().expect("Couldn't get the base dir from os");
    let data_path = base.data_dir().join("llama-buddy");
    config.data.path = data_path;
    let config_toml = config.display().expect("Couldn't get the config");
    println!("{config_toml}");
}
