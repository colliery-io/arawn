use thiserror::Error;

#[derive(Debug, Error)]
pub enum StewardError {
    #[error("storage: {0}")]
    Storage(String),

    #[error("memory: {0}")]
    Memory(String),

    #[error("journal: {0}")]
    Journal(String),

    #[error("subroutine `{name}`: {message}")]
    Subroutine { name: String, message: String },

    #[error("cap exceeded for {subroutine}: would apply {requested}, cap is {cap}")]
    CapExceeded {
        subroutine: String,
        requested: usize,
        cap: usize,
    },

    #[error("not found: {0}")]
    NotFound(String),

    #[error("parse: {0}")]
    Parse(String),
}

impl From<rusqlite::Error> for StewardError {
    fn from(e: rusqlite::Error) -> Self {
        StewardError::Storage(e.to_string())
    }
}

impl From<serde_json::Error> for StewardError {
    fn from(e: serde_json::Error) -> Self {
        StewardError::Parse(e.to_string())
    }
}

impl From<arawn_memory::MemoryError> for StewardError {
    fn from(e: arawn_memory::MemoryError) -> Self {
        StewardError::Memory(e.to_string())
    }
}
