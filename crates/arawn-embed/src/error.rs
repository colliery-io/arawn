use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbedError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("model loading error: {0}")]
    ModelLoad(String),

    #[error("inference error: {0}")]
    Inference(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("tokenization error: {0}")]
    Tokenization(String),
}
