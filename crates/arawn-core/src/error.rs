use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("workstream error: {0}")]
    Workstream(String),

    #[error("session error: {0}")]
    Session(String),
}
