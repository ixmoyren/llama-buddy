use crate::{
    batch::BatchError, context::ContextError, ggml_numa::StrategyError as GgmlNumaStrategyError,
    runtime::RuntimeError, token::TokenError, vocabulary::VocabularyTypeError,
};
use snafu::Snafu;

pub mod batch;
pub mod context;
pub mod error;
pub mod ggml_numa;
pub mod model;
pub mod runtime;
pub mod sampler;
pub mod token;
pub mod utils;
pub mod vocabulary;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(transparent)]
    Runtime { source: RuntimeError },
    #[snafu(transparent)]
    Token { source: TokenError },
    #[snafu(transparent)]
    Batch { source: BatchError },
    #[snafu(transparent)]
    GgmlNuma { source: GgmlNumaStrategyError },
    #[snafu(transparent)]
    VocabType { source: VocabularyTypeError },
    #[snafu(transparent)]
    Context { source: ContextError },
    #[snafu(whatever, display("{message}"))]
    GenericError {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error>, Some)))]
        source: Option<Box<dyn std::error::Error>>,
    },
}
