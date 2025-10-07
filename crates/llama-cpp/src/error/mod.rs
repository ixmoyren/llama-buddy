mod chat;
mod lora;
mod model;
mod token;

pub use chat::{ChatMessageError, ChatTemplateError};
pub use lora::{LlamaAdapterLoraInitError, LlamaAdapterLoraRemoveError, LlamaAdapterLoraSetError};
pub use model::{
    ApplyChatTemplateError, LlamaModelLoadError, StringConversionError, TokenConversionError,
};
use thiserror::Error;
pub use token::TokenTypeConversionError;

/// llama_cpp 全部的错误汇总
#[derive(Debug, Eq, PartialEq, Error)]
pub enum LLamaCppError {
    #[error("{0}")]
    ChatTemplate(#[from] ChatTemplateError),
    #[error("{0}")]
    ChatMessage(#[from] ChatMessageError),
    #[error("{0}")]
    TokenConversion(#[from] TokenConversionError),
    #[error("{0}")]
    StringConversion(#[from] StringConversionError),
    #[error("{0}")]
    TokenTypeConversion(#[from] TokenTypeConversionError),
    #[error("{0}")]
    ApplyChatTemplate(#[from] ApplyChatTemplateError),
    #[error("{0}")]
    LlamaModelLoad(#[from] LlamaModelLoadError),
    #[error("{0}")]
    LlamaAdapterLoraInit(#[from] LlamaAdapterLoraInitError),
}
