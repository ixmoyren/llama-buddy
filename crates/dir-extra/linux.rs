use crate::BaseDirs;
use std::{env, env::home_dir, ffi::OsString, path::PathBuf};

pub fn base_dirs() -> Option<BaseDirs> {
    let home = home_dir()?;
    let cache = from_env(env::var_os("XDG_CACHE_HOME"), || home.join(".cache"));
    let config = from_env(env::var_os("XDG_CONFIG_HOME"), || home.join(".config"));
    let config_local = config.clone();
    let data = from_env(env::var_os("XDG_DATA_HOME"), || home.join(".local/share"));
    let data_local = data.clone();
    let executable = Some(home.join(".local/bin"));
    let preference = None;
    let runtime = env::var_os("XDG_RUNTIME_DIRS").map(PathBuf::from);
    let state = Some(from_env(env::var_os("XDG_STATE_HOME"), || {
        home.join(".local/state")
    }));
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

fn from_env(var: Option<OsString>, f: impl FnOnce() -> PathBuf) -> PathBuf {
    var.map(PathBuf::from)
        .and_then(|path| {
            if path.is_dir() & path.is_absolute() {
                Some(path)
            } else {
                None
            }
        })
        .unwrap_or_else(f)
}
