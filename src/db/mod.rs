pub(crate) mod config;
mod llama_buddy;
pub(crate) mod model;

pub(crate) use llama_buddy::*;

pub enum CompletedStatus {
    #[allow(dead_code)]
    NotStarted,
    Completed,
    InProgress,
    Failed,
}

impl AsRef<str> for CompletedStatus {
    fn as_ref(&self) -> &str {
        match self {
            Self::NotStarted => "Not Started",
            Self::Completed => "Completed",
            Self::InProgress => "In Progress",
            Self::Failed => "Failed",
        }
    }
}
