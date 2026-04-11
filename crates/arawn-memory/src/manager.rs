//! Two-tier memory manager — global + workstream knowledge bases.
//!
//! `MemoryManager` is the single handle the rest of the system uses.
//! It abstracts the two-tier scoping and routes entities to the appropriate store.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tracing::{info, warn};

use crate::error::MemoryError;
use crate::store::MemoryStore;
use crate::types::{EntityType, Scope};
use crate::vector;

/// Two-tier memory manager holding global and workstream knowledge bases.
pub struct MemoryManager {
    /// Global KB: user preferences, cross-project facts, people.
    pub global: Arc<MemoryStore>,
    /// Workstream KB: project decisions, conventions, notes.
    pub workstream: Arc<MemoryStore>,
    /// Whether vector storage is initialized.
    vectors_enabled: bool,
}

impl MemoryManager {
    /// Open both KB tiers. Creates databases if they don't exist.
    /// `data_dir` is typically `~/.arawn/`.
    /// `ws_dir` is the workstream subdirectory name (e.g., "my-project-{uuid}").
    pub fn open(data_dir: &Path, ws_dir: &str, embedding_dims: Option<usize>) -> Result<Self, MemoryError> {
        // Initialize sqlite-vec globally (idempotent)
        if embedding_dims.is_some() {
            vector::init_vector_extension();
        }

        let global_path = data_dir.join("memory.db");
        let ws_path = data_dir.join("workstreams").join(ws_dir).join("memory.db");

        let global = Arc::new(MemoryStore::open(&global_path)?);
        let workstream = Arc::new(MemoryStore::open(&ws_path)?);

        let mut vectors_enabled = false;
        if let Some(dims) = embedding_dims {
            if let Err(e) = global.init_vectors(dims) {
                warn!(error = %e, "failed to init vectors on global KB");
            } else if let Err(e) = workstream.init_vectors(dims) {
                warn!(error = %e, "failed to init vectors on workstream KB");
            } else {
                vectors_enabled = true;
                info!(dims, "vector storage enabled on both KB tiers");
            }
        }

        info!(
            global = ?global_path,
            workstream = ?ws_path,
            vectors = vectors_enabled,
            "memory manager opened"
        );

        Ok(Self {
            global,
            workstream,
            vectors_enabled,
        })
    }

    /// Create a MemoryManager from pre-built stores (for testing).
    pub fn open_with_stores(global: Arc<MemoryStore>, workstream: Arc<MemoryStore>) -> Self {
        Self {
            global,
            workstream,
            vectors_enabled: false,
        }
    }

    /// Get the store for a given scope.
    pub fn store_for(&self, scope: Scope) -> &Arc<MemoryStore> {
        match scope {
            Scope::Global => &self.global,
            Scope::Workstream => &self.workstream,
        }
    }

    /// Get the store for a given entity type (uses default scope).
    pub fn store_for_type(&self, entity_type: EntityType) -> &Arc<MemoryStore> {
        self.store_for(entity_type.default_scope())
    }

    /// Whether vector storage is available.
    pub fn vectors_enabled(&self) -> bool {
        self.vectors_enabled
    }

    /// Retrieve entities matching keywords (by title FTS or tag match) from both tiers.
    /// Returns entities within the token budget, deduplicated by ID.
    pub fn retrieve_topical(
        &self,
        keywords: &[String],
        budget_tokens: usize,
    ) -> Vec<crate::types::Entity> {
        if keywords.is_empty() || budget_tokens == 0 {
            return Vec::new();
        }

        let mut seen = std::collections::HashSet::new();
        let mut results = Vec::new();
        let mut tokens_used = 0;

        // Search both tiers by FTS and tags
        for store in [&self.global, &self.workstream] {
            for keyword in keywords {
                // FTS search
                if let Ok(entities) = store.search(keyword, 10) {
                    for entity in entities {
                        if entity.superseded || seen.contains(&entity.id) {
                            continue;
                        }
                        let cost = (entity.title.len() + entity.content.as_ref().map(|c| c.len().min(80)).unwrap_or(0)) / 4;
                        if tokens_used + cost > budget_tokens {
                            continue;
                        }
                        seen.insert(entity.id);
                        tokens_used += cost;
                        results.push(entity);
                    }
                }

                // Tag search
                if let Ok(entities) = store.search_by_tags(&[keyword.clone()], 10) {
                    for entity in entities {
                        if entity.superseded || seen.contains(&entity.id) {
                            continue;
                        }
                        let cost = (entity.title.len() + entity.content.as_ref().map(|c| c.len().min(80)).unwrap_or(0)) / 4;
                        if tokens_used + cost > budget_tokens {
                            continue;
                        }
                        seen.insert(entity.id);
                        tokens_used += cost;
                        results.push(entity);
                    }
                }
            }
        }

        results
    }
}

/// Try to open a MemoryManager, returning None on failure (graceful degradation).
pub fn try_open_memory(
    data_dir: &Path,
    ws_dir: &str,
    embedding_dims: Option<usize>,
) -> Option<Arc<MemoryManager>> {
    match MemoryManager::open(data_dir, ws_dir, embedding_dims) {
        Ok(mgr) => Some(Arc::new(mgr)),
        Err(e) => {
            warn!(error = %e, "memory system unavailable — continuing without memory");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, MemoryManager) {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("workstreams/test-ws")).unwrap();
        let mgr = MemoryManager::open(tmp.path(), "test-ws", None).unwrap();
        (tmp, mgr)
    }

    fn setup_with_vectors() -> (TempDir, MemoryManager) {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("workstreams/test-ws")).unwrap();
        let mgr = MemoryManager::open(tmp.path(), "test-ws", Some(4)).unwrap();
        (tmp, mgr)
    }

    #[test]
    fn opens_both_stores() {
        let (tmp, mgr) = setup();
        assert!(tmp.path().join("memory.db").exists());
        assert!(tmp.path().join("workstreams/test-ws/memory.db").exists());

        // Both stores should be functional
        let entity = Entity::new(EntityType::Preference, "test pref");
        mgr.global.insert_entity(&entity).unwrap();
        assert!(mgr.global.get_entity(entity.id).unwrap().is_some());
    }

    #[test]
    fn scope_routing() {
        let (_tmp, mgr) = setup();

        // Preferences → global
        assert!(std::ptr::eq(
            Arc::as_ptr(mgr.store_for_type(EntityType::Preference)),
            Arc::as_ptr(&mgr.global)
        ));
        assert!(std::ptr::eq(
            Arc::as_ptr(mgr.store_for_type(EntityType::Person)),
            Arc::as_ptr(&mgr.global)
        ));

        // Decisions → workstream
        assert!(std::ptr::eq(
            Arc::as_ptr(mgr.store_for_type(EntityType::Decision)),
            Arc::as_ptr(&mgr.workstream)
        ));
        assert!(std::ptr::eq(
            Arc::as_ptr(mgr.store_for_type(EntityType::Convention)),
            Arc::as_ptr(&mgr.workstream)
        ));
        assert!(std::ptr::eq(
            Arc::as_ptr(mgr.store_for_type(EntityType::Fact)),
            Arc::as_ptr(&mgr.workstream)
        ));
        assert!(std::ptr::eq(
            Arc::as_ptr(mgr.store_for_type(EntityType::Note)),
            Arc::as_ptr(&mgr.workstream)
        ));
    }

    #[test]
    fn vectors_disabled_by_default() {
        let (_tmp, mgr) = setup();
        assert!(!mgr.vectors_enabled());
    }

    #[test]
    fn vectors_enabled_with_dims() {
        let (_tmp, mgr) = setup_with_vectors();
        assert!(mgr.vectors_enabled());

        // Should be able to store embeddings
        let entity = Entity::new(EntityType::Fact, "test");
        mgr.global.insert_entity(&entity).unwrap();
        mgr.global
            .store_embedding(entity.id, &[0.1, 0.2, 0.3, 0.4])
            .unwrap();
        assert!(mgr.global.has_embedding(entity.id).unwrap());
    }

    #[test]
    fn graceful_degradation() {
        // Non-writable path should return None
        let result = try_open_memory(Path::new("/nonexistent/path"), "ws", None);
        assert!(result.is_none());
    }

    #[test]
    fn stores_are_independent() {
        let (_tmp, mgr) = setup();

        let global_entity = Entity::new(EntityType::Preference, "global pref");
        let ws_entity = Entity::new(EntityType::Decision, "ws decision");

        mgr.global.insert_entity(&global_entity).unwrap();
        mgr.workstream.insert_entity(&ws_entity).unwrap();

        // Each store only sees its own entities
        assert!(mgr.global.get_entity(global_entity.id).unwrap().is_some());
        assert!(mgr.global.get_entity(ws_entity.id).unwrap().is_none());
        assert!(mgr.workstream.get_entity(ws_entity.id).unwrap().is_some());
        assert!(mgr.workstream.get_entity(global_entity.id).unwrap().is_none());
    }
}
