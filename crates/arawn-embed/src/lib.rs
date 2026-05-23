//! Configurable embedding provider for arawn.
//!
//! Provides a trait-based embedding system with two backends:
//! - **Local**: ONNX Runtime with all-MiniLM-L6-v2 (default, works offline)
//! - **API**: OpenAI-compatible embedding endpoints
//!
//! Configuration lives in `arawn.toml` under `[embeddings]`.

mod api;
mod config;
mod error;
mod local;

pub use api::ApiEmbedder;
pub use config::EmbeddingConfig;
pub use error::EmbedError;
pub use local::LocalEmbedder;

use std::sync::Arc;

use async_trait::async_trait;

/// Trait for embedding text into dense vectors.
/// Implementations must be `Send + Sync` for use in async contexts.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Embed a single text string into a dense vector.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError>;

    /// Embed multiple texts in a batch. Default implementation calls `embed`
    /// sequentially — backends can override for efficiency.
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// The dimensionality of the embedding vectors produced.
    fn dimensions(&self) -> usize;
}

/// Create an embedder from configuration.
/// Returns `None` if embeddings are disabled.
pub fn create_embedder(config: &EmbeddingConfig) -> Result<Arc<dyn Embedder>, EmbedError> {
    match config.provider.as_str() {
        "local" => {
            let embedder = LocalEmbedder::new(config)?;
            Ok(Arc::new(embedder))
        }
        "api" | "openai" => {
            let embedder = ApiEmbedder::new(config)?;
            Ok(Arc::new(embedder))
        }
        other => Err(EmbedError::Config(format!(
            "unknown embedding provider: '{other}'. Use 'local' or 'api'."
        ))),
    }
}
