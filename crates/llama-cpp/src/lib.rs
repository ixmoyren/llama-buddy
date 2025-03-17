pub mod batch;
pub mod context;
pub mod error;
pub mod ggml_numa;
pub mod model;
pub mod runtime;
pub mod sampler;
pub mod token;
pub mod vocabulary;

use crate::error::LLamaCppError;

pub type Result<T> = std::result::Result<T, LLamaCppError>;
