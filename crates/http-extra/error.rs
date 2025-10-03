use snafu::prelude::*;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Failed to fetch head"))]
    FetchHead { source: reqwest::Error },
    #[snafu(display("Failed to fetch resources"))]
    FetchResources { source: reqwest::Error },
    #[snafu(display("Failed to get default home directory"))]
    GetDefaultHomeDirectory { source: sys_extra::dir::Error },
    #[snafu(display("Failed to set timeout"))]
    SetTimeout { source: tokio::time::error::Elapsed },
    #[snafu(display("Failed to get chunk"))]
    GetChunk { source: reqwest::Error },
    #[snafu(display("{message}"))]
    IoOperation {
        message: String,
        source: std::io::Error,
    },
    #[snafu(whatever, display("{message}"))]
    GenericError {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error>, Some)))]
        source: Option<Box<dyn std::error::Error>>,
    },
}
