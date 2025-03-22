use std::{
    ffi::NulError, num::TryFromIntError, os::raw::c_int, path::PathBuf, string::FromUtf8Error,
};
use thiserror::Error;

#[derive(Debug, Eq, PartialEq, Error)]
pub enum TokenConversionError {
    #[error("Unknown Token Type")]
    UnknownType,
    #[error("Insufficient Buffer Space, {0}")]
    InsufficientBufferSpace(c_int),
    #[error("Can't convert to utf-8, {0}")]
    CannotConvertToUtf8(#[from] FromUtf8Error),
    #[error("Error buffer size is positive, {0}")]
    TryFromInt(#[from] TryFromIntError),
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum StringConversionError {
    #[error("The string contained a null byte and thus could not be converted to a c string, {0}")]
    Nul(#[from] NulError),
    #[error("Failed to convert a provided integer to a [`c_int`], {0}")]
    CIntConversion(#[from] TryFromIntError),
    #[error("{0}")]
    BufferCapacity(String),
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum ApplyChatTemplateError {
    #[error("The string contained a null byte and thus could not be converted to a c string, {0}")]
    Nul(#[from] NulError),
    #[error("The string could not be converted to utf-8, {0}")]
    FromUtf8(#[from] FromUtf8Error),
    #[error("Buffer size exceeds i32::MAX, {0}")]
    TryFromInt(#[from] TryFromIntError),
    #[error("Res is negative")]
    ResUnreasonable,
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum LlamaModelLoadError {
    #[error(
        "There was a null byte in a provided string, and thus it could not be converted to a C string, {0}"
    )]
    Nul(#[from] NulError),
    #[error("llama.cpp returned a nullptr")]
    NullReturn,
    #[error("Failed to convert path {0} to str")]
    PathToStr(PathBuf),
}
