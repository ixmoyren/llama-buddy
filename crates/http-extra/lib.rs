#![feature(return_type_notation)]

pub mod client;
pub mod download;
mod error;
pub mod retry;
pub mod sha256;

pub use error::*;

type Result<T, E = Error> = std::result::Result<T, E>;
