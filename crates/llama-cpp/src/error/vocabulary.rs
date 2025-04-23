use std::{ffi::NulError, num::TryFromIntError, str::Utf8Error};
use thiserror::Error;

#[derive(Debug, Eq, PartialEq, Error)]
pub enum VocabularyTypeConversionError {
    #[error("Unknown Value {0}")]
    UnknownValue(llama_cpp_sys::llama_vocab_type),
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum TokenizeError {
    #[error("The entered text contains NULL bytes, {0}")]
    TextInvalid(#[from] NulError),
    #[error("The text length({0}) exceeds the buffer")]
    TextTooLong(usize),
    #[error("Too Many Tokens({0})")]
    TokenTooMany(i32),
    #[error("{0}")]
    TokenizeFailed(String),
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum TokenToPieceError {
    #[error("Buffer size({0}) too small")]
    BufferTooSmall(i32),
    #[error("Unknown Token Type")]
    UnknownTokenType,
    #[error(transparent)]
    TryFromInt(#[from] TryFromIntError),
    #[error(transparent)]
    Uft8(#[from] Utf8Error),
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum DeTokenizeError {
    #[error("Buffer size({0}) too small")]
    BufferTooSmall(i32),
    #[error("Unknown Token Type")]
    UnknownTokenType,
    #[error(transparent)]
    TryFromInt(#[from] TryFromIntError),
    #[error(transparent)]
    Uft8(#[from] Utf8Error),
}
