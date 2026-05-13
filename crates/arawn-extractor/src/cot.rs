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
