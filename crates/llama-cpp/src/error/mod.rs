mod lora;

pub use lora::{LlamaAdapterLoraInitError, LlamaAdapterLoraRemoveError, LlamaAdapterLoraSetError};
use thiserror::Error;

/// llama_cpp 全部的错误汇总
#[derive(Debug, Eq, PartialEq, Error)]
pub enum LLamaCppError {
    #[error("{0}")]
    LlamaAdapterLoraInit(#[from] LlamaAdapterLoraInitError),
}
