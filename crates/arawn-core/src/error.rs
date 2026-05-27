use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("lens error: {0}")]
    Lens(String),

    #[error("session error: {0}")]
    Session(String),
}
