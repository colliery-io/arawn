use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtractionError {
    #[error("storage: {0}")]
    Storage(String),

    #[error("memory: {0}")]
    Memory(String),

    #[error("llm: {0}")]
    Llm(String),

    #[error("parse: {0}")]
    Parse(String),

    #[error("not found: {0}")]
    NotFound(String),
}

impl From<arawn_storage::StorageError> for ExtractionError {
    fn from(e: arawn_storage::StorageError) -> Self {
        ExtractionError::Storage(e.to_string())
    }
}

impl From<arawn_memory::MemoryError> for ExtractionError {
    fn from(e: arawn_memory::MemoryError) -> Self {
        ExtractionError::Memory(e.to_string())
    }
}

impl From<arawn_projections::ProjectionError> for ExtractionError {
    fn from(e: arawn_projections::ProjectionError) -> Self {
        ExtractionError::Storage(e.to_string())
    }
}

impl From<serde_json::Error> for ExtractionError {
    fn from(e: serde_json::Error) -> Self {
        ExtractionError::Parse(e.to_string())
    }
}
