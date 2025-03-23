use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpExtraError {
    #[error(transparent)]
    Dirs(#[from] dir_extra::DirsError),
    #[error("There is no default download directory")]
    NoDownloadDir,
    #[error("{0}")]
    InvalidUrl(String),
}
