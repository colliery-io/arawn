//! `LocalService` inherent methods backing the `memory.*` portion of
//! `ArawnService`. The trait shell in `super::mod` delegates to these.

use arawn_service::{
    ForgetCandidate, ForgetResult, MemoryStoreResult, MemoryStoreSummary, MemorySummary,
    MemoryTypeCount, ServiceError,
};

use super::{LocalService, infer_entity_type};

impl LocalService {
    pub(super) async fn remember_fact_inner(
        &self,
        text: &str,
    ) -> Result<MemoryStoreResult, ServiceError> {
        use arawn_memory::{ConfidenceSource, Entity};

        let memory = self
            .memory_manager
            .as_ref()
            .ok_or_else(|| ServiceError::Internal("Memory system not available".into()))?;

        let (entity_type, title) = infer_entity_type(text);
        let mut entity = Entity::new(entity_type, &title).with_confidence(ConfidenceSource::Stated);
        if text.len() > title.len() + 5 {
            entity = entity.with_content(text);
        }

        // Use store_fact_embedded to auto-embed if embedder is available
        let result = memory.store_fact_embedded(&entity, None).await?;

        match result {
            arawn_memory::StoreFactResult::Inserted { entity_id } => {
                Ok(MemoryStoreResult::Inserted {
                    entity_id: entity_id.to_string(),
                    title,
                    entity_type: entity_type.as_str().to_string(),
                })
            }
            arawn_memory::StoreFactResult::Reinforced {
                entity_id,
                new_count,
            } => Ok(MemoryStoreResult::Reinforced {
                entity_id: entity_id.to_string(),
                title,
                count: new_count as u64,
            }),
            arawn_memory::StoreFactResult::Superseded {
                old_entity_id,
                new_entity_id,
            } => Ok(MemoryStoreResult::Superseded {
                old_id: old_entity_id.to_string(),
                new_id: new_entity_id.to_string(),
                title,
            }),
        }
    }

    pub(super) async fn memory_summary_inner(&self) -> Result<MemorySummary, ServiceError> {
        use arawn_memory::EntityType;

        let memory = self
            .memory_manager
            .as_ref()
            .ok_or_else(|| ServiceError::Internal("Memory system not available".into()))?;

        let types = [
            EntityType::Fact,
            EntityType::Decision,
            EntityType::Convention,
            EntityType::Preference,
            EntityType::Person,
            EntityType::Note,
        ];

        // ARAWN-I-0061: memory is global. Per-lens stores hold extractor-written
        // signals (recallable via `signal_*`), not memory — so this summary
        // covers global only.
        let mut global_counts = Vec::new();
        for et in &types {
            let g = memory.global.count_by_type(*et).unwrap_or(0);
            if g > 0 {
                global_counts.push(MemoryTypeCount {
                    entity_type: et.as_str().to_string(),
                    count: g as u64,
                });
            }
        }

        Ok(MemorySummary {
            global: MemoryStoreSummary {
                total: memory.global.count_all().unwrap_or(0) as u64,
                by_type: global_counts,
            },
        })
    }

    pub(super) async fn forget_entity_inner(
        &self,
        query: &str,
    ) -> Result<ForgetResult, ServiceError> {
        let memory = self
            .memory_manager
            .as_ref()
            .ok_or_else(|| ServiceError::Internal("Memory system not available".into()))?;

        // ARAWN-I-0061: memory is global; /forget only operates on global memory.
        // (Extracted signals in per-lens KBs are managed via signal/extraction
        // tools, not /forget.)
        let candidates: Vec<_> = memory
            .global
            .search(query, 5)
            .unwrap_or_default()
            .into_iter()
            .collect();

        if candidates.is_empty() {
            return Err(ServiceError::NotFound(format!(
                "No entities matching '{query}' found"
            )));
        }

        if candidates.len() == 1 {
            let entity = &candidates[0];
            match memory.global.delete_entity(entity.id) {
                Ok(true) => Ok(ForgetResult::Deleted {
                    title: entity.title.clone(),
                    entity_type: entity.entity_type.as_str().to_string(),
                    scope: "global".into(),
                }),
                Ok(false) => Err(ServiceError::NotFound("Entity not found".into())),
                Err(e) => Err(e.into()),
            }
        } else {
            Ok(ForgetResult::Ambiguous {
                candidates: candidates
                    .iter()
                    .map(|e| ForgetCandidate {
                        id: e.id.to_string(),
                        title: e.title.clone(),
                        entity_type: e.entity_type.as_str().to_string(),
                        scope: "global".into(),
                    })
                    .collect(),
            })
        }
    }
}
