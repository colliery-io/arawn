//! `LocalService` inherent methods backing the inspection reads — signals,
//! memory search, and the extraction log — for the GUI surfaces
//! (ARAWN-I-0070; ARAWN-T-0498 / ARAWN-T-0499). The trait shell in
//! `super::mod` delegates to these.

use arawn_service::{ExtractionLogEntry, MemorySearchResult, ServiceError, SignalDto};

use crate::lock_ext::Recover;

use super::LocalService;

impl LocalService {
    /// Recent extracted signals across every lens KB (ARAWN-T-0498). Opens
    /// each lens's `memory.db` read-only — no embedder needed, this is a
    /// ranked list rather than a vector search — labels each hit with its
    /// source lens, then merges newest-first and caps to `limit`. Lens KBs
    /// that fail to open are skipped, not fatal.
    pub(super) async fn list_signals_inner(
        &self,
        limit: usize,
    ) -> Result<Vec<SignalDto>, ServiceError> {
        let lenses_dir = self.data_dir.join("lenses");
        let Ok(entries) = std::fs::read_dir(&lenses_dir) else {
            return Ok(Vec::new());
        };
        let mut out: Vec<SignalDto> = Vec::new();
        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let db = dir.join("memory.db");
            if !db.exists() {
                continue;
            }
            let Ok(store) = arawn_memory::MemoryStore::open(&db) else {
                continue;
            };
            let lens = entry.file_name().to_string_lossy().to_string();
            for e in store.list_all_ranked(limit).unwrap_or_default() {
                out.push(SignalDto {
                    id: e.id.to_string(),
                    lens: lens.clone(),
                    entity_type: e.entity_type.as_str().to_string(),
                    title: e.title,
                    summary: e.content,
                    tags: e.tags,
                    confidence: e.confidence_source.as_str().to_string(),
                    updated_at: e.updated_at.to_rfc3339(),
                });
            }
        }
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        out.truncate(limit);
        Ok(out)
    }

    /// Read-only free-text search of the global KB (ARAWN-T-0499).
    pub(super) async fn memory_search_inner(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemorySearchResult>, ServiceError> {
        let memory = self
            .memory_manager
            .as_ref()
            .ok_or_else(|| ServiceError::Internal("Memory system not available".into()))?;
        let entities = memory
            .global
            .search(query, limit)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        Ok(entities
            .into_iter()
            .map(|e| MemorySearchResult {
                id: e.id.to_string(),
                entity_type: e.entity_type.as_str().to_string(),
                title: e.title,
                summary: e.content,
                updated_at: e.updated_at.to_rfc3339(),
            })
            .collect())
    }

    /// Recent extraction-log rows — per-(lens, projection) run outcomes,
    /// newest first (ARAWN-T-0499).
    pub(super) async fn extraction_log_inner(
        &self,
        limit: usize,
    ) -> Result<Vec<ExtractionLogEntry>, ServiceError> {
        let store = self.store.lock().recover();
        let log = arawn_storage::extraction_log_store::ExtractionLogStore::new(store.database());
        let rows = log
            .list_recent(limit)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| ExtractionLogEntry {
                lens: r.lens_name,
                projection_id: r.projection_id,
                run_id: r.run_id,
                outcome: r.outcome,
                reason: r.reason,
                dismissed: r.dismissed,
                updated_at: r.updated_at,
            })
            .collect())
    }
}
