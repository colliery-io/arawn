//! `signal_search` / `signal_query` / `signal_timeline` — agent-facing read
//! tools over the lens KBs.
//!
//! ARAWN-I-0061: signals are *extracted* from feeds by each lens's standing
//! extractor (`arawn-extractor::cot`) and live in that lens's KB. These tools
//! read across **every** lens by default and label each hit with its source
//! lens; an explicit `lens` arg narrows to one when present.
//!
//! `signal_*` are lens-tier reads. Global **memory** (the user's standing facts
//! and behavioral tuning) is a separate concern — search it with `memory_search`.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use tracing::debug;

use arawn_embed::Embedder;
use arawn_memory::{Entity, EntityType, MemoryManager, MemoryStore};

use crate::lens_router::{LensMemoryRouter, MemoryHandle};
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

/// Resolve the manager for the active lens, or the explicit
/// `lens` arg when provided. `Fixed` handles always return the
/// same manager regardless of the override (for test ergonomics).
fn resolve_manager(
    handle: &MemoryHandle,
    explicit: Option<&str>,
    router: Option<&Arc<LensMemoryRouter>>,
) -> Result<Arc<MemoryManager>, ToolError> {
    if let Some(name) = explicit
        && let Some(r) = router
    {
        return r
            .for_lens(name)
            .map_err(|e| ToolError::ExecutionFailed(format!("lens `{name}`: {e}")));
    }
    handle
        .manager()
        .map_err(|e| ToolError::ExecutionFailed(format!("memory routing: {e}")))
}

/// The set of `(lens, store)` a read tool should query (ARAWN-I-0060): an
/// explicit `lens` narrows to one; otherwise roam every lens. A Fixed handle
/// (tests / non-routed) yields its single manager's lens tier, labeled `"lens"`.
fn lens_stores(
    handle: &MemoryHandle,
    router: Option<&Arc<LensMemoryRouter>>,
    explicit: Option<&str>,
) -> Result<Vec<(String, Arc<MemoryStore>)>, ToolError> {
    match (explicit, router) {
        (Some(name), Some(r)) => {
            let mgr = r
                .for_lens(name)
                .map_err(|e| ToolError::ExecutionFailed(format!("lens `{name}`: {e}")))?;
            Ok(vec![(name.to_string(), Arc::clone(&mgr.lens))])
        }
        (None, Some(r)) => Ok(r
            .all_lens_managers()
            .into_iter()
            .map(|(n, m)| (n, Arc::clone(&m.lens)))
            .collect()),
        _ => {
            let mgr = resolve_manager(handle, explicit, router)?;
            Ok(vec![("lens".to_string(), Arc::clone(&mgr.lens))])
        }
    }
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
// signal_search — hybrid FTS5 + vector over the lens KB
// ─────────────────────────────────────────────────────────────────────────

pub struct SignalSearchTool {
    memory: MemoryHandle,
    router: Option<Arc<LensMemoryRouter>>,
    embedder: Option<Arc<dyn Embedder>>,
}

impl SignalSearchTool {
    pub fn new(memory: impl Into<MemoryHandle>, embedder: Option<Arc<dyn Embedder>>) -> Self {
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
        "Semantic + FTS5 search over your curated knowledge base **across all \
         lenses**. Returns **entities** (decisions, facts, notes, conventions, \
         events, mentions) extracted from feeds and ranked by hybrid similarity; \
         each hit is labeled with the `lens` it came from. Pass `lens` to restrict \
         to one.\n\n\
         Reach for this first when synthesizing across sources — \"what's on my \
         plate today\", \"morning briefing across calendar/inbox/Slack\", \"what \
         did we decide / agree / observe about X\" — because it returns the \
         *extracted* signal stream without the top-N FTS cutoff that `feed_search` \
         applies. For raw, single-source content — \"read this exact gmail thread\", \
         \"what's the last slack message in #X\" — `feed_search` is the right call. \
         The daily ceremony tablet (`daily_list_items`) is a curated brief, not a \
         substitute for either."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        // signal_* operate on per-lens KBs, so they belong to
        // the Lens category — the query engine's filter scopes
        // tool exposure by category based on keywords in the user
        // message. Putting them in Memory caused the dust/refine/
        // signal_search chain to vanish from the tool list whenever
        // the user prompt didn't include "remember"/"recall"/etc.
        ToolCategory::Lens
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Free-text search query" },
                "lens": {
                    "type": "string",
                    "description": "Restrict to one lens by name; omit to search across all lenses"
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
        let explicit = params.get("lens").and_then(|v| v.as_str());
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(50) as usize;

        // Read across ALL lenses by default; an explicit `lens` narrows to one.
        // signal_* is lens-tier only (global preferences/people are
        // `memory_search`'s job). Each hit is labeled with its source lens.
        let stores = lens_stores(&self.memory, self.router.as_ref(), explicit)?;

        // Embed once (caller-side); the fusion primitive is synchronous.
        let query_embedding = match self.embedder.as_ref() {
            Some(emb) => match emb.embed(query).await {
                Ok(qv) => Some(qv),
                Err(e) => {
                    debug!(error = %e, "signal_search: embed failed; FTS-only");
                    None
                }
            },
            None => None,
        };

        let hits =
            arawn_memory::search_labeled_stores(&stores, query, query_embedding.as_deref(), limit);

        let results: Vec<Value> = hits
            .iter()
            .map(|h| {
                let mut row = entity_summary(&h.entity);
                if let Value::Object(ref mut m) = row {
                    m.insert("lens".into(), json!(h.lens));
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

// ─────────────────────────────────────────────────────────────────────────
// signal_query — structured filter (entity_type, tags, since/until)
// ─────────────────────────────────────────────────────────────────────────

pub struct SignalQueryTool {
    memory: MemoryHandle,
    router: Option<Arc<LensMemoryRouter>>,
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
        "Structured filter over the signal stream extracted by every lens. \
         Searches all lens KBs by default and labels each hit with the `lens` it \
         came from; pass `lens` to narrow to one. Use when you know what *shape* \
         of entity you want (e.g. all decisions tagged stripe:migration since \
         last month) rather than a free-text query. Filters compose: every \
         filter narrows the result set."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        // signal_* operate on per-lens KBs, so they belong to
        // the Lens category — the query engine's filter scopes
        // tool exposure by category based on keywords in the user
        // message. Putting them in Memory caused the dust/refine/
        // signal_search chain to vanish from the tool list whenever
        // the user prompt didn't include "remember"/"recall"/etc.
        ToolCategory::Lens
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
                "lens": { "type": "string" },
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
        let explicit = params.get("lens").and_then(|v| v.as_str());
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(25)
            .min(200) as usize;

        // Roam every lens by default; `lens` narrows to one. Filter each store's
        // candidates, then merge, order by recency, and cap.
        let stores = lens_stores(&self.memory, self.router.as_ref(), explicit)?;
        let fetch = (limit * 4).max(50);
        let mut candidates: Vec<(String, Entity)> = Vec::new();
        for (lens, store) in &stores {
            let mut ents: Vec<Entity> = match entity_type {
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
                ents.retain(|e| {
                    let onto_hit = e.tags_ontology.iter().any(|t| tags.contains(t));
                    let disc_hit = include_discovered && e.tags.iter().any(|t| tags.contains(t));
                    onto_hit || disc_hit
                });
            }
            if let Some(s) = since {
                ents.retain(|e| e.updated_at >= s);
            }
            if let Some(u) = until {
                ents.retain(|e| e.updated_at <= u);
            }
            candidates.extend(ents.into_iter().map(|e| (lens.clone(), e)));
        }
        // Uniform cross-lens ordering: most-recently-updated first.
        candidates.sort_by(|a, b| b.1.updated_at.cmp(&a.1.updated_at));
        candidates.truncate(limit);

        let results: Vec<Value> = candidates
            .iter()
            .map(|(lens, e)| {
                let mut row = entity_summary(e);
                if let Value::Object(ref mut m) = row {
                    m.insert("lens".into(), json!(lens));
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

// ─────────────────────────────────────────────────────────────────────────
// signal_timeline — chronological slice across a lens
// ─────────────────────────────────────────────────────────────────────────

pub struct SignalTimelineTool {
    memory: MemoryHandle,
    router: Option<Arc<LensMemoryRouter>>,
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
        "Chronological slice over the signal stream extracted by every lens. \
         Returns entities in created_at-descending order within an optional \
         [since, until] window; each hit is labeled with the source lens. Pass \
         `lens` to restrict to one. Useful for \"what happened across my lenses \
         last week\" summaries."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        // signal_* operate on per-lens KBs, so they belong to
        // the Lens category — the query engine's filter scopes
        // tool exposure by category based on keywords in the user
        // message. Putting them in Memory caused the dust/refine/
        // signal_search chain to vanish from the tool list whenever
        // the user prompt didn't include "remember"/"recall"/etc.
        ToolCategory::Lens
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "since": { "type": "string", "description": "RFC3339" },
                "until": { "type": "string", "description": "RFC3339" },
                "lens": { "type": "string" },
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
        let explicit = params.get("lens").and_then(|v| v.as_str());
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50)
            .min(200) as usize;

        // Roam every lens by default; `lens` narrows to one. Merge each store's
        // entities into one chronological timeline.
        let stores = lens_stores(&self.memory, self.router.as_ref(), explicit)?;
        let fetch = (limit * 4).max(100);
        let mut all: Vec<(String, Entity)> = Vec::new();
        for (lens, store) in &stores {
            let mut ents = store
                .list_all_ranked(fetch)
                .map_err(|e| ToolError::ExecutionFailed(format!("list_all: {e}")))?;
            if let Some(s) = since {
                ents.retain(|e| e.created_at >= s);
            }
            if let Some(u) = until {
                ents.retain(|e| e.created_at <= u);
            }
            all.extend(ents.into_iter().map(|e| (lens.clone(), e)));
        }
        all.sort_by(|a, b| b.1.created_at.cmp(&a.1.created_at));
        all.truncate(limit);

        let events: Vec<Value> = all
            .iter()
            .map(|(lens, e)| {
                json!({
                    "ts": e.created_at.to_rfc3339(),
                    "kind": "entity_created",
                    "lens": lens,
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
    use arawn_core::Lens;
    use arawn_memory::{ConfidenceSource, Entity, EntityType, MemoryManager};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn setup() -> (
        TempDir,
        Arc<MemoryManager>,
        crate::context::EngineToolContext,
    ) {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("lenses/test-ws")).unwrap();
        let mgr = Arc::new(MemoryManager::open(tmp.path(), "test-ws", None).unwrap());
        let ws = Lens::scratch(tmp.path());
        let ctx = crate::context::EngineToolContext::new(&ws, Uuid::new_v4());
        (tmp, mgr, ctx)
    }

    fn seed(mgr: &MemoryManager) {
        // signal_query defaults to filtering on tags_ontology (ADR-0004).
        // Tests seed ontology tags directly.
        mgr.lens
            .insert_entity(
                &Entity::new(EntityType::Decision, "use postgres for storage")
                    .with_content("chose postgres over mysql for jsonb support")
                    .with_tags_ontology(vec!["db".into(), "infra".into()])
                    .with_confidence(ConfidenceSource::Stated),
            )
            .unwrap();
        mgr.lens
            .insert_entity(
                &Entity::new(EntityType::Convention, "PRs require two reviewers")
                    .with_tags_ontology(vec!["process".into()]),
            )
            .unwrap();
        mgr.lens
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
            results
                .iter()
                .any(|e| e["title"].as_str().unwrap().contains("postgres")),
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
        let r = tool.execute(&ctx, json!({"tags": ["team"]})).await.unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        let results = v["results"].as_array().unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0]["title"].as_str().unwrap().contains("alice"));
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
            .execute(&ctx, json!({"since": "2099-01-01T00:00:00Z"}))
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
        let ts: Vec<&str> = events.iter().map(|e| e["ts"].as_str().unwrap()).collect();
        assert!(ts[0] >= ts[1]);
        assert!(ts[1] >= ts[2]);
    }

    #[tokio::test]
    async fn explicit_lens_arg_routes_via_router() {
        let tmp = TempDir::new().unwrap();
        let session = crate::tools::SessionLens::scratch();
        let router = Arc::new(LensMemoryRouter::new(
            tmp.path(),
            None,
            None,
            session.clone(),
        ));
        // Seed "other" lens
        {
            let other = router.for_lens("other").unwrap();
            other
                .lens
                .insert_entity(&Entity::new(EntityType::Fact, "secret from other ws"))
                .unwrap();
        }
        let ws = Lens::scratch(tmp.path());
        let ctx = crate::context::EngineToolContext::new(&ws, Uuid::new_v4());
        let tool = SignalSearchTool::new(router, None);

        // No override now ROAMS across all lenses (I-0060) — so it finds the
        // entity living in `other`, labeled with its source lens.
        let r = tool
            .execute(&ctx, json!({"query": "secret"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(
            v["count"], 1,
            "roam-all should find the `other` lens entity"
        );
        assert_eq!(
            v["results"][0]["lens"], "other",
            "hit should be labeled with its source lens"
        );

        // Explicit `lens` narrows to one store.
        let r = tool
            .execute(&ctx, json!({"query": "secret", "lens": "other"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(v["count"], 1, "narrow to `other` finds it");

        // Narrowing to the empty scratch lens finds nothing.
        let r = tool
            .execute(&ctx, json!({"query": "secret", "lens": "scratch"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(v["count"], 0, "narrow to empty scratch is empty");
    }
}
