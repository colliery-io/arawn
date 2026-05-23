use serde::{Deserialize, Serialize};

/// Configuration for the embedding provider.
/// Parsed from `[embeddings]` section in arawn.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider type: "local" (ONNX) or "api" (OpenAI-compatible).
    #[serde(default = "default_provider")]
    pub provider: String,

    /// Model name. For local: "all-MiniLM-L6-v2". For API: "text-embedding-3-small".
    #[serde(default = "default_model")]
    pub model: String,

    /// Embedding dimensions. Must match the model's output.
    #[serde(default = "default_dimensions")]
    pub dimensions: usize,

    /// For API provider: environment variable containing the API key.
    #[serde(default)]
    pub api_key_env: Option<String>,

    /// For API provider: base URL (default: https://api.openai.com/v1).
    #[serde(default)]
    pub api_base_url: Option<String>,

    /// For local provider: path to ONNX model file. If not set, downloads
    /// to ~/.arawn/models/ on first use.
    #[serde(default)]
    pub model_path: Option<String>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            dimensions: default_dimensions(),
            api_key_env: None,
            api_base_url: None,
            model_path: None,
        }
    }
}

fn default_provider() -> String {
    "local".to_string()
}

fn default_model() -> String {
    "all-MiniLM-L6-v2".to_string()
}

fn default_dimensions() -> usize {
    384
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.provider, "local");
        assert_eq!(config.model, "all-MiniLM-L6-v2");
        assert_eq!(config.dimensions, 384);
        assert!(config.api_key_env.is_none());
    }

    #[test]
    fn deserialize_local() {
        let toml = r#"
            provider = "local"
            model = "all-MiniLM-L6-v2"
            dimensions = 384
        "#;
        let config: EmbeddingConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.provider, "local");
        assert_eq!(config.dimensions, 384);
    }

    #[test]
    fn deserialize_api() {
        let toml = r#"
            provider = "api"
            model = "text-embedding-3-small"
            dimensions = 1536
            api_key_env = "OPENAI_API_KEY"
            api_base_url = "https://api.openai.com/v1"
        "#;
        let config: EmbeddingConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.provider, "api");
        assert_eq!(config.dimensions, 1536);
        assert_eq!(config.api_key_env.as_deref(), Some("OPENAI_API_KEY"));
    }

    #[test]
    fn deserialize_minimal() {
        let toml = "";
        let config: EmbeddingConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.provider, "local");
        assert_eq!(config.dimensions, 384);
    }
}
