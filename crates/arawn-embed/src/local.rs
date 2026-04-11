//! Local ONNX-based embedder using sentence-transformers models.
//!
//! Runs entirely locally via ONNX Runtime — no API calls needed.
//! Model files are downloaded to ~/.arawn/models/ on first use.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::Tensor;
use tracing::info;

use crate::config::EmbeddingConfig;
use crate::error::EmbedError;
use crate::Embedder;

const MAX_TOKENS: usize = 512;

/// HuggingFace repo base for downloading model files.
const HF_REPO_BASE: &str = "https://huggingface.co/sentence-transformers";

/// Local ONNX-based embedder. Thread-safe via internal Mutex on the
/// ORT session (Session::run requires &mut self).
pub struct LocalEmbedder {
    session: Mutex<Session>,
    tokenizer: tokenizers::Tokenizer,
    dimensions: usize,
}

// Safety: Mutex<Session> serializes access. Tokenizer is Send+Sync.
unsafe impl Send for LocalEmbedder {}
unsafe impl Sync for LocalEmbedder {}

impl LocalEmbedder {
    pub fn new(config: &EmbeddingConfig) -> Result<Self, EmbedError> {
        let model_dir = resolve_model_dir(config)?;
        let model_path = model_dir.join("model.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        // Download model files if not present
        if !model_path.exists() || !tokenizer_path.exists() {
            download_model_files(&model_dir, &config.model)?;
        }

        info!(
            model = %config.model,
            path = ?model_path,
            "loading local embedding model"
        );

        let session = Session::builder()
            .map_err(|e| EmbedError::ModelLoad(format!("ORT session builder: {e}")))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| EmbedError::ModelLoad(format!("optimization level: {e}")))?
            .with_intra_threads(1)
            .map_err(|e| EmbedError::ModelLoad(format!("thread config: {e}")))?
            .commit_from_file(&model_path)
            .map_err(|e| EmbedError::ModelLoad(format!("load ONNX model: {e}")))?;

        let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| EmbedError::ModelLoad(format!("load tokenizer: {e}")))?;

        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            dimensions: config.dimensions,
        })
    }

    /// Run inference on a batch of texts. Returns mean-pooled, normalized embeddings.
    fn run_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Tokenize batch
        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| EmbedError::Tokenization(format!("{e}")))?;

        let batch_size = encodings.len();
        let max_len = encodings
            .iter()
            .map(|e| e.get_ids().len().min(MAX_TOKENS))
            .max()
            .unwrap_or(0);

        // Build padded flat arrays (batch_size * max_len)
        let mut input_ids_flat = vec![0i64; batch_size * max_len];
        let mut attention_mask_flat = vec![0i64; batch_size * max_len];
        let mut token_type_ids_flat = vec![0i64; batch_size * max_len];

        for (i, enc) in encodings.iter().enumerate() {
            let ids = enc.get_ids();
            let mask = enc.get_attention_mask();
            let types = enc.get_type_ids();
            let seq_len = ids.len().min(MAX_TOKENS);
            let offset = i * max_len;

            for j in 0..seq_len {
                input_ids_flat[offset + j] = ids[j] as i64;
                attention_mask_flat[offset + j] = mask[j] as i64;
                token_type_ids_flat[offset + j] = types[j] as i64;
            }
        }

        // Create tensors using (shape, Vec) tuple form — avoids ndarray version issues
        let shape = [batch_size, max_len];
        let input_ids_tensor = Tensor::from_array((shape, input_ids_flat.clone()))
            .map_err(|e| EmbedError::Inference(format!("input_ids tensor: {e}")))?;
        let attention_mask_tensor = Tensor::from_array((shape, attention_mask_flat.clone()))
            .map_err(|e| EmbedError::Inference(format!("attention_mask tensor: {e}")))?;
        let token_type_ids_tensor = Tensor::from_array((shape, token_type_ids_flat))
            .map_err(|e| EmbedError::Inference(format!("token_type_ids tensor: {e}")))?;

        // Run inference
        let mut session = self.session.lock().unwrap();
        let outputs = session
            .run(ort::inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor,
                "token_type_ids" => token_type_ids_tensor,
            ])
            .map_err(|e| EmbedError::Inference(format!("inference: {e}")))?;

        // Extract output: (shape, data) where shape = [batch, seq_len, hidden_dim]
        let (out_shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| EmbedError::Inference(format!("extract output: {e}")))?;

        let seq_len_out = out_shape[1] as usize;
        let hidden_dim = out_shape[2] as usize;
        let stride_batch = seq_len_out * hidden_dim;

        // Mean pooling with attention mask + L2 normalization
        let mut results = Vec::with_capacity(batch_size);

        for i in 0..batch_size {
            let mut sum = vec![0.0f32; hidden_dim];
            let mut count = 0.0f32;

            for j in 0..seq_len_out.min(max_len) {
                let mask_val = attention_mask_flat[i * max_len + j];
                if mask_val > 0 {
                    let offset = i * stride_batch + j * hidden_dim;
                    for k in 0..hidden_dim {
                        sum[k] += data[offset + k];
                    }
                    count += 1.0;
                }
            }

            if count > 0.0 {
                for v in &mut sum {
                    *v /= count;
                }
            }

            // L2 normalize
            let norm: f32 = sum.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for v in &mut sum {
                    *v /= norm;
                }
            }

            results.push(sum);
        }

        Ok(results)
    }
}

#[async_trait]
impl Embedder for LocalEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let results = self.run_batch(&[text])?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| EmbedError::Inference("empty result".into()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        const CHUNK_SIZE: usize = 32;
        let mut all = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(CHUNK_SIZE) {
            all.extend(self.run_batch(chunk)?);
        }
        Ok(all)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

fn resolve_model_dir(config: &EmbeddingConfig) -> Result<PathBuf, EmbedError> {
    if let Some(ref path) = config.model_path {
        return Ok(PathBuf::from(path));
    }
    // Use ARAWN_DATA_DIR if set, otherwise ~/.arawn
    let base = std::env::var("ARAWN_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".arawn")
        });
    Ok(base.join("models").join(&config.model))
}

fn download_model_files(model_dir: &Path, model_name: &str) -> Result<(), EmbedError> {
    std::fs::create_dir_all(model_dir)
        .map_err(|e| EmbedError::ModelLoad(format!("create model dir: {e}")))?;

    let base_url = format!("{HF_REPO_BASE}/{model_name}/resolve/main");

    let files = [
        ("onnx/model.onnx", "model.onnx"),
        ("tokenizer.json", "tokenizer.json"),
    ];

    for (remote, local) in &files {
        let local_path = model_dir.join(local);
        if !local_path.exists() {
            let url = format!("{base_url}/{remote}");
            info!(url = %url, "downloading model file (first-time setup)");
            let response = reqwest::blocking::get(&url)
                .map_err(|e| EmbedError::ModelLoad(format!("download {url}: {e}")))?;
            if !response.status().is_success() {
                return Err(EmbedError::ModelLoad(format!(
                    "download {url}: HTTP {}", response.status()
                )));
            }
            let bytes = response.bytes()
                .map_err(|e| EmbedError::ModelLoad(format!("read response: {e}")))?;
            std::fs::write(&local_path, &bytes)
                .map_err(|e| EmbedError::ModelLoad(format!("write {}: {e}", local_path.display())))?;
            info!(path = ?local_path, bytes = bytes.len(), "downloaded");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_default_dir() {
        let config = EmbeddingConfig::default();
        let dir = resolve_model_dir(&config).unwrap();
        assert!(dir.to_string_lossy().contains("all-MiniLM-L6-v2"));
    }

    #[test]
    fn resolve_custom_dir() {
        let config = EmbeddingConfig {
            model_path: Some("/tmp/my-model".into()),
            ..Default::default()
        };
        let dir = resolve_model_dir(&config).unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/my-model"));
    }
}
