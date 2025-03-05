use std::num::TryFromIntError;
use thiserror::Error;

#[derive(Debug, Eq, PartialEq, Error)]
pub enum BatchAddError {
    #[error("Insufficient Space of {0}")]
    InsufficientSpace(usize),
    #[error("Empty buffer")]
    EmptyBuffer,
    #[error(transparent)]
    TryFromInt(#[from] TryFromIntError),
}
