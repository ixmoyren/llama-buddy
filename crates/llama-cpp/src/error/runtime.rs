use thiserror::Error;

#[derive(Debug, Eq, PartialEq, Error)]
pub enum GgmlNumaStrategyError {
    #[error("Nonsupport ggml_numa_strategy: {0}")]
    NonsupportValue(String),
}
