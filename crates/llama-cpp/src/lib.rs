extern crate core;

use crate::batch::BatchError;
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
    Batch { source: BatchError },
    #[snafu(whatever, display("{message}"))]
    GenericError {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error>, Some)))]
        source: Option<Box<dyn std::error::Error>>,
    },
}
