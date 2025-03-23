use crate::{BaseDirs, UserDirs};
use std::{
    collections::HashMap,
    env,
    env::home_dir,
    ffi::OsString,
    fs,
    io::Read,
    os::unix::ffi::OsStringExt,
    path::PathBuf,
};

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

pub fn user_dirs() -> Option<UserDirs> {
    let home = home_dir()?;
    let data = from_env(env::var_os("XDG_DATA_HOME"), || home.join(".local/share"));
    let font = Some(data.join("fonts"));
    let mut user_dir_map = user_dir_map(&home);
    let audio = user_dir_map.remove("MUSIC");
    let desktop = user_dir_map.remove("DESKTOP");
    let document = user_dir_map.remove("DOCUMENTS");
    let download = user_dir_map.remove("DOWNLOAD");
    let picture = user_dir_map.remove("PICTURES");
    let public = user_dir_map.remove("PUBLICSHARE");
    let template = user_dir_map.remove("TEMPLATES");
    let video = user_dir_map.remove("VIDEOS");
    Some(UserDirs {
        home,
        audio,
        desktop,
        document,
        download,
        picture,
        public,
        template,
        video,
        font,
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

fn user_dir_map(home: &PathBuf) -> HashMap<String, PathBuf> {
    let user_dirs_file =
        from_env(env::var_os("XDG_CONFIG_HOME"), || home.join(".config")).join("user-dirs.dirs");
    let user_dirs_file = user_dirs_file.as_path();
    let mut file = fs::File::open(user_dirs_file).unwrap_or_else(|_| {
        panic!(
            "Could not open user-dirs.dirs({})",
            user_dirs_file.display()
        )
    });
    let mut bytes = Vec::with_capacity(1024);
    file.read_to_end(&mut bytes).unwrap_or_else(|_| {
        panic!(
            "Could not read user-dirs.dirs({})",
            user_dirs_file.display()
        )
    });

    let mut user_dirs = HashMap::new();

    for line in bytes.split(|b| *b == b'\n') {
        // 排除注释
        if line[..=0] == [b'#'] {
            continue;
        }
        let mut single_dir_found = false;
        // = 分隔，前面部分为 key，后面部分为 value
        let (key, value) = match split_once(line, b'=') {
            Some(kv) => kv,
            None => continue,
        };
        // 去除 key 首尾的空格和制表符
        let key = trim_blank(key);
        // 获取 key 中的关键字
        let key = if key.starts_with(b"XDG_") && key.ends_with(b"_DIR") {
            match str::from_utf8(&key[4..key.len() - 4]) {
                Ok(key) => key,
                Err(_) => continue,
            }
        } else {
            continue;
        };
        // xdg-user-dirs-update 使用双引号，这里只支持双引号
        let value = trim_blank(value);
        let mut value = if value.starts_with(b"\"") && value.ends_with(b"\"") {
            &value[1..value.len() - 1]
        } else {
            continue;
        };

        // 环境变量只允许绝对路径或者相对于 Home 目录的相对路径
        let is_relative = if value == b"$HOME/" {
            continue;
        } else if value.starts_with(b"$HOME/") {
            value = &value[b"$HOME/".len()..];
            true
        } else if value.starts_with(b"/") {
            false
        } else {
            continue;
        };

        let value = OsString::from_vec(shell_unescape(value));

        let path = if is_relative {
            home.join(value)
        } else {
            PathBuf::from(value)
        };
        user_dirs.insert(key.to_owned(), path);
    }

    user_dirs
}

fn split_once(bytes: &[u8], separator: u8) -> Option<(&[u8], &[u8])> {
    bytes
        .iter()
        .position(|b| *b == separator)
        .map(|i| (&bytes[..i], &bytes[i + 1..]))
}

fn trim_blank(bytes: &[u8]) -> &[u8] {
    let i = bytes
        .iter()
        .cloned()
        .take_while(|b| *b == b' ' || *b == b'\t')
        .count();
    let bytes = &bytes[i..];
    let i = bytes
        .iter()
        .cloned()
        .rev()
        .take_while(|b| *b == b' ' || *b == b'\t')
        .count();
    &bytes[..bytes.len() - i]
}

fn shell_unescape(escaped: &[u8]) -> Vec<u8> {
    // We assume that byte string was created by xdg-user-dirs-update which
    // escapes all characters that might potentially have special meaning,
    // so there is no need to check if backslash is actually followed by
    // $ ` " \ or a <newline>.

    let mut unescaped: Vec<u8> = Vec::with_capacity(escaped.len());
    let mut i = escaped.iter().cloned();

    while let Some(b) = i.next() {
        if b == b'\\' {
            if let Some(b) = i.next() {
                unescaped.push(b);
            }
        } else {
            unescaped.push(b);
        }
    }

    unescaped
}
