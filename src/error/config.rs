use std::env::VarError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    TomlDe(#[from] toml_edit::de::Error),
    #[error(transparent)]
    TomlSe(#[from] toml_edit::ser::Error),
    #[error("No directory allowed here")]
    NotDir,
    #[error("Empty string isn't allowed")]
    NotAllowedEmptyStr,
    #[error(transparent)]
    NotHome(#[from] sys_extra::dir::DirsError),
    #[error("Couldn't interpret {key}: {error}")]
    NotInterpret { key: String, error: VarError },
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}
