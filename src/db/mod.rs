pub(crate) mod config;
mod llama_buddy;
pub(crate) mod model;
mod rustyline_history;

pub(crate) use llama_buddy::*;
pub(crate) use rustyline_history::*;

pub enum CompletedStatus {
    #[allow(unused)]
    NotStarted,
    Completed,
    #[allow(unused)]
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
