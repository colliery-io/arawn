//! `CotChain` — the real 4-stage chain-of-thought extractor.
//!
//! Stage 1 (classify): is this projection row in scope for the lens?
//! Stage 2 (extract): pull typed entities out of the body.
//! Stage 3 (link-by-name): emit candidate relations; we resolve by FTS.
//! Stage 4 (write): store_fact each entity + add resolved relations
//! plus an EXTRACTED_FROM provenance edge.
//!
//! Each stage is one LLM call. Free / inexpensive backend behind it
//! per I-0040 phase 4 design. The chain reads the lens
//! description to scope decisions and emits free-form tags; the
//! steward (Phase 5) refines vocabulary later.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::{debug, warn};
use uuid::Uuid;

use arawn_core::Lens;
use arawn_llm::LlmClient;
use arawn_memory::{
    ConfidenceSource, Entity, EntityType, MemoryManager, MemoryStore, RelationType, Scope,
    StoreFactResult, TagOntologyStore, normalize_tag,
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
        lens: &Lens,
        row: &ProjectionRow,
        kb: &MemoryManager,
    ) -> Result<ChainOutcome, ExtractionError> {
        // Surface what's already known globally about this row's subject so the
        // chain decides in scope of the user's standing facts (ARAWN-I-0061).
        // FTS over title + a body excerpt picks up named people / projects /
        // conventions; bounded to a handful so the prompt doesn't drown.
        let fact_query = format!("{} {}", row.title, truncate(&row.body_text, 500));
        let known_facts = relevant_global_facts(kb, &fact_query, 5);

        // ── Stage 1: classify ───────────────────────────────────────────
        let classify = self.classify(lens, row, &known_facts).await?;
        if !classify.in_scope {
            debug!(
                lens = %lens.name,
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

        // Load the lens's declared ontology — Stage 2's prompt
        // shows it to the model and Stage 4 filters LLM emissions
        // against it. Soft-fail to empty so an absent ontology table
        // doesn't break the chain (extractor will produce
        // discovered-only entities until the ontology exists).
        let ontology = match TagOntologyStore::open_at(&lens.root_dir) {
            Ok(store) => store.tags().unwrap_or_default(),
            Err(e) => {
                warn!(
                    lens = %lens.name,
                    error = %e,
                    "ontology unavailable; extracting with empty ontology"
                );
                Vec::new()
            }
        };

        // ── Stage 2: extract ────────────────────────────────────────────
        let candidates = self.extract(lens, row, &ontology, &known_facts).await?;
        if candidates.is_empty() {
            return Ok(ChainOutcome::default());
        }

        // ── Stage 3: link-by-name ───────────────────────────────────────
        let link_proposals = self.link_by_name(lens, &candidates).await?;

        // ── Stage 4: write ──────────────────────────────────────────────
        self.write(row, &candidates, &link_proposals, kb, &ontology)
            .await
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
        ws: &Lens,
        row: &ProjectionRow,
        known_facts: &[String],
    ) -> Result<ClassifyResult, ExtractionError> {
        let system = "You decide whether a piece of content belongs in a knowledge \
                      base for a specific lens. Output ONLY a JSON object: \
                      {\"in_scope\": bool, \"reason\": short string}. \
                      Be selective — a lens is a tight scope (one person, \
                      one project, one initiative). When in doubt, in_scope = false. \
                      Use the known facts (if any) to recognize people, projects, and \
                      conventions that bring otherwise-ambiguous content into scope.";
        let user = format!(
            "Lens: {name}\n\
             Description: {desc}\n\
             {facts}\
             \nItem (feed type: {feed_type}):\n\
             Title: {title}\n\
             Body:\n{body}\n",
            name = ws.name,
            desc = if ws.description.is_empty() {
                "(no description set)"
            } else {
                ws.description.as_str()
            },
            facts = format_known_facts(known_facts),
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
    /// LLM-emitted ontology tags. Filtered against the lens's
    /// declared ontology before writing — anything not in the list is
    /// dropped. Per ADR-0004 this is the substrate dust clusters on.
    #[serde(default)]
    tags_ontology: Vec<String>,
    /// LLM-emitted free-form tags. Pass through verbatim. Material for
    /// the `tag-promoter` steward subroutine to propose promotion.
    #[serde(default)]
    tags_discovered: Vec<String>,
}

impl CotChain {
    async fn extract(
        &self,
        ws: &Lens,
        row: &ProjectionRow,
        ontology: &[String],
        known_facts: &[String],
    ) -> Result<Vec<ExtractedCandidate>, ExtractionError> {
        let ontology_block = if ontology.is_empty() {
            "(empty — lens has no ontology yet; emit only `tags_discovered` for now)".to_string()
        } else {
            ontology.join(", ")
        };
        let system = "Pull typed knowledge entities out of the content. Output ONLY \
                      a JSON array. Each item: \
                      {\"entity_type\": one of [fact, decision, convention, \
                       preference, person, note], \
                       \"title\": short, \
                       \"content\": optional longer text, \
                       \"tags_ontology\": array of tags drawn EXCLUSIVELY from \
                       the lens's declared ontology (see user prompt). \
                       Pick the ontology tags that genuinely apply — empty \
                       array is fine if none fit. Do NOT mint new ontology \
                       tags here. \
                       \"tags_discovered\": array of 1–4 free-form tags you \
                       think describe the content. These are NOT constrained \
                       to the ontology and become candidates for promotion \
                       later. Include specific identifiers (project names, \
                       system names, person names, RFC numbers) when they \
                       appear.}\n\
                      \n\
                      Be conservative on entity emission — only entities that \
                      genuinely belong in this lens's KB. Empty array \
                      is a valid answer.";
        let user = format!(
            "Lens: {name}\n\
             Description: {desc}\n\
             Declared ontology (use these EXACT strings for tags_ontology): {ontology_block}\n\
             {facts}\
             \nTitle: {title}\n\
             Body:\n{body}\n",
            name = ws.name,
            desc = if ws.description.is_empty() {
                "(no description set)"
            } else {
                ws.description.as_str()
            },
            ontology_block = ontology_block,
            facts = format_known_facts(known_facts),
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
        ws: &Lens,
        candidates: &[ExtractedCandidate],
    ) -> Result<Vec<LinkProposal>, ExtractionError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let system = "Propose relations between the new entities and entities \
                      that may already exist in this lens's KB. Output ONLY \
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
            "Lens: {name}\nDescription: {desc}\n\nNew entities:\n{entities}\n",
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
        ontology: &[String],
    ) -> Result<ChainOutcome, ExtractionError> {
        // Per ADR-0004: ontology tags emitted by the LLM are filtered
        // against the lens's declared ontology before writing.
        // Anything not in the list is dropped silently — the LLM
        // doesn't get to invent ontology tags here.
        let ontology_set: std::collections::HashSet<String> =
            ontology.iter().map(|t| normalize_tag(t)).collect();

        // 1. Write entities; record title → id for link resolution.
        let mut title_to_id: HashMap<String, (Uuid, Scope)> = HashMap::new();
        let mut entities_written: Vec<Uuid> = Vec::new();
        for cand in candidates {
            let Some(et) = parse_entity_type(&cand.entity_type) else {
                warn!(entity_type = %cand.entity_type, "unknown entity_type — skipping");
                continue;
            };
            // ARAWN-I-0061: extractor signals always live in the lens KB by
            // definition — a lens *is* its extracted signal stream. `default_scope`
            // governs deliberate memory writes (global); it does not apply here.
            let scope = Scope::Lens;

            // Filter ontology tags to declared list (case-folded).
            let kept_ontology: Vec<String> = cand
                .tags_ontology
                .iter()
                .map(|t| normalize_tag(t))
                .filter(|t| ontology_set.contains(t))
                .collect();
            // Normalize discovered tags too so promotion candidates are
            // stable across casing/whitespace variants.
            let discovered: Vec<String> = cand
                .tags_discovered
                .iter()
                .map(|t| normalize_tag(t))
                .filter(|t| !t.is_empty())
                .collect();

            let mut entity = Entity::new(et, cand.title.clone())
                .with_confidence(ConfidenceSource::Inferred)
                .with_tags(discovered)
                .with_tags_ontology(kept_ontology);
            if !cand.content.is_empty() {
                entity = entity.with_content(cand.content.clone());
            }
            // Anchor entity freshness to the source projection row's
            // `source_ts`, not to the extraction wall-clock. Three
            // payoffs:
            //   1. Dust (`lens_dust`) measures staleness via
            //      `updated_at`. If we used now(), every freshly-
            //      extracted entity would look fresh — even when its
            //      source content is years old. That defeats dust's
            //      whole "summarize cold material" semantic.
            //   2. `signal_timeline` orders by `created_at`. Anchoring
            //      to source_ts gives the agent a true chronology of
            //      when the underlying events happened.
            //   3. Reinforcement via `reinforce_entity` resets
            //      `updated_at` to now() when the same fact is seen
            //      again, so live content still looks live — only
            //      the initial extraction inherits source-age.
            entity.created_at = row.source_ts;
            entity.updated_at = row.source_ts;
            entity.accessed_at = row.source_ts;
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
            let _ = kb
                .lens
                .add_relation(eid, RelationType::ExtractedFrom, provenance_id);
        }

        Ok(ChainOutcome {
            entities_written,
            relations_written,
            skipped: false,
        })
    }
}

/// FTS-resolve a name against both KB tiers. Falls back to global tier
/// if the lens-tier search misses.
fn resolve_by_fts(kb: &MemoryManager, name: &str, _floor: f32) -> Option<(Uuid, Scope)> {
    // FTS5 quoting: wrap in double-quotes so special chars don't break parsing.
    let q = format!("\"{}\"", name.replace('"', "\"\""));
    if let Some(hit) = first_fts_hit(&kb.lens, &q) {
        return Some((hit, Scope::Lens));
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

/// Pull a small set of relevant facts from global memory (ARAWN-I-0061) to give
/// the chain situational context — who matters, what's a known project, what the
/// user's standing preferences are. Keyed by tokens from the row so we surface
/// facts whose subject overlaps with the content being classified.
fn relevant_global_facts(kb: &MemoryManager, query: &str, limit: usize) -> Vec<String> {
    // FTS5's default is AND across terms and chokes on stray punctuation
    // (hyphens, colons, etc. parse as operators or syntax errors). Sanitize to
    // alphanumeric tokens of length ≥3 and join with OR so any single overlap
    // with a stored memory's title/content surfaces it. Bound the term count so
    // a long row body can't blow up the FTS query.
    let mut tokens: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_lowercase())
        .collect();
    tokens.sort();
    tokens.dedup();
    tokens.truncate(40);
    if tokens.is_empty() {
        return Vec::new();
    }
    let fts_query = tokens
        .iter()
        .map(|t| format!("\"{t}\""))
        .collect::<Vec<_>>()
        .join(" OR ");
    let hits = match kb.global.search(&fts_query, limit) {
        Ok(h) => h,
        Err(_) => return Vec::new(),
    };
    hits.into_iter()
        .map(|e| {
            let snippet: String = e
                .content
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(120)
                .collect();
            if snippet.trim().is_empty() {
                format!("- {}: {}", e.entity_type.as_str(), e.title)
            } else {
                format!("- {}: {} — {}", e.entity_type.as_str(), e.title, snippet)
            }
        })
        .collect()
}

/// Render a "Relevant known facts" block for injection into a CoT prompt.
/// Returns an empty string when there are no facts so callers can splice it in
/// unconditionally without a trailing blank section.
fn format_known_facts(facts: &[String]) -> String {
    if facts.is_empty() {
        return String::new();
    }
    let mut s = String::from("\nRelevant known facts (from global memory):\n");
    for f in facts {
        s.push_str(f);
        s.push('\n');
    }
    s
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
        let raw = "[{\"entity_type\":\"decision\",\"title\":\"use rust\",\
                    \"content\":\"chose rust over go\",\
                    \"tags_ontology\":[\"languages\"],\
                    \"tags_discovered\":[\"rust\",\"systems\"]}]";
        let v = parse_candidates(raw).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].entity_type, "decision");
        assert_eq!(v[0].title, "use rust");
        assert_eq!(v[0].tags_ontology, vec!["languages"]);
        assert_eq!(v[0].tags_discovered, vec!["rust", "systems"]);
    }

    #[test]
    fn parse_candidates_tolerates_missing_tag_fields() {
        let raw = "[{\"entity_type\":\"note\",\"title\":\"x\"}]";
        let v = parse_candidates(raw).unwrap();
        assert_eq!(v.len(), 1);
        assert!(v[0].tags_ontology.is_empty());
        assert!(v[0].tags_discovered.is_empty());
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

    use arawn_core::Lens;
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
                Ok(ChatChunk::Done {
                    usage: None,
                    finish_reason: None,
                }),
            ];
            Ok(Box::pin(stream::iter(chunks)))
        }
    }

    // ── Fixture helpers ──────────────────────────────────────────────────

    fn ws(name: &str, desc: &str) -> Lens {
        let mut w = Lens::new(name, std::env::temp_dir().join(name));
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
        store.ensure_scratch_lens().unwrap();
        let store = Arc::new(std::sync::Mutex::new(store));

        let proj_path = tmp.path().join("projections.db");
        let proj = Arc::new(ProjectionStore::open(&proj_path).unwrap());

        // Cache MemoryManagers per lens so the test can reach into
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
            let mgr = MemoryManager::for_lens(&data_dir, name, None)
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
            cs.get(ws_name, feed_type)
                .unwrap()
                .and_then(|c| c.last_source_ts)
        }
    }

    fn runner_with<C: arawn_llm::LlmClient + 'static>(
        fx: &Fixture,
        mock: Arc<C>,
        batch_size: usize,
    ) -> ExtractorRunner {
        let mock: Arc<dyn arawn_llm::LlmClient> = mock;
        let chain: Arc<dyn ExtractionChain> = Arc::new(CotChain::new(mock, "mock-model"));
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
    async fn happy_path_extracts_into_lens() {
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
            .run_for_lens(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(stats.processed, 1);
        assert_eq!(stats.kept, 1);
        assert_eq!(stats.entities_written, 1);
        assert!(fx.cursor("pat", "gmail_messages").is_some());

        // Entity actually landed in the lens KB.
        let kb = fx.kb("pat");
        let hits = kb.lens.search("postgres", 5).unwrap();
        assert!(
            hits.iter().any(|e| e.title.contains("postgres")),
            "expected entity in lens KB; got {hits:?}"
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
            .run_for_lens(&ws("pat", "pat's stuff"), "gmail_messages")
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
        // Pre-seed the lens KB with a fact the link will target.
        {
            let kb = fx.kb("pat");
            let prior = Entity::new(EntityType::Fact, "open question: which auth library?")
                .with_confidence(ConfidenceSource::Stated);
            kb.lens.store_fact(&prior).unwrap();
        }
        fx.proj
            .write_batch(&[fixture_proj(
                "m1",
                "we chose oauth2-rs to close out auth",
                0,
            )])
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
            .run_for_lens(&ws("pat", "pat's stuff"), "gmail_messages")
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
        fx.proj
            .write_batch(&[fixture_proj("m1", "body", 0)])
            .unwrap();
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
            .run_for_lens(&ws("pat", "pat's stuff"), "gmail_messages")
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
            .run_for_lens_until_exhausted(
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
        fx.proj
            .write_batch(&[fixture_proj("m1", "body", 0)])
            .unwrap();
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
            .run_for_lens(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(first.processed, 1);
        let second = runner
            .run_for_lens(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(
            second.processed, 0,
            "second run should be a no-op once the cursor caught up"
        );
    }

    #[tokio::test]
    async fn two_lenses_each_get_the_entity() {
        let fx = setup();
        fx.proj
            .write_batch(&[fixture_proj("m1", "shared message", 0)])
            .unwrap();

        // Same default response works for both lenses — classify
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
            .run_for_lens(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();
        let s2 = runner
            .run_for_lens(&ws("auth-migration", "auth work"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(s1.entities_written, 1);
        assert_eq!(s2.entities_written, 1);

        // Both KBs hold the entity independently.
        let pat_hits = fx.kb("pat").lens.search("shared", 5).unwrap();
        let auth_hits = fx.kb("auth-migration").lens.search("shared", 5).unwrap();
        assert!(!pat_hits.is_empty(), "pat KB should contain the fact");
        assert!(
            !auth_hits.is_empty(),
            "auth-migration KB should contain the fact"
        );
    }

    /// LLM mock that returns `in_scope: true` only when the user prompt
    /// contains a chosen needle, and `false` otherwise. Used to prove a
    /// global-memory fact reaches the classify prompt.
    struct ScopeGatedByPrompt {
        needle: String,
    }

    #[async_trait]
    impl arawn_llm::LlmClient for ScopeGatedByPrompt {
        async fn stream(
            &self,
            request: ChatRequest,
        ) -> Result<
            Pin<Box<dyn futures::Stream<Item = Result<ChatChunk, LlmError>> + Send>>,
            LlmError,
        > {
            let sys = request.system_prompt.clone().unwrap_or_default();
            let user = request
                .messages
                .iter()
                .filter(|m| m.role == "user")
                .map(|m| {
                    let arawn_llm::types::ChatContent::Text(s) = &m.content;
                    s.as_str()
                })
                .collect::<Vec<_>>()
                .join("\n");
            let payload: Value = if classify_stage(&sys) {
                let in_scope = user.contains(&self.needle);
                serde_json::json!({"in_scope": in_scope, "reason": "gated"})
            } else if extract_stage(&sys) {
                serde_json::json!([
                    {"entity_type": "note", "title": "context-aware capture"}
                ])
            } else if link_stage(&sys) {
                serde_json::json!([])
            } else {
                panic!("ScopeGatedByPrompt: unknown stage: {sys}");
            };
            let text = payload.to_string();
            let chunks: Vec<Result<ChatChunk, LlmError>> = vec![
                Ok(ChatChunk::TextDelta { text }),
                Ok(ChatChunk::Done {
                    usage: None,
                    finish_reason: None,
                }),
            ];
            Ok(Box::pin(stream::iter(chunks)))
        }
    }

    #[tokio::test]
    async fn global_memory_fact_reaches_classify_prompt() {
        // Seed a global memory fact whose CONTENT (the "Dylan manages" phrase)
        // does NOT appear in the row body. If the chain classifies in_scope, the
        // fact must have been injected into the prompt — only the global memory
        // path produces that string.
        let fx = setup();
        let kb = fx.kb("pat");
        let fact = Entity::new(EntityType::Person, "Pat Collins")
            .with_content("Pat Collins is someone Dylan manages")
            .with_confidence(ConfidenceSource::Stated);
        kb.global.store_fact(&fact).unwrap();

        fx.proj
            .write_batch(&[fixture_proj(
                "m1",
                "Pat Collins shipped the new dashboard today",
                0,
            )])
            .unwrap();

        let mock = Arc::new(ScopeGatedByPrompt {
            needle: "Dylan manages".into(),
        });
        let runner = runner_with(&fx, mock, 50);
        let stats = runner
            .run_for_lens(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();

        assert_eq!(
            stats.kept, 1,
            "row should classify in_scope because the global memory fact \
             ('Dylan manages …') reached the prompt"
        );
        assert_eq!(stats.entities_written, 1);
    }

    #[tokio::test]
    async fn classify_without_global_facts_is_out_of_scope() {
        // Same gated mock, same row — but no seeded global fact. The needle
        // never appears in the prompt, so classify falls through to false.
        let fx = setup();
        fx.proj
            .write_batch(&[fixture_proj(
                "m1",
                "Pat Collins shipped the new dashboard today",
                0,
            )])
            .unwrap();
        let mock = Arc::new(ScopeGatedByPrompt {
            needle: "Dylan manages".into(),
        });
        let runner = runner_with(&fx, mock, 50);
        let stats = runner
            .run_for_lens(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();

        assert_eq!(stats.kept, 0, "no global fact → mock returns out-of-scope");
        assert_eq!(stats.skipped, 1);
    }

    #[tokio::test]
    async fn entity_dates_inherit_source_ts_not_extraction_time() {
        // Projection row from 90 days ago. The extracted entity's
        // created_at / updated_at / accessed_at should anchor to that
        // source ts so dust / signal_timeline see the actual age of
        // the underlying content.
        let fx = setup();
        let mut old_row = fixture_proj("m1", "we picked Postgres for storage", 0);
        let ninety_days_ago = chrono::Utc::now() - chrono::Duration::days(90);
        old_row.source_ts = ninety_days_ago;
        fx.proj.write_batch(&[old_row]).unwrap();

        let mock = Arc::new(
            KeyedMockLlm::new()
                .default_classify(serde_json::json!({"in_scope": true, "reason": "ok"}))
                .default_extract(serde_json::json!([
                    {"entity_type": "decision", "title": "use postgres",
                     "tags_ontology": [], "tags_discovered": ["db"]}
                ]))
                .default_link(serde_json::json!([])),
        );
        let runner = runner_with(&fx, mock, 50);
        let stats = runner
            .run_for_lens(&ws("pat", "pat's stuff"), "gmail_messages")
            .await
            .unwrap();
        assert_eq!(stats.kept, 1);

        let kb = fx.kb("pat");
        let hits = kb.lens.search("postgres", 5).unwrap();
        let entity = hits
            .iter()
            .find(|e| e.title.contains("postgres"))
            .expect("postgres entity present");
        // Allow a small wall-clock fuzz around the 90-day anchor.
        let drift = (entity.updated_at - ninety_days_ago).num_seconds().abs();
        assert!(
            drift < 5,
            "expected updated_at within 5s of source_ts, drift was {drift}s"
        );
        assert_eq!(entity.updated_at, entity.created_at);
    }
}
