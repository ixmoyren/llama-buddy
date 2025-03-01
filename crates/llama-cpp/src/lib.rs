mod backend;
mod error;

pub use error::*;

pub type Result<T> = std::result::Result<T, LLamaCppError>;
