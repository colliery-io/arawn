use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectionError {
    #[error("storage: {0}")]
    Storage(String),

    #[error("schema: {0}")]
    Schema(String),

    #[error("io: {0}")]
    Io(String),
}

impl From<rusqlite::Error> for ProjectionError {
    fn from(value: rusqlite::Error) -> Self {
        ProjectionError::Storage(value.to_string())
    }
}

impl From<std::io::Error> for ProjectionError {
    fn from(value: std::io::Error) -> Self {
        ProjectionError::Io(value.to_string())
    }
}

impl From<serde_json::Error> for ProjectionError {
    fn from(value: serde_json::Error) -> Self {
        ProjectionError::Schema(value.to_string())
    }
}
