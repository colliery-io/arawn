//! Memory store and embedder initialization for the start command.

use std::sync::Arc;

use anyhow::Result;

use arawn_config::EmbeddingProvider;
use arawn_llm::EmbedderSpec;
use arawn_memory::{MemoryStore, init_vector_extension};

use super::Context;

/// Phase 5: Initialize the embedding provider.
pub(super) async fn init_embedder(
    config: &arawn_config::ArawnConfig,
    ctx: &Context,
) -> Result<Arc<dyn arawn_llm::Embedder>> {
    let embedding_config = config.embedding.clone().unwrap_or_default();
    let embedder_spec = build_embedder_spec(&embedding_config);

    if ctx.verbose {
        println!(
            "Embedding provider: {:?} ({}d)",
            embedding_config.provider,
            embedding_config.effective_dimensions()
        );
    }

    let embedder = arawn_llm::build_embedder(&embedder_spec).await?;

    if ctx.verbose {
        println!("Embedder: {} ({}d)", embedder.name(), embedder.dimensions());
    }

    Ok(embedder)
}

/// Phase 7: Initialize the memory store with graph + vector extensions.
pub(super) fn init_memory_store(
    memory_cfg: &arawn_config::MemoryConfig,
    data_dir: &std::path::Path,
    embedder: &Arc<dyn arawn_llm::Embedder>,
    ctx: &Context,
) -> Option<Arc<MemoryStore>> {
    let memory_db_path = memory_cfg
        .database
        .clone()
        .map(|p| if p.is_relative() { data_dir.join(p) } else { p })
        .unwrap_or_else(|| data_dir.join("memory.db"));

    init_vector_extension();
    match MemoryStore::open(&memory_db_path) {
        Ok(mut store) => {
            let graph_db_path = memory_db_path.with_extension("graph.db");
            if let Err(e) = store.init_graph_at_path(&graph_db_path) {
                tracing::warn!("failed to init knowledge graph: {}", e);
            }
            if let Err(e) = store.init_vectors(embedder.dimensions(), embedder.name()) {
                tracing::warn!("failed to init vector store: {}", e);
            }
            store.wal_checkpoint();

            if ctx.verbose {
                println!("Memory store: {}", memory_db_path.display());
            }
            Some(Arc::new(store))
        }
        Err(e) => {
            tracing::warn!("failed to open memory store: {}", e);
            None
        }
    }
}

/// Build an `EmbedderSpec` from the application's `EmbeddingConfig`.
pub(super) fn build_embedder_spec(config: &arawn_config::EmbeddingConfig) -> EmbedderSpec {
    let provider = match config.provider {
        EmbeddingProvider::Local => "local",
        EmbeddingProvider::OpenAi => "openai",
        EmbeddingProvider::Mock => "mock",
    };

    let (openai_api_key, openai_model, openai_base_url) = config
        .openai
        .as_ref()
        .map(|c| {
            let ref_name = c.api_key_ref.as_deref().unwrap_or("OPENAI_API_KEY");
            let api_key = arawn_config::secrets::resolve_api_key_ref(ref_name).map(|r| r.value);
            (api_key, Some(c.model.clone()), c.base_url.clone())
        })
        .unwrap_or((None, None, None));

    let (local_model_path, local_tokenizer_path, local_model_url, local_tokenizer_url) = config
        .local
        .as_ref()
        .map(|c| {
            (
                c.model_path.clone(),
                c.tokenizer_path.clone(),
                c.model_url.clone(),
                c.tokenizer_url.clone(),
            )
        })
        .unwrap_or((None, None, None, None));

    EmbedderSpec {
        provider: provider.to_string(),
        openai_api_key,
        openai_model,
        openai_base_url,
        local_model_path,
        local_tokenizer_path,
        dimensions: Some(config.effective_dimensions()),
        local_model_url,
        local_tokenizer_url,
    }
}
