use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::{debug, info};

use arawn_embed::Embedder;
use arawn_memory::{
    ConfidenceSource, Entity, EntityType, MemoryManager, RelationType, Scope, StoreFactResult,
};

use crate::context::ToolContext;
use crate::error::EngineError;
use crate::tool::{Tool, ToolOutput};

/// Tool that stores knowledge in the KB with search-before-create deduplication.
pub struct MemoryStoreTool {
    memory: Arc<MemoryManager>,
    embedder: Option<Arc<dyn Embedder>>,
}

impl MemoryStoreTool {
    pub fn new(memory: Arc<MemoryManager>, embedder: Option<Arc<dyn Embedder>>) -> Self {
        Self { memory, embedder }
    }
}

#[async_trait]
impl Tool for MemoryStoreTool {
    fn name(&self) -> &str {
        "memory_store"
    }

    fn description(&self) -> &str {
        "Store knowledge in the knowledge base for cross-session memory. Uses search-before-create \
         to avoid duplicates — if the same fact already exists, it's reinforced instead of duplicated.\n\n\
         Entity types:\n\
         - **fact**: Extracted knowledge (\"project uses PostgreSQL 15\")\n\
         - **decision**: Choices made (\"went with microservices architecture\")\n\
         - **convention**: Patterns/rules (\"tests go inline, not in separate files\")\n\
         - **preference**: User preferences (\"prefers terse responses\") — stored globally\n\
         - **person**: Team members (\"Alice — backend lead\") — stored globally\n\
         - **note**: Freeform annotations\n\n\
         Use this when you learn something worth remembering across sessions."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Concise title for the knowledge entity"
                },
                "entity_type": {
                    "type": "string",
                    "enum": ["fact", "decision", "convention", "preference", "person", "note"],
                    "description": "Type of knowledge being stored"
                },
                "content": {
                    "type": "string",
                    "description": "Detailed content (markdown). Optional — title alone may suffice for simple facts."
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Categorization tags for filtering"
                },
                "scope": {
                    "type": "string",
                    "enum": ["global", "workstream"],
                    "description": "Which KB tier to store in. Defaults based on entity_type: preference/person → global, others → workstream."
                }
            },
            "required": ["title", "entity_type"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Tool("missing 'title' parameter".into()))?;

        let type_str = params
            .get("entity_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Tool("missing 'entity_type' parameter".into()))?;

        let entity_type = EntityType::from_str(type_str)
            .ok_or_else(|| EngineError::Tool(format!("unknown entity_type: '{type_str}'")))?;

        let content = params.get("content").and_then(|v| v.as_str());

        let tags: Vec<String> = params
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let scope = params
            .get("scope")
            .and_then(|v| v.as_str())
            .and_then(|s| match s {
                "global" => Some(Scope::Global),
                "workstream" => Some(Scope::Workstream),
                _ => None,
            })
            .unwrap_or_else(|| entity_type.default_scope());

        // Build entity
        let mut entity = Entity::new(entity_type, title)
            .with_confidence(ConfidenceSource::Stated)
            .with_tags(tags)
            .with_session(ctx.session_id);

        if let Some(c) = content {
            entity = entity.with_content(c);
        }

        // Route to appropriate store
        let store = self.memory.store_for(scope);

        // Search-before-create via store_fact
        let result = store
            .store_fact(&entity)
            .map_err(|e| EngineError::Tool(format!("memory store error: {e}")))?;

        // Embed if embedder available
        if let Some(ref embedder) = self.embedder {
            let text_to_embed = format!("{} {}", title, content.unwrap_or(""));
            match embedder.embed(&text_to_embed).await {
                Ok(embedding) => {
                    let entity_id = match &result {
                        StoreFactResult::Inserted { entity_id } => *entity_id,
                        StoreFactResult::Reinforced { entity_id, .. } => *entity_id,
                        StoreFactResult::Superseded { new_entity_id, .. } => *new_entity_id,
                    };
                    if let Err(e) = store.store_embedding(entity_id, &embedding) {
                        debug!(error = %e, "failed to store embedding (non-fatal)");
                    }
                }
                Err(e) => {
                    debug!(error = %e, "failed to embed entity (non-fatal)");
                }
            }
        }

        // Add extracted_from relation to session
        let entity_id = match &result {
            StoreFactResult::Inserted { entity_id } => *entity_id,
            StoreFactResult::Reinforced { entity_id, .. } => *entity_id,
            StoreFactResult::Superseded { new_entity_id, .. } => *new_entity_id,
        };

        // Create a session-reference entity ID from the session UUID
        // (We use the session ID directly as a relation target — it doesn't need
        // to be a stored entity, just a UUID for provenance tracking)
        let _ = store.add_relation(
            entity_id,
            RelationType::ExtractedFrom,
            ctx.session_id,
        );

        // Format output
        let scope_label = match scope {
            Scope::Global => "global",
            Scope::Workstream => "workstream",
        };

        match result {
            StoreFactResult::Inserted { entity_id } => {
                info!(id = %entity_id, title, scope = scope_label, "new entity stored");
                Ok(ToolOutput::success(format!(
                    "Stored new {type_str} in {scope_label} KB: \"{title}\" (id: {entity_id})"
                )))
            }
            StoreFactResult::Reinforced {
                entity_id,
                new_count,
            } => {
                info!(id = %entity_id, title, count = new_count, "entity reinforced");
                Ok(ToolOutput::success(format!(
                    "Reinforced existing {type_str}: \"{title}\" (now confirmed {new_count} times, id: {entity_id})"
                )))
            }
            StoreFactResult::Superseded {
                old_entity_id,
                new_entity_id,
            } => {
                info!(old = %old_entity_id, new = %new_entity_id, title, "entity superseded");
                Ok(ToolOutput::success(format!(
                    "Superseded old {type_str} with: \"{title}\" (old: {old_entity_id} → new: {new_entity_id})"
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Workstream;
    use arawn_memory::MemoryManager;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn setup() -> (TempDir, Arc<MemoryManager>, ToolContext) {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("workstreams/test-ws")).unwrap();
        let mgr = Arc::new(
            MemoryManager::open(tmp.path(), "test-ws", None).unwrap(),
        );
        let ws = Workstream::scratch(tmp.path());
        let ctx = ToolContext::new(&ws, Uuid::new_v4());
        (tmp, mgr, ctx)
    }

    #[tokio::test]
    async fn store_new_fact() {
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None);

        let result = tool
            .execute(&ctx, json!({"title": "Rust is fast", "entity_type": "fact"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("Stored new fact"));
        assert_eq!(mgr.workstream.count_all().unwrap(), 1);
    }

    #[tokio::test]
    async fn store_preference_goes_global() {
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None);

        tool.execute(&ctx, json!({"title": "Prefers terse output", "entity_type": "preference"}))
            .await
            .unwrap();

        assert_eq!(mgr.global.count_all().unwrap(), 1);
        assert_eq!(mgr.workstream.count_all().unwrap(), 0);
    }

    #[tokio::test]
    async fn store_decision_goes_workstream() {
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None);

        tool.execute(&ctx, json!({"title": "Use microservices", "entity_type": "decision"}))
            .await
            .unwrap();

        assert_eq!(mgr.global.count_all().unwrap(), 0);
        assert_eq!(mgr.workstream.count_all().unwrap(), 1);
    }

    #[tokio::test]
    async fn store_reinforces_duplicate() {
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None);

        tool.execute(&ctx, json!({"title": "Rust is fast", "entity_type": "fact"}))
            .await
            .unwrap();

        let result = tool
            .execute(&ctx, json!({"title": "Rust is fast", "entity_type": "fact"}))
            .await
            .unwrap();

        assert!(result.content.contains("Reinforced"));
        assert_eq!(mgr.workstream.count_all().unwrap(), 1);
    }

    #[tokio::test]
    async fn store_with_tags() {
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None);

        tool.execute(
            &ctx,
            json!({"title": "Tagged fact", "entity_type": "fact", "tags": ["rust", "perf"]}),
        )
        .await
        .unwrap();

        let results = mgr.workstream.search_by_tags(&["rust".into()], 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn store_with_explicit_scope_override() {
        let (_tmp, mgr, ctx) = setup();
        let tool = MemoryStoreTool::new(mgr.clone(), None);

        // Fact defaults to workstream, but override to global
        tool.execute(
            &ctx,
            json!({"title": "Global fact", "entity_type": "fact", "scope": "global"}),
        )
        .await
        .unwrap();

        assert_eq!(mgr.global.count_all().unwrap(), 1);
        assert_eq!(mgr.workstream.count_all().unwrap(), 0);
    }
}
