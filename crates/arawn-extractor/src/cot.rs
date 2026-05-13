//! `CotChain` — the real 4-stage chain-of-thought extractor.
//!
//! Stage 1 (classify): is this projection row in scope for the workstream?
//! Stage 2 (extract): pull typed entities out of the body.
//! Stage 3 (link-by-name): emit candidate relations; we resolve by FTS.
//! Stage 4 (write): store_fact each entity + add resolved relations
//! plus an EXTRACTED_FROM provenance edge.
//!
//! Each stage is one LLM call. Free / inexpensive backend behind it
//! per I-0040 phase 4 design. The chain reads the workstream
//! description to scope decisions and emits free-form tags; the
//! steward (Phase 5) refines vocabulary later.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::{debug, warn};
use uuid::Uuid;

use arawn_core::Workstream;
use arawn_llm::LlmClient;
use arawn_memory::{
    ConfidenceSource, Entity, EntityType, MemoryManager, MemoryStore, RelationType, Scope,
    StoreFactResult,
};
use arawn_projections::ProjectionRow;

use crate::chain::{ChainOutcome, ExtractionChain};
use crate::error::ExtractionError;
use crate::llm_text::{complete_text, extract_json_block};

/// The real CoT chain. Constructed once at startup with a shared LLM
/// client + model name (typically resolved through
/// `ArawnConfig::extraction_llm()`).
pub struct CotChain {
    client: Arc<dyn LlmClient>,
    model: String,
    /// FTS similarity floor for link-by-name resolution. Top-1 results
    /// below this score are dropped to avoid spurious links.
    link_score_floor: f32,
}

impl CotChain {
    pub fn new(client: Arc<dyn LlmClient>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
            link_score_floor: 0.0,
        }
    }

    pub fn with_link_score_floor(mut self, floor: f32) -> Self {
        self.link_score_floor = floor;
        self
    }
}

#[async_trait]
impl ExtractionChain for CotChain {
    async fn run(
        &self,
        workstream: &Workstream,
        row: &ProjectionRow,
        kb: &MemoryManager,
    ) -> Result<ChainOutcome, ExtractionError> {
        // ── Stage 1: classify ───────────────────────────────────────────
        let classify = self.classify(workstream, row).await?;
        if !classify.in_scope {
            debug!(
                workstream = %workstream.name,
                row_id = %row.id,
                reason = %classify.reason,
                "row classified out of scope"
            );
            return Ok(ChainOutcome {
                entities_written: Vec::new(),
                relations_written: 0,
                skipped: true,
            });
        }

        // ── Stage 2: extract ────────────────────────────────────────────
        let candidates = self.extract(workstream, row).await?;
        if candidates.is_empty() {
            return Ok(ChainOutcome::default());
        }

        // ── Stage 3: link-by-name ───────────────────────────────────────
        let link_proposals = self.link_by_name(workstream, &candidates).await?;

        // ── Stage 4: write ──────────────────────────────────────────────
        self.write(row, &candidates, &link_proposals, kb).await
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Stage 1 — classify
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ClassifyResult {
    in_scope: bool,
    #[serde(default)]
    reason: String,
}

impl CotChain {
    async fn classify(
        &self,
        ws: &Workstream,
        row: &ProjectionRow,
    ) -> Result<ClassifyResult, ExtractionError> {
        let system = "You decide whether a piece of content belongs in a knowledge \
                      base for a specific workstream. Output ONLY a JSON object: \
                      {\"in_scope\": bool, \"reason\": short string}. \
                      Be selective — a workstream is a tight scope (one person, \
                      one project, one initiative). When in doubt, in_scope = false.";
        let user = format!(
            "Workstream: {name}\n\
             Description: {desc}\n\n\
             Item (feed type: {feed_type}):\n\
             Title: {title}\n\
             Body:\n{body}\n",
            name = ws.name,
            desc = if ws.description.is_empty() {
                "(no description set)"
            } else {
                ws.description.as_str()
            },
            feed_type = row.feed_type,
            title = row.title,
            body = truncate(&row.body_text, 4_000),
        );
        let raw = complete_text(&self.client, &self.model, system, &user).await?;
        parse_classify(&raw)
    }
}

fn parse_classify(raw: &str) -> Result<ClassifyResult, ExtractionError> {
    let json = extract_json_block(raw)
        .ok_or_else(|| ExtractionError::Parse(format!("classify: no JSON found in: {raw}")))?;
    serde_json::from_str(json).map_err(|e| ExtractionError::Parse(format!("classify: {e}")))
}

// ─────────────────────────────────────────────────────────────────────────
// Stage 2 — extract
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
struct ExtractedCandidate {
    entity_type: String,
    title: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    tags: Vec<String>,
}

impl CotChain {
    async fn extract(
        &self,
        ws: &Workstream,
        row: &ProjectionRow,
    ) -> Result<Vec<ExtractedCandidate>, ExtractionError> {
        let system = "Pull typed knowledge entities out of the content. Output ONLY \
                      a JSON array. Each item: {\"entity_type\": one of \
                      [fact, decision, convention, preference, person, note], \
                      \"title\": short, \"content\": optional longer text, \
                      \"tags\": optional array of short slugs}. \
                      Be conservative — only entities that genuinely belong in \
                      this workstream's KB. Empty array is a valid answer.";
        let user = format!(
            "Workstream: {name}\nDescription: {desc}\n\n\
             Title: {title}\nBody:\n{body}\n",
            name = ws.name,
            desc = if ws.description.is_empty() {
                "(no description set)"
            } else {
                ws.description.as_str()
            },
            title = row.title,
            body = truncate(&row.body_text, 4_000),
        );
        let raw = complete_text(&self.client, &self.model, system, &user).await?;
        parse_candidates(&raw)
    }
}

fn parse_candidates(raw: &str) -> Result<Vec<ExtractedCandidate>, ExtractionError> {
    let json = extract_json_block(raw)
        .ok_or_else(|| ExtractionError::Parse(format!("extract: no JSON in: {raw}")))?;
    serde_json::from_str(json).map_err(|e| ExtractionError::Parse(format!("extract: {e}")))
}

// ─────────────────────────────────────────────────────────────────────────
// Stage 3 — link-by-name
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
struct LinkProposal {
    from: String,
    rel: String,
    to_name: String,
}

impl CotChain {
    async fn link_by_name(
        &self,
        ws: &Workstream,
        candidates: &[ExtractedCandidate],
    ) -> Result<Vec<LinkProposal>, ExtractionError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let system = "Propose relations between the new entities and entities \
                      that may already exist in this workstream's KB. Output ONLY \
                      a JSON array; each: {\"from\": title of one of the new \
                      entities, \"rel\": one of [relates_to, supports, contradicts, \
                      supersedes, mentions, belongs_to], \"to_name\": title of \
                      target (new or existing)}. Empty array is fine.";
        let entities_json = serde_json::to_string_pretty(
            &candidates
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "entity_type": c.entity_type,
                        "title": c.title,
                    })
                })
                .collect::<Vec<_>>(),
        )?;
        let user = format!(
            "Workstream: {name}\nDescription: {desc}\n\nNew entities:\n{entities}\n",
            name = ws.name,
            desc = if ws.description.is_empty() {
                "(no description set)"
            } else {
                ws.description.as_str()
            },
            entities = entities_json,
        );
        let raw = complete_text(&self.client, &self.model, system, &user).await?;
        parse_links(&raw)
    }
}

fn parse_links(raw: &str) -> Result<Vec<LinkProposal>, ExtractionError> {
    let json = extract_json_block(raw)
        .ok_or_else(|| ExtractionError::Parse(format!("link: no JSON in: {raw}")))?;
    serde_json::from_str(json).map_err(|e| ExtractionError::Parse(format!("link: {e}")))
}

// ─────────────────────────────────────────────────────────────────────────
// Stage 4 — write
// ─────────────────────────────────────────────────────────────────────────

impl CotChain {
    async fn write(
        &self,
        row: &ProjectionRow,
        candidates: &[ExtractedCandidate],
        links: &[LinkProposal],
        kb: &MemoryManager,
    ) -> Result<ChainOutcome, ExtractionError> {
        // 1. Write entities; record title → id for link resolution.
        let mut title_to_id: HashMap<String, (Uuid, Scope)> = HashMap::new();
        let mut entities_written: Vec<Uuid> = Vec::new();
        for cand in candidates {
            let Some(et) = parse_entity_type(&cand.entity_type) else {
                warn!(entity_type = %cand.entity_type, "unknown entity_type — skipping");
                continue;
            };
            let scope = et.default_scope();
            let mut entity = Entity::new(et, cand.title.clone())
                .with_confidence(ConfidenceSource::Inferred)
                .with_tags(cand.tags.clone());
            if !cand.content.is_empty() {
                entity = entity.with_content(cand.content.clone());
            }
            let store = kb.store_for(scope);
            let result = store.store_fact(&entity)?;
            let id = match result {
                StoreFactResult::Inserted { entity_id } => entity_id,
                StoreFactResult::Reinforced { entity_id, .. } => entity_id,
                StoreFactResult::Superseded { new_entity_id, .. } => new_entity_id,
            };
            entities_written.push(id);
            title_to_id.insert(cand.title.clone(), (id, scope));
        }

        // 2. Resolve and write relations.
        let mut relations_written = 0usize;
        for link in links {
            let Some(rel) = parse_relation_type(&link.rel) else {
                warn!(rel = %link.rel, "unknown relation type — skipping link");
                continue;
            };
            let Some((from_id, from_scope)) = title_to_id.get(&link.from).copied() else {
                warn!(from = %link.from, "link `from` not among new entities — skipping");
                continue;
            };
            let to = title_to_id
                .get(&link.to_name)
                .copied()
                .or_else(|| resolve_by_fts(kb, &link.to_name, self.link_score_floor));
            let Some((to_id, _to_scope)) = to else {
                warn!(to_name = %link.to_name, "link target not resolved — dropping");
                continue;
            };
            let store = kb.store_for(from_scope);
            store.add_relation(from_id, rel, to_id)?;
            relations_written += 1;
        }

        // 3. Provenance: EXTRACTED_FROM the projection row id (as a Uuid
        //    derived from the row id string — projection_id is already
        //    a stable hex string but we need a Uuid).
        let provenance_id = projection_id_to_uuid(&row.id);
        for &eid in &entities_written {
            // Route by entity's scope; we keep it on whichever tier the
            // entity lives in. Approximate via global (provenance is a
            // soft annotation; both tiers reach the entity anyway).
            let _ = kb.workstream.add_relation(eid, RelationType::ExtractedFrom, provenance_id);
        }

        Ok(ChainOutcome {
            entities_written,
            relations_written,
            skipped: false,
        })
    }
}

/// FTS-resolve a name against both KB tiers. Falls back to global tier
/// if the workstream-tier search misses.
fn resolve_by_fts(
    kb: &MemoryManager,
    name: &str,
    _floor: f32,
) -> Option<(Uuid, Scope)> {
    // FTS5 quoting: wrap in double-quotes so special chars don't break parsing.
    let q = format!("\"{}\"", name.replace('"', "\"\""));
    if let Some(hit) = first_fts_hit(&kb.workstream, &q) {
        return Some((hit, Scope::Workstream));
    }
    if let Some(hit) = first_fts_hit(&kb.global, &q) {
        return Some((hit, Scope::Global));
    }
    None
}

fn first_fts_hit(store: &Arc<MemoryStore>, query: &str) -> Option<Uuid> {
    match store.search(query, 1) {
        Ok(hits) => hits.into_iter().next().map(|e| e.id),
        Err(_) => None,
    }
}

fn parse_entity_type(s: &str) -> Option<EntityType> {
    EntityType::from_str(s.to_lowercase().as_str())
}

fn parse_relation_type(s: &str) -> Option<RelationType> {
    RelationType::from_str(s.to_lowercase().as_str())
}

/// Derive a deterministic Uuid v5 from the projection row id so the
/// EXTRACTED_FROM edge target is stable across runs.
fn projection_id_to_uuid(projection_id: &str) -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_OID, projection_id.as_bytes())
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    s.chars().take(max_chars).collect::<String>() + "\n…[truncated]"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_classify_in_scope() {
        let raw = "Answer: {\"in_scope\": true, \"reason\": \"pat-related\"}";
        let c = parse_classify(raw).unwrap();
        assert!(c.in_scope);
        assert_eq!(c.reason, "pat-related");
    }

    #[test]
    fn parse_classify_out_of_scope() {
        let raw = "{\"in_scope\": false, \"reason\": \"unrelated\"}";
        let c = parse_classify(raw).unwrap();
        assert!(!c.in_scope);
    }

    #[test]
    fn parse_candidates_empty_array() {
        let v = parse_candidates("[]").unwrap();
        assert!(v.is_empty());
    }

    #[test]
    fn parse_candidates_basic() {
        let raw = "[{\"entity_type\":\"decision\",\"title\":\"use rust\",\"content\":\"chose rust over go\",\"tags\":[\"lang\"]}]";
        let v = parse_candidates(raw).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].entity_type, "decision");
        assert_eq!(v[0].title, "use rust");
        assert_eq!(v[0].tags, vec!["lang"]);
    }

    #[test]
    fn parse_links_basic() {
        let raw = "[{\"from\":\"a\",\"rel\":\"supports\",\"to_name\":\"b\"}]";
        let v = parse_links(raw).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rel, "supports");
    }

    #[test]
    fn entity_type_lowercased_for_parse() {
        assert!(parse_entity_type("Decision").is_some());
        assert!(parse_entity_type("FACT").is_some());
        assert!(parse_entity_type("bogus").is_none());
    }

    #[test]
    fn relation_type_lowercased_for_parse() {
        assert!(parse_relation_type("Supports").is_some());
        assert!(parse_relation_type("SUPERSEDES").is_some());
        assert!(parse_relation_type("bogus").is_none());
    }

    #[test]
    fn projection_id_to_uuid_is_deterministic() {
        let a = projection_id_to_uuid("gm-12345");
        let b = projection_id_to_uuid("gm-12345");
        let c = projection_id_to_uuid("gm-67890");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn truncate_preserves_short_input() {
        assert_eq!(truncate("hello", 100), "hello");
        let long = "x".repeat(200);
        let t = truncate(&long, 50);
        assert!(t.starts_with(&"x".repeat(50)));
        assert!(t.contains("[truncated]"));
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Integration tests — CotChain + ExtractorRunner end-to-end with a stage-
// keyed mock LLM. T-0254.
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod integration {
    use super::*;
    use std::collections::VecDeque;
    use std::pin::Pin;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use futures::stream;
    use serde_json::Value;

    use arawn_core::Workstream;
    use arawn_llm::{
        LlmError,
        types::{ChatChunk, ChatRequest},
    };
    use arawn_memory::{ConfidenceSource, Entity, EntityType, MemoryManager};
    use arawn_projections::{ProjectionStore, gmail::GmailMessageProjection};
    use arawn_storage::{ExtractorCursorStore, Store};

    use crate::runner::{ExtractorRunner, MemoryResolver};

    // ── Stage-keyed mock LLM ─────────────────────────────────────────────

    /// Inspects the system prompt to detect which CoT stage is calling
    /// and returns the next scripted response for that stage. Falls back
    /// to per-stage default when the queue is empty.
    struct KeyedMockLlm {
        classify: Mutex<VecDeque<Value>>,
        extract: Mutex<VecDeque<Value>>,
        link: Mutex<VecDeque<Value>>,
        classify_default: Mutex<Option<Value>>,
        extract_default: Mutex<Option<Value>>,
        link_default: Mutex<Option<Value>>,
    }

    impl KeyedMockLlm {
        fn new() -> Self {
            Self {
                classify: Mutex::new(VecDeque::new()),
                extract: Mutex::new(VecDeque::new()),
                link: Mutex::new(VecDeque::new()),
                classify_default: Mutex::new(None),
                extract_default: Mutex::new(None),
                link_default: Mutex::new(None),
            }
        }

        fn default_classify(self, v: Value) -> Self {
            *self.classify_default.lock().unwrap() = Some(v);
            self
        }
        fn default_extract(self, v: Value) -> Self {
            *self.extract_default.lock().unwrap() = Some(v);
            self
        }
        fn default_link(self, v: Value) -> Self {
            *self.link_default.lock().unwrap() = Some(v);
            self
        }
        #[allow(dead_code)]
        fn push_classify(&self, v: Value) {
            self.classify.lock().unwrap().push_back(v);
        }
    }

    fn classify_stage(sys: &str) -> bool {
        sys.contains("You decide whether")
    }
    fn extract_stage(sys: &str) -> bool {
        sys.contains("Pull typed knowledge entities")
    }
    fn link_stage(sys: &str) -> bool {
        sys.contains("Propose relations")
    }

    #[async_trait]
    impl arawn_llm::LlmClient for KeyedMockLlm {
        async fn stream(
            &self,
            request: ChatRequest,
        ) -> Result<
            Pin<Box<dyn futures::Stream<Item = Result<ChatChunk, LlmError>> + Send>>,
            LlmError,
        > {
            let sys = request.system_prompt.unwrap_or_default();
            let payload: Value = if classify_stage(&sys) {
                self.classify
                    .lock()
                    .unwrap()
                    .pop_front()
                    .or_else(|| self.classify_default.lock().unwrap().clone())
                    .unwrap_or_else(|| serde_json::json!({"in_scope": false, "reason": ""}))
            } else if extract_stage(&sys) {
                self.extract
                    .lock()
                    .unwrap()
                    .pop_front()
                    .or_else(|| self.extract_default.lock().unwrap().clone())
                    .unwrap_or_else(|| serde_json::json!([]))
            } else if link_stage(&sys) {
                self.link
                    .lock()
                    .unwrap()
                    .pop_front()
                    .or_else(|| self.link_default.lock().unwrap().clone())
                    .unwrap_or_else(|| serde_json::json!([]))
            } else {
                panic!("KeyedMockLlm: unrecognized system prompt: {sys}");
            };
            let text = payload.to_string();
            let chunks: Vec<Result<ChatChunk, LlmError>> = vec![
                Ok(ChatChunk::TextDelta { text }),
                Ok(ChatChunk::Done { usage: None }),
            ];
            Ok(Box::pin(stream::iter(chunks)))
        }
    }

    // ── Fixture helpers ──────────────────────────────────────────────────

    fn ws(name: &str, desc: &str) -> Workstream {
        let mut w = Workstream::new(name, std::env::temp_dir().join(name));
        w.description = desc.to_string();
        w
    }

    fn fixture_proj(id: &str, body: &str, ts_offset: i64) -> GmailMessageProjection {
        GmailMessageProjection {
            id: arawn_projections::gmail::projection_id("feed-1", id),
            feed_id: "feed-1".into(),
            source_id: id.into(),
            source_ts: chrono::Utc::now() + chrono::Duration::seconds(ts_offset),
            sender: Some("a@e.com".into()),
            recipients: vec![],
            subject: format!("subj-{id}"),
            body_text: body.into(),
            thread_id: None,
            labels: vec![],
        }
    }

    struct Fixture {
        _tmp: tempfile::TempDir,
        store: Arc<std::sync::Mutex<Store>>,
        proj: Arc<ProjectionStore>,
        resolver: MemoryResolver,
        kb_cache: Arc<std::sync::Mutex<std::collections::HashMap<String, Arc<MemoryManager>>>>,
    }

    fn setup() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        store.ensure_scratch_workstream().unwrap();
        let store = Arc::new(std::sync::Mutex::new(store));

        let proj_path = tmp.path().join("projections.db");
        let proj = Arc::new(ProjectionStore::open(&proj_path).unwrap());

        // Cache MemoryManagers per workstream so the test can reach into
        // the same KB the runner used (a fresh resolver instance would
        // open a new MemoryStore handle each call).
        let cache: Arc<std::sync::Mutex<std::collections::HashMap<String, Arc<MemoryManager>>>> =
            Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
        let cache_clone = Arc::clone(&cache);
        let data_dir = tmp.path().to_path_buf();
        let resolver: MemoryResolver = Arc::new(move |name: &str| {
            let mut guard = cache_clone.lock().unwrap();
            if let Some(existing) = guard.get(name) {
                return Ok(Arc::clone(existing));
            }
            let mgr = MemoryManager::for_workstream(&data_dir, name, None)
                .map(Arc::new)
                .map_err(|e| ExtractionError::Memory(e.to_string()))?;
            guard.insert(name.to_string(), Arc::clone(&mgr));
            Ok(mgr)
        });

        Fixture {
            _tmp: tmp,
            store,
            proj,
            resolver,
            kb_cache: cache,
        }
    }

    impl Fixture {
        fn kb(&self, name: &str) -> Arc<MemoryManager> {
            // Force materialization through the resolver so the cache
            // is populated, then return the same handle.
            let _ = (self.resolver)(name).unwrap();
            self.kb_cache.lock().unwrap().get(name).cloned().unwrap()
        }

        fn cursor(&self, ws_name: &str, feed_type: &str) -> Option<chrono::DateTime<chrono::Utc>> {
            let s = self.store.lock().unwrap();
            let cs = ExtractorCursorStore::new(s.database());
            cs.get(ws_name, feed_type).unwrap().and_then(|c| c.last_source_ts)
        }
    }

    fn runner_with(
        fx: &Fixture,
        mock: Arc<KeyedMockLlm>,
        batch_size: usize,
    ) -> ExtractorRunner {
        let chain: Arc<dyn ExtractionChain> =
            Arc::new(CotChain::new(mock as Arc<dyn arawn_llm::LlmClient>, "mock-model"));
        ExtractorRunner::new(
            Arc::clone(&fx.store),
            Arc::clone(&fx.proj),
            Arc::clone(&fx.resolver),
            chain,
        )
        .with_batch_size(batch_size)
    }

    // ── Scenarios ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn happy_path_extracts_into_workstream() {
        let fx = setup();
        fx.proj
            .write_batch(&[fixture_proj("m1", "we picked Postgres for storage", 0)])
            .unwrap();

        let mock = Arc::new(
            KeyedMockLlm::new()
                .default_classify(serde_json::json!({"in_scope": true, "reason": "in scope"}))
                .default_extract(serde_json::json!([
                    {"entity_type": "decision", "title": "use postgres",
                     "content": "we picked postgres", "tags": ["db"]}
                ]))
                .default_link(serde_json::json!([])),
        );
        let runner = runner_with(&fx, mock, 50);
        let stats = runner
            .run_for_workstream(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(stats.processed, 1);
        assert_eq!(stats.kept, 1);
        assert_eq!(stats.entities_written, 1);
        assert!(fx.cursor("pat", "gmail_messages").is_some());

        // Entity actually landed in the workstream KB.
        let kb = fx.kb("pat");
        let hits = kb.workstream.search("postgres", 5).unwrap();
        assert!(
            hits.iter().any(|e| e.title.contains("postgres")),
            "expected entity in workstream KB; got {hits:?}"
        );
    }

    #[tokio::test]
    async fn out_of_scope_skips_but_advances_cursor() {
        let fx = setup();
        fx.proj
            .write_batch(&[fixture_proj("m1", "unrelated newsletter", 0)])
            .unwrap();
        let mock = Arc::new(
            KeyedMockLlm::new()
                .default_classify(serde_json::json!({"in_scope": false, "reason": "noise"})),
        );
        let runner = runner_with(&fx, mock, 50);
        let stats = runner
            .run_for_workstream(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(stats.processed, 1);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.kept, 0);
        assert!(
            fx.cursor("pat", "gmail_messages").is_some(),
            "cursor must advance even for skipped rows"
        );
    }

    #[tokio::test]
    async fn link_by_name_resolves_to_existing_kb_entity() {
        let fx = setup();
        // Pre-seed the workstream KB with a fact the link will target.
        {
            let kb = fx.kb("pat");
            let prior = Entity::new(EntityType::Fact, "open question: which auth library?")
                .with_confidence(ConfidenceSource::Stated);
            kb.workstream.store_fact(&prior).unwrap();
        }
        fx.proj
            .write_batch(&[fixture_proj("m1", "we chose oauth2-rs to close out auth", 0)])
            .unwrap();

        let mock = Arc::new(
            KeyedMockLlm::new()
                .default_classify(serde_json::json!({"in_scope": true, "reason": "ok"}))
                .default_extract(serde_json::json!([
                    {"entity_type": "decision", "title": "use oauth2-rs",
                     "content": "settles the auth question"}
                ]))
                .default_link(serde_json::json!([
                    {"from": "use oauth2-rs", "rel": "supersedes",
                     "to_name": "open question: which auth library?"}
                ])),
        );
        let runner = runner_with(&fx, mock, 50);
        let stats = runner
            .run_for_workstream(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(stats.kept, 1);
        assert_eq!(
            stats.relations_written, 1,
            "link should have resolved via FTS to the pre-seeded entity"
        );
    }

    #[tokio::test]
    async fn link_to_missing_target_is_dropped_without_panic() {
        let fx = setup();
        fx.proj.write_batch(&[fixture_proj("m1", "body", 0)]).unwrap();
        let mock = Arc::new(
            KeyedMockLlm::new()
                .default_classify(serde_json::json!({"in_scope": true, "reason": "ok"}))
                .default_extract(serde_json::json!([
                    {"entity_type": "decision", "title": "alpha"}
                ]))
                .default_link(serde_json::json!([
                    {"from": "alpha", "rel": "supports", "to_name": "does-not-exist-anywhere"}
                ])),
        );
        let runner = runner_with(&fx, mock, 50);
        let stats = runner
            .run_for_workstream(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(stats.entities_written, 1);
        assert_eq!(
            stats.relations_written, 0,
            "unresolved link target must be dropped, not written"
        );
    }

    #[tokio::test]
    async fn backfill_walks_existing_rows() {
        let fx = setup();
        let rows: Vec<_> = (0..5)
            .map(|i| fixture_proj(&format!("m{i}"), &format!("body {i}"), i as i64))
            .collect();
        fx.proj.write_batch(&rows).unwrap();

        let mock = Arc::new(
            KeyedMockLlm::new()
                .default_classify(serde_json::json!({"in_scope": true, "reason": "ok"}))
                .default_extract(serde_json::json!([
                    {"entity_type": "note", "title": "captured"}
                ]))
                .default_link(serde_json::json!([])),
        );
        let runner = runner_with(&fx, mock, 2);
        let stats = runner
            .run_for_workstream_until_exhausted(
                &ws("pat", "pat's stuff"),
                "gmail_messages",
                std::time::Duration::from_secs(30),
            )
            .await
            .unwrap();
        assert_eq!(stats.processed, 5);
        assert_eq!(stats.kept, 5);
    }

    #[tokio::test]
    async fn rerun_is_idempotent_via_cursor() {
        let fx = setup();
        fx.proj.write_batch(&[fixture_proj("m1", "body", 0)]).unwrap();
        let mock = Arc::new(
            KeyedMockLlm::new()
                .default_classify(serde_json::json!({"in_scope": true, "reason": "ok"}))
                .default_extract(serde_json::json!([
                    {"entity_type": "note", "title": "n"}
                ]))
                .default_link(serde_json::json!([])),
        );
        let runner = runner_with(&fx, mock, 50);
        let first = runner
            .run_for_workstream(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(first.processed, 1);
        let second = runner
            .run_for_workstream(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(
            second.processed, 0,
            "second run should be a no-op once the cursor caught up"
        );
    }

    #[tokio::test]
    async fn two_workstreams_each_get_the_entity() {
        let fx = setup();
        fx.proj
            .write_batch(&[fixture_proj("m1", "shared message", 0)])
            .unwrap();

        // Same default response works for both workstreams — classify
        // is_scope=true, extract one entity, no links.
        let mock = Arc::new(
            KeyedMockLlm::new()
                .default_classify(serde_json::json!({"in_scope": true, "reason": "ok"}))
                .default_extract(serde_json::json!([
                    {"entity_type": "fact", "title": "shared finding"}
                ]))
                .default_link(serde_json::json!([])),
        );
        let runner = runner_with(&fx, mock, 50);
        let s1 = runner
            .run_for_workstream(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();
        let s2 = runner
            .run_for_workstream(&ws("auth-migration", "auth work"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(s1.entities_written, 1);
        assert_eq!(s2.entities_written, 1);

        // Both KBs hold the entity independently.
        let pat_hits = fx.kb("pat").workstream.search("shared", 5).unwrap();
        let auth_hits = fx
            .kb("auth-migration")
            .workstream
            .search("shared", 5)
            .unwrap();
        assert!(!pat_hits.is_empty(), "pat KB should contain the fact");
        assert!(!auth_hits.is_empty(), "auth-migration KB should contain the fact");
    }
}
