//! API-based embedder using OpenAI-compatible embedding endpoints.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::config::EmbeddingConfig;
use crate::error::EmbedError;
use crate::Embedder;

const DEFAULT_API_BASE: &str = "https://api.openai.com/v1";

/// Embedder that calls an OpenAI-compatible embedding API.
pub struct ApiEmbedder {
    client: reqwest::Client,
    model: String,
    dimensions: usize,
    api_key: String,
    base_url: String,
}

impl ApiEmbedder {
    pub fn new(config: &EmbeddingConfig) -> Result<Self, EmbedError> {
        let api_key_env = config
            .api_key_env
            .as_deref()
            .unwrap_or("OPENAI_API_KEY");

        let api_key = std::env::var(api_key_env).map_err(|_| {
            EmbedError::Config(format!(
                "environment variable '{api_key_env}' not set (needed for API embeddings)"
            ))
        })?;

        let base_url = config
            .api_base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_API_BASE.to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            model: config.model.clone(),
            dimensions: config.dimensions,
            api_key,
            base_url,
        })
    }
}

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[async_trait]
impl Embedder for ApiEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let results = self.embed_batch(&[text]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| EmbedError::Api("empty response from embedding API".into()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        debug!(
            model = %self.model,
            count = texts.len(),
            "calling embedding API"
        );

        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: texts.iter().map(|t| t.to_string()).collect(),
        };

        let url = format!("{}/embeddings", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| EmbedError::Api(format!("request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unable to read body".into());
            return Err(EmbedError::Api(format!(
                "API returned {status}: {body}"
            )));
        }

        let result: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| EmbedError::Api(format!("failed to parse response: {e}")))?;

        let embeddings: Vec<Vec<f32>> = result.data.into_iter().map(|d| d.embedding).collect();

        // Validate dimensions
        for emb in &embeddings {
            if emb.len() != self.dimensions {
                return Err(EmbedError::Api(format!(
                    "expected {} dimensions, got {}",
                    self.dimensions,
                    emb.len()
                )));
            }
        }

        Ok(embeddings)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_embedder_requires_key() {
        // Use a key name that's guaranteed to not exist
        let config = EmbeddingConfig {
            provider: "api".into(),
            api_key_env: Some("ARAWN_TEST_NONEXISTENT_KEY_12345".into()),
            ..Default::default()
        };
        let result = ApiEmbedder::new(&config);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("not set"));
    }
}
