use std::ffi::FromBytesWithNulError;
use std::str::Utf8Error;
use thiserror::Error;

#[derive(Debug, Eq, PartialEq, Error)]
pub enum ChatTemplateError {
    #[error("An interior nul byte was found")]
    Nul(#[from] std::ffi::NulError),
    #[error("The original C string cannot be converted to utf8, {0}")]
    CannotCovertToUtf8(#[from] Utf8Error),
    #[error("The buffer is too small")]
    RetryWithLargerBuffer(usize),
    #[error("The model has no meta val - returned code, {0}")]
    MissingTemplate(i32),
    #[error("The string should have null byte, {0}")]
    FromBytesWithNul(#[from] FromBytesWithNulError),
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum ChatMessageError {
    #[error(transparent)]
    Nul(#[from] std::ffi::NulError),
}
