use std::num::TryFromIntError;
use thiserror::Error;

#[derive(Debug, Eq, PartialEq, Error)]
pub enum KvCacheConversionError {
    #[error("Provided sequence id is too large for a i32")]
    SeqIdTooLarge(#[source] TryFromIntError),
    #[error("Provided start position is too large for a i32")]
    P0TooLarge(#[source] TryFromIntError),
    #[error("Provided end position is too large for a i32")]
    P1TooLarge(#[source] TryFromIntError),
}
