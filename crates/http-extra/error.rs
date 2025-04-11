use thiserror::Error;
use tokio::time::error::Elapsed;

#[derive(Debug, Error)]
pub enum HttpExtraError {
    #[error(transparent)]
    Dirs(#[from] dir_extra::DirsError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("There is no default download directory.")]
    NoDownloadDir,
    #[error(transparent)]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    #[error("{0}")]
    InvalidUrl(String),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("The path to save the file is not a directory.")]
    PathNotDirectory,
    #[error(transparent)]
    Elapsed(#[from] Elapsed),
    #[error(transparent)]
    HexDecode(#[from] faster_hex::Error),
}
