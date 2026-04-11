use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::debug;

use arawn_embed::Embedder;
use arawn_memory::{Entity, EntityType, MemoryManager, MemoryStore, RelationType};

use crate::tool::{Tool, ToolCategory, ToolError, ToolOutput};

/// Tool that searches the knowledge base using composite retrieval:
/// semantic similarity + FTS5 text search + tag filtering + graph expansion.
pub struct MemorySearchTool {
    memory: Arc<MemoryManager>,
    embedder: Option<Arc<dyn Embedder>>,
}

impl MemorySearchTool {
    pub fn new(memory: Arc<MemoryManager>, embedder: Option<Arc<dyn Embedder>>) -> Self {
        Self { memory, embedder }
    }
}

#[async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &str {
        "memory_search"
    }

    fn description(&self) -> &str {
        "Search the knowledge base for stored facts, decisions, conventions, preferences, and notes. \
         Uses semantic similarity + text search for best recall.\n\n\
         Use this when you need to check what's already known before making assumptions."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Memory
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural language search query"
                },
                "entity_type": {
                    "type": "string",
                    "enum": ["fact", "decision", "convention", "preference", "person", "note"],
                    "description": "Filter results to a specific entity type"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Filter by tag intersection"
                },
                "scope": {
                    "type": "string",
                    "enum": ["global", "workstream", "both"],
                    "description": "Which KB tier to search (default: both)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results (default: 10)"
                },
                "include_related": {
                    "type": "boolean",
                    "description": "Include graph-connected entities (default: false)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'query' parameter".into()))?;

        let entity_type = params
            .get("entity_type")
            .and_then(|v| v.as_str())
            .and_then(EntityType::from_str);

        let tags: Option<Vec<String>> = params
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

        let scope = params
            .get("scope")
            .and_then(|v| v.as_str())
            .unwrap_or("both");

        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let include_related = params
            .get("include_related")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Collect results from each store, keyed by entity ID to deduplicate
        let mut scored: HashMap<uuid::Uuid, ScoredEntity> = HashMap::new();

        let stores_to_search: Vec<(&Arc<MemoryStore>, &str)> = match scope {
            "global" => vec![(&self.memory.global, "global")],
            "workstream" => vec![(&self.memory.workstream, "workstream")],
            _ => vec![
                (&self.memory.global, "global"),
                (&self.memory.workstream, "workstream"),
            ],
        };

        for (store, store_label) in &stores_to_search {
            // FTS5 search
            let fts_results = if let Some(et) = entity_type {
                store.search_by_type(query, et, limit * 2)
            } else {
                store.search(query, limit * 2)
            }
            .map_err(|e| ToolError::ExecutionFailed(format!("FTS search error: {e}")))?;

            for (rank, entity) in fts_results.iter().enumerate() {
                let fts_score = 1.0 / (1.0 + rank as f32); // rank-based score
                let confidence = entity.confidence_score();
                let entry = scored.entry(entity.id).or_insert_with(|| ScoredEntity {
                    entity: entity.clone(),
                    fts_score: 0.0,
                    semantic_score: 0.0,
                    confidence,
                    source: store_label.to_string(),
                    related: Vec::new(),
                });
                entry.fts_score = fts_score;
            }

            // Semantic search (if embedder available)
            if let Some(ref embedder) = self.embedder {
                match embedder.embed(query).await {
                    Ok(query_embedding) => {
                        let sim_results = store
                            .search_similar(&query_embedding, limit * 2)
                            .map_err(|e| ToolError::ExecutionFailed(format!("vector search error: {e}")))?;

                        for result in &sim_results {
                            let semantic_score = 1.0 / (1.0 + result.distance);
                            if let Ok(Some(entity)) = store.get_entity(result.entity_id) {
                                if entity.superseded {
                                    continue;
                                }
                                if let Some(et) = entity_type {
                                    if entity.entity_type != et {
                                        continue;
                                    }
                                }
                                let confidence = entity.confidence_score();
                                let entry = scored
                                    .entry(entity.id)
                                    .or_insert_with(|| ScoredEntity {
                                        entity: entity.clone(),
                                        fts_score: 0.0,
                                        semantic_score: 0.0,
                                        confidence,
                                        source: store_label.to_string(),
                                        related: Vec::new(),
                                    });
                                entry.semantic_score = semantic_score;
                            }
                        }
                    }
                    Err(e) => {
                        debug!(error = %e, "semantic search failed (falling back to FTS only)");
                    }
                }
            }

            // Tag filter
            if let Some(ref tag_list) = tags {
                let tag_results = store
                    .search_by_tags(tag_list, limit * 2)
                    .map_err(|e| ToolError::ExecutionFailed(format!("tag search error: {e}")))?;

                // Remove entities that don't match tags
                let tag_ids: std::collections::HashSet<_> =
                    tag_results.iter().map(|e| e.id).collect();
                scored.retain(|id, _| tag_ids.contains(id));
            }
        }

        // Compute composite score and sort
        let mut results: Vec<ScoredEntity> = scored.into_values().collect();
        for r in &mut results {
            r.compute_composite();
        }
        results.sort_by(|a, b| b.composite().partial_cmp(&a.composite()).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        // Graph expansion
        if include_related && !results.is_empty() {
            for result in &mut results {
                for (store, _) in &stores_to_search {
                    if let Ok(relations) = store.get_relations(result.entity.id) {
                        for rel in relations {
                            let neighbor_id = if rel.source_id == result.entity.id {
                                rel.target_id
                            } else {
                                rel.source_id
                            };
                            if let Ok(Some(neighbor)) = store.get_entity(neighbor_id) {
                                result.related.push((rel.relation_type, neighbor.title.clone()));
                            }
                        }
                    }
                }
            }
        }

        // Format output
        if results.is_empty() {
            return Ok(ToolOutput::success("No matching knowledge found."));
        }

        let mut output = format!("Found {} results:\n\n", results.len());
        for (i, r) in results.iter().enumerate() {
            let snippet = r.entity.content.as_deref().map(|c| {
                let s: String = c.chars().take(150).collect();
                if c.len() > 150 { format!("{s}...") } else { s }
            });
            let tags_str = if r.entity.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", r.entity.tags.join(", "))
            };

            output.push_str(&format!(
                "{}. **[{}]** {} (score: {:.2}, confidence: {:.2}, reinforced: {}x){}\n",
                i + 1,
                r.entity.entity_type.as_str(),
                r.entity.title,
                r.composite(),
                r.confidence,
                r.entity.reinforcement_count,
                tags_str,
            ));

            if let Some(ref s) = snippet {
                output.push_str(&format!("   {s}\n"));
            }

            for (rel_type, title) in &r.related {
                output.push_str(&format!("   → {} {}\n", rel_type.as_str(), title));
            }
            output.push('\n');
        }

        Ok(ToolOutput::success(output))
    }
}

struct ScoredEntity {
    entity: Entity,
    fts_score: f32,
    semantic_score: f32,
    confidence: f32,
    source: String,
    related: Vec<(RelationType, String)>,
}

impl ScoredEntity {
    fn composite(&self) -> f32 {
        0.4 * self.semantic_score + 0.3 * self.fts_score + 0.3 * self.confidence
    }

    fn compute_composite(&mut self) {
        // pre-compute for sorting — just ensures fields are populated
    }
}

impl Default for ScoredEntity {
    fn default() -> Self {
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Workstream;
    use arawn_memory::{ConfidenceSource, Entity, EntityType, MemoryManager};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn setup() -> (TempDir, Arc<MemoryManager>, crate::context::EngineToolContext) {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("workstreams/test-ws")).unwrap();
        let mgr = Arc::new(MemoryManager::open(tmp.path(), "test-ws", None).unwrap());
        let ws = Workstream::scratch(tmp.path());
        let ctx = crate::context::EngineToolContext::new(&ws, Uuid::new_v4());
        (tmp, mgr, ctx)
    }

    fn populate(mgr: &MemoryManager) {
        mgr.workstream
            .insert_entity(&Entity::new(EntityType::Fact, "Rust ownership model"))
            .unwrap();
        mgr.workstream
            .insert_entity(
                &Entity::new(EntityType::Decision, "Use Rust for backend")
                    .with_content("Decided in sprint 5"),
            )
            .unwrap();
        mgr.global
            .insert_entity(
                &Entity::new(EntityType::Preference, "Prefers terse responses")
                    .with_confidence(ConfidenceSource::Stated),
            )
            .unwrap();
        mgr.workstream
            .insert_entity(
                &Entity::new(EntityType::Fact, "Python GIL limitations")
                    .with_tags(vec!["python".into()]),
            )
            .unwrap();
    }

    #[tokio::test]
    async fn search_fts_both_tiers() {
        let (_tmp, mgr, ctx) = setup();
        populate(&mgr);
        let tool = MemorySearchTool::new(mgr, None);

        let result = tool
            .execute(&ctx, json!({"query": "Rust"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(result.content.contains("Rust ownership"));
        assert!(result.content.contains("Use Rust"));
    }

    #[tokio::test]
    async fn search_with_type_filter() {
        let (_tmp, mgr, ctx) = setup();
        populate(&mgr);
        let tool = MemorySearchTool::new(mgr, None);

        let result = tool
            .execute(&ctx, json!({"query": "Rust", "entity_type": "decision"}))
            .await
            .unwrap();

        assert!(result.content.contains("Use Rust"));
        assert!(!result.content.contains("ownership"));
    }

    #[tokio::test]
    async fn search_global_only() {
        let (_tmp, mgr, ctx) = setup();
        populate(&mgr);
        let tool = MemorySearchTool::new(mgr, None);

        let result = tool
            .execute(&ctx, json!({"query": "terse", "scope": "global"}))
            .await
            .unwrap();

        assert!(result.content.contains("terse"));
    }

    #[tokio::test]
    async fn search_no_results() {
        let (_tmp, mgr, ctx) = setup();
        let tool = MemorySearchTool::new(mgr, None);

        let result = tool
            .execute(&ctx, json!({"query": "nonexistent"}))
            .await
            .unwrap();

        assert!(result.content.contains("No matching"));
    }

    #[tokio::test]
    async fn search_with_tags() {
        let (_tmp, mgr, ctx) = setup();
        populate(&mgr);
        let tool = MemorySearchTool::new(mgr, None);

        let result = tool
            .execute(&ctx, json!({"query": "Python", "tags": ["python"]}))
            .await
            .unwrap();

        assert!(result.content.contains("GIL"));
    }
}
