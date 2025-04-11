#![feature(return_type_notation)]
#![feature(let_chains)]

pub mod client;
pub mod download;
mod error;
pub mod retry;
pub mod sha256;

pub use error::*;
