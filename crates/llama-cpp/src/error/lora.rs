use std::{ffi::NulError, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Eq, PartialEq, Error)]
pub enum LlamaAdapterLoraInitError {
    #[error(
        "There was a null byte in a provided string, and thus it could not be converted to a C string, {0}"
    )]
    Nul(#[from] NulError),
    #[error("null result from llama cpp")]
    NullReturn,
    #[error(
        "Failed to convert the path {0} to a rust str. This means the path was not valid for Unicode"
    )]
    PathToStr(PathBuf),
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum LlamaAdapterLoraSetError {
    #[error("Error code from llama.cpp, {0}")]
    ErrorReturn(i32),
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum LlamaAdapterLoraRemoveError {
    #[error("Error code from llama.cpp, {0}")]
    ErrorReturn(i32),
}
