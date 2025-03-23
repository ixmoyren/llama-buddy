use crate::{BaseDirs, UserDirs};
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

pub fn user_dirs() -> Option<UserDirs> {
    let home = home_dir()?;
    let audio = Some(home.join("Music"));
    let desktop = Some(home.join("Desktop"));
    let document = Some(home.join("Documents"));
    let download = Some(home.join("Downloads"));
    let picture = Some(home.join("Pictures"));
    let public = Some(home.join("Public"));
    let video = Some(home.join("Movies"));
    let font = Some(home.join("Library/Fonts"));
    Some(UserDirs {
        home,
        audio,
        desktop,
        document,
        download,
        font,
        picture,
        public,
        template: None,
        video,
    })
}
