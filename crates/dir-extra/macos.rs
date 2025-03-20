use crate::BaseDirs;
use std::env::home_dir;

pub fn base_dirs() -> Option<BaseDirs> {
    let home = home_dir()?;
    let cache = home.join("Library/Caches");
    let config = home.join("Library/Application Support");
    let config_local = config.clone();
    let data = config.clone();
    let data_local = config.clone();
    let executable = None;
    let preference = Some(home.join("Library/Preferences"));
    let runtime = None;
    let state = None;
    Some(BaseDirs {
        home,
        cache,
        config,
        config_local,
        data,
        data_local,
        executable,
        preference,
        runtime,
        state,
    })
}
