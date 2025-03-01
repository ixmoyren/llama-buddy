use thiserror::Error;

/// llama_cpp 全部的错误汇总
#[derive(Debug, Eq, PartialEq, Error)]
pub enum LLamaCppError {
    /// 后端错误
    #[error("{0}")]
    BackendError(#[from] BackendError),
}

#[derive(Debug, Eq, PartialEq, Error)]
pub enum BackendError {
    #[error("Nonsupport ggml_numa_strategy: {0}")]
    NonsupportGgmlNumaStrategy(String),
}
