//! `signal_search` / `signal_query` / `signal_timeline` — agent-facing
//! read tools over a workstream KB. Phase 6 of I-0040.
//!
//! All three operate on the active workstream by default and route
//! through `MemoryHandle` so a `SessionWorkstream` switch is reflected
//! immediately. An explicit `workstream` arg routes to a named one when
//! the handle is `Routed`.
//!
//! Scoping: signal_* are workstream-tier only (Decision, Note, Fact in
//! workstream scope, Convention, etc.). The global tier (Preference,
//! Person) is reachable via the existing `memory_search` tool.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use tracing::debug;
use uuid::Uuid;

use arawn_embed::Embedder;
use arawn_memory::{Entity, EntityType, MemoryManager, MemoryStore};

use crate::tool::{Tool, ToolCategory, ToolError, ToolOutput};
use crate::workstream_router::{MemoryHandle, WorkstreamMemoryRouter};

/// RRF constant — same value `feed_search` uses.
const RRF_K: f32 = 60.0;

fn rrf(rank: usize) -> f32 {
    1.0 / (RRF_K + rank as f32 + 1.0)
}

/// Resolve the manager for the active workstream, or the explicit
/// `workstream` arg when provided. `Fixed` handles always return the
/// same manager regardless of the override (for test ergonomics).
fn resolve_manager(
    handle: &MemoryHandle,
    explicit: Option<&str>,
    router: Option<&Arc<WorkstreamMemoryRouter>>,
) -> Result<Arc<MemoryManager>, ToolError> {
    if let Some(name) = explicit
        && let Some(r) = router
    {
        return r
            .for_workstream(name)
            .map_err(|e| ToolError::ExecutionFailed(format!("workstream `{name}`: {e}")));
    }
    handle
        .manager()
        .map_err(|e| ToolError::ExecutionFailed(format!("memory routing: {e}")))
}

fn entity_summary(e: &Entity) -> Value {
    json!({
        "id": e.id,
        "entity_type": e.entity_type.as_str(),
        "title": e.title,
        "content_snippet": e.content.as_deref().map(|c| snippet(c, 240)),
        "tags_ontology": e.tags_ontology,
        "tags_discovered": e.tags,
        "confidence": e.confidence_score(),
        "reinforcement_count": e.reinforcement_count,
        "created_at": e.created_at.to_rfc3339(),
        "updated_at": e.updated_at.to_rfc3339(),
    })
}

fn snippet(s: &str, cap: usize) -> String {
    if s.chars().count() <= cap {
        return s.to_string();
    }
    let head: String = s.chars().take(cap).collect();
    format!("{head}…")
}

// ─────────────────────────────────────────────────────────────────────────
// signal_search — hybrid FTS5 + vector over the workstream KB
// ─────────────────────────────────────────────────────────────────────────

pub struct SignalSearchTool {
    memory: MemoryHandle,
    router: Option<Arc<WorkstreamMemoryRouter>>,
    embedder: Option<Arc<dyn Embedder>>,
}

impl SignalSearchTool {
    pub fn new(
        memory: impl Into<MemoryHandle>,
        embedder: Option<Arc<dyn Embedder>>,
    ) -> Self {
        let memory = memory.into();
        let router = match &memory {
            MemoryHandle::Routed(r) => Some(Arc::clone(r)),
            MemoryHandle::Fixed(_) => None,
        };
        Self {
            memory,
            router,
            embedder,
        }
    }
}

#[async_trait]
impl Tool for SignalSearchTool {
    fn name(&self) -> &str {
        "signal_search"
    }

    fn description(&self) -> &str {
        "Semantic + FTS5 search over the active workstream's curated knowledge \
         base. Returns entities (decisions, facts, notes, conventions) extracted \
         from feeds and ranked by hybrid similarity. Pair with `feed_search` when \
         you need the raw projection rows behind a finding."
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
                "query": { "type": "string", "description": "Free-text search query" },
                "workstream": {
                    "type": "string",
                    "description": "Override the active workstream; defaults to current"
                },
                "limit": { "type": "integer", "description": "Max results (default 10, max 50)" }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'query'".into()))?;
        let explicit = params.get("workstream").and_then(|v| v.as_str());
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(50) as usize;

        let mgr = resolve_manager(&self.memory, explicit, self.router.as_ref())?;
        let store: &Arc<MemoryStore> = &mgr.workstream;

        // FTS5 ranks
        let fts_hits = store
            .search(query, limit * 4)
            .map_err(|e| ToolError::ExecutionFailed(format!("fts: {e}")))?;
        let mut fused: HashMap<Uuid, FusedHit> = HashMap::new();
        for (rank, ent) in fts_hits.into_iter().enumerate() {
            fused
                .entry(ent.id)
                .or_insert_with(|| FusedHit::new(ent.clone()))
                .score += rrf(rank);
        }

        // Vector ranks (when an embedder is configured)
        if let Some(emb) = self.embedder.as_ref() {
            match emb.embed(query).await {
                Ok(qv) => {
                    let hits = store
                        .search_similar(&qv, limit * 4)
                        .map_err(|e| ToolError::ExecutionFailed(format!("vec: {e}")))?;
                    for (rank, sim) in hits.into_iter().enumerate() {
                        if let Ok(Some(ent)) = store.get_entity(sim.entity_id) {
                            if ent.superseded {
                                continue;
                            }
                            fused
                                .entry(ent.id)
                                .or_insert_with(|| FusedHit::new(ent))
                                .score += rrf(rank);
                        }
                    }
                }
                Err(e) => debug!(error = %e, "signal_search: embed failed; FTS-only"),
            }
        }

        let mut hits: Vec<FusedHit> = fused.into_values().collect();
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(limit);

        let results: Vec<Value> = hits
            .iter()
            .map(|h| {
                let mut row = entity_summary(&h.entity);
                if let Value::Object(ref mut m) = row {
                    m.insert("score".into(), json!(h.score));
                }
                row
            })
            .collect();
        Ok(ToolOutput::success(
            json!({
                "results": results,
                "count": results.len(),
            })
            .to_string(),
        ))
    }
}

struct FusedHit {
    entity: Entity,
    score: f32,
}

impl FusedHit {
    fn new(entity: Entity) -> Self {
        Self {
            entity,
            score: 0.0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// signal_query — structured filter (entity_type, tags, since/until)
// ─────────────────────────────────────────────────────────────────────────

pub struct SignalQueryTool {
    memory: MemoryHandle,
    router: Option<Arc<WorkstreamMemoryRouter>>,
}

impl SignalQueryTool {
    pub fn new(memory: impl Into<MemoryHandle>) -> Self {
        let memory = memory.into();
        let router = match &memory {
            MemoryHandle::Routed(r) => Some(Arc::clone(r)),
            MemoryHandle::Fixed(_) => None,
        };
        Self { memory, router }
    }
}

#[async_trait]
impl Tool for SignalQueryTool {
    fn name(&self) -> &str {
        "signal_query"
    }

    fn description(&self) -> &str {
        "Structured filter over the active workstream's KB. Use when you know \
         what *shape* of entity you want (e.g. all decisions tagged \
         stripe:migration since last month) rather than a free-text query. \
         Filters compose: every filter narrows the result set."
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
                "entity_type": {
                    "type": "string",
                    "enum": ["fact", "decision", "convention", "preference", "person", "note"]
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Match any of these tags (OR semantics). Filters against `tags_ontology` by default — set `include_discovered: true` to also match against `tags_discovered`."
                },
                "include_discovered": {
                    "type": "boolean",
                    "description": "When true, the `tags` filter also matches against the LLM-free `tags_discovered` field. Default false (ontology-only)."
                },
                "since": { "type": "string", "description": "RFC3339; updated_at >= since" },
                "until": { "type": "string", "description": "RFC3339; updated_at <= until" },
                "workstream": { "type": "string" },
                "limit": { "type": "integer", "description": "Max results (default 25, max 200)" }
            }
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let entity_type = params
            .get("entity_type")
            .and_then(|v| v.as_str())
            .and_then(EntityType::from_str);
        let tags: Vec<String> = params
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let include_discovered = params
            .get("include_discovered")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let since = params
            .get("since")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let until = params
            .get("until")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let explicit = params.get("workstream").and_then(|v| v.as_str());
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(25)
            .min(200) as usize;

        let mgr = resolve_manager(&self.memory, explicit, self.router.as_ref())?;
        let store = &mgr.workstream;

        // Candidate set: list_by_type when entity_type is specified,
        // otherwise list_all_ranked. We over-fetch since downstream
        // filters (tags, since/until) can reduce the set arbitrarily.
        let fetch = (limit * 4).max(50);
        let mut candidates: Vec<Entity> = match entity_type {
            Some(et) => store
                .list_by_type(et, fetch)
                .map_err(|e| ToolError::ExecutionFailed(format!("list_by_type: {e}")))?,
            None => store
                .list_all_ranked(fetch)
                .map_err(|e| ToolError::ExecutionFailed(format!("list_all: {e}")))?,
        };

        if !tags.is_empty() {
            // ADR-0004: default filter is ontology-only (deterministic).
            // `include_discovered` widens to LLM-free tags for recall.
            candidates.retain(|e| {
                let onto_hit = e.tags_ontology.iter().any(|t| tags.contains(t));
                let disc_hit = include_discovered && e.tags.iter().any(|t| tags.contains(t));
                onto_hit || disc_hit
            });
        }
        if let Some(s) = since {
            candidates.retain(|e| e.updated_at >= s);
        }
        if let Some(u) = until {
            candidates.retain(|e| e.updated_at <= u);
        }
        candidates.truncate(limit);

        let results: Vec<Value> = candidates.iter().map(entity_summary).collect();
        Ok(ToolOutput::success(
            json!({
                "results": results,
                "count": results.len(),
            })
            .to_string(),
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────────
// signal_timeline — chronological slice across a workstream
// ─────────────────────────────────────────────────────────────────────────

pub struct SignalTimelineTool {
    memory: MemoryHandle,
    router: Option<Arc<WorkstreamMemoryRouter>>,
}

impl SignalTimelineTool {
    pub fn new(memory: impl Into<MemoryHandle>) -> Self {
        let memory = memory.into();
        let router = match &memory {
            MemoryHandle::Routed(r) => Some(Arc::clone(r)),
            MemoryHandle::Fixed(_) => None,
        };
        Self { memory, router }
    }
}

#[async_trait]
impl Tool for SignalTimelineTool {
    fn name(&self) -> &str {
        "signal_timeline"
    }

    fn description(&self) -> &str {
        "Chronological slice across a workstream's KB. Returns entities in \
         created_at-descending order within an optional [since, until] window. \
         Useful for `what happened in this workstream last week` summaries."
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
                "since": { "type": "string", "description": "RFC3339" },
                "until": { "type": "string", "description": "RFC3339" },
                "workstream": { "type": "string" },
                "limit": { "type": "integer", "description": "Max events (default 50, max 200)" }
            }
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let since = params
            .get("since")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let until = params
            .get("until")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let explicit = params.get("workstream").and_then(|v| v.as_str());
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50)
            .min(200) as usize;

        let mgr = resolve_manager(&self.memory, explicit, self.router.as_ref())?;
        let store = &mgr.workstream;

        // No native "list all ordered by created_at" — list_all_ranked
        // returns the active set, we sort by created_at here. Window
        // filtering happens before truncate.
        let mut all = store
            .list_all_ranked((limit * 4).max(100))
            .map_err(|e| ToolError::ExecutionFailed(format!("list_all: {e}")))?;
        if let Some(s) = since {
            all.retain(|e| e.created_at >= s);
        }
        if let Some(u) = until {
            all.retain(|e| e.created_at <= u);
        }
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        all.truncate(limit);

        let events: Vec<Value> = all
            .iter()
            .map(|e| {
                json!({
                    "ts": e.created_at.to_rfc3339(),
                    "kind": "entity_created",
                    "entity": entity_summary(e),
                })
            })
            .collect();
        Ok(ToolOutput::success(
            json!({
                "events": events,
                "count": events.len(),
            })
            .to_string(),
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Workstream;
    use arawn_memory::{ConfidenceSource, Entity, EntityType, MemoryManager};
    use tempfile::TempDir;

    fn setup() -> (TempDir, Arc<MemoryManager>, crate::context::EngineToolContext) {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("workstreams/test-ws")).unwrap();
        let mgr = Arc::new(MemoryManager::open(tmp.path(), "test-ws", None).unwrap());
        let ws = Workstream::scratch(tmp.path());
        let ctx = crate::context::EngineToolContext::new(&ws, Uuid::new_v4());
        (tmp, mgr, ctx)
    }

    fn seed(mgr: &MemoryManager) {
        // signal_query defaults to filtering on tags_ontology (ADR-0004).
        // Tests seed ontology tags directly.
        mgr.workstream
            .insert_entity(
                &Entity::new(EntityType::Decision, "use postgres for storage")
                    .with_content("chose postgres over mysql for jsonb support")
                    .with_tags_ontology(vec!["db".into(), "infra".into()])
                    .with_confidence(ConfidenceSource::Stated),
            )
            .unwrap();
        mgr.workstream
            .insert_entity(
                &Entity::new(EntityType::Convention, "PRs require two reviewers")
                    .with_tags_ontology(vec!["process".into()]),
            )
            .unwrap();
        mgr.workstream
            .insert_entity(
                &Entity::new(EntityType::Note, "alice is on parental leave through june")
                    .with_tags_ontology(vec!["team".into()]),
            )
            .unwrap();
    }

    #[tokio::test]
    async fn signal_search_finds_decision_by_title() {
        let (_tmp, mgr, ctx) = setup();
        seed(&mgr);
        let tool = SignalSearchTool::new(mgr, None);
        let r = tool
            .execute(&ctx, json!({"query": "postgres"}))
            .await
            .unwrap();
        assert!(!r.is_error);
        let v: Value = serde_json::from_str(&r.content).unwrap();
        let results = v["results"].as_array().unwrap();
        assert!(
            results.iter().any(|e| e["title"]
                .as_str()
                .unwrap()
                .contains("postgres")),
            "expected postgres entity in results: {v}"
        );
    }

    #[tokio::test]
    async fn signal_search_empty_kb_returns_zero() {
        let (_tmp, mgr, ctx) = setup();
        let tool = SignalSearchTool::new(mgr, None);
        let r = tool
            .execute(&ctx, json!({"query": "anything"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(v["count"], 0);
    }

    #[tokio::test]
    async fn signal_query_filters_by_entity_type() {
        let (_tmp, mgr, ctx) = setup();
        seed(&mgr);
        let tool = SignalQueryTool::new(mgr);
        let r = tool
            .execute(&ctx, json!({"entity_type": "decision"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        let results = v["results"].as_array().unwrap();
        assert!(!results.is_empty());
        assert!(
            results
                .iter()
                .all(|e| e["entity_type"].as_str() == Some("decision")),
            "non-decision leaked into results: {v}"
        );
    }

    #[tokio::test]
    async fn signal_query_filters_by_tag_any_of() {
        let (_tmp, mgr, ctx) = setup();
        seed(&mgr);
        let tool = SignalQueryTool::new(mgr);
        let r = tool
            .execute(&ctx, json!({"tags": ["team"]}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        let results = v["results"].as_array().unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0]["title"]
            .as_str()
            .unwrap()
            .contains("alice"));
    }

    #[tokio::test]
    async fn signal_query_no_filters_returns_all_active() {
        let (_tmp, mgr, ctx) = setup();
        seed(&mgr);
        let tool = SignalQueryTool::new(mgr);
        let r = tool.execute(&ctx, json!({})).await.unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(v["count"], 3);
    }

    #[tokio::test]
    async fn signal_query_window_filters() {
        let (_tmp, mgr, ctx) = setup();
        seed(&mgr);
        let tool = SignalQueryTool::new(mgr);
        // Future window — nothing should match
        let r = tool
            .execute(
                &ctx,
                json!({"since": "2099-01-01T00:00:00Z"}),
            )
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(v["count"], 0);
    }

    #[tokio::test]
    async fn signal_timeline_orders_desc_and_caps_to_window() {
        let (_tmp, mgr, ctx) = setup();
        seed(&mgr);
        let tool = SignalTimelineTool::new(mgr);
        let r = tool.execute(&ctx, json!({})).await.unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        let events = v["events"].as_array().unwrap();
        assert_eq!(events.len(), 3);
        // created_at descending: ts[0] >= ts[1] >= ts[2]
        let ts: Vec<&str> = events
            .iter()
            .map(|e| e["ts"].as_str().unwrap())
            .collect();
        assert!(ts[0] >= ts[1]);
        assert!(ts[1] >= ts[2]);
    }

    #[tokio::test]
    async fn explicit_workstream_arg_routes_via_router() {
        let tmp = TempDir::new().unwrap();
        let session = crate::tools::SessionWorkstream::scratch();
        let router = Arc::new(WorkstreamMemoryRouter::new(
            tmp.path(),
            None,
            None,
            session.clone(),
        ));
        // Seed "other" workstream
        {
            let other = router.for_workstream("other").unwrap();
            other
                .workstream
                .insert_entity(&Entity::new(EntityType::Fact, "secret from other ws"))
                .unwrap();
        }
        let ws = Workstream::scratch(tmp.path());
        let ctx = crate::context::EngineToolContext::new(&ws, Uuid::new_v4());
        let tool = SignalSearchTool::new(router, None);

        // No override: scratch (active) has nothing.
        let r = tool
            .execute(&ctx, json!({"query": "secret"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(v["count"], 0, "scratch ws should be empty");

        // Explicit override routes to "other".
        let r = tool
            .execute(&ctx, json!({"query": "secret", "workstream": "other"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(v["count"], 1, "override should route to `other`");
    }
}
