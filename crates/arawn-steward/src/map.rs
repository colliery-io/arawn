//! Map subroutine — proposal-only. Looks at recent entities in a
//! workstream and asks the LLM to suggest relations that *should* exist
//! between them but don't. Proposals are journaled with `applied=false`
//! and surface through `/workstream refine`.
//!
//! Per ARAWN-A-0003 map never mutates the KB graph.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tracing::{debug, warn};

use arawn_llm::LlmClient;
use arawn_memory::{Entity, RelationType};

use crate::cursor::CursorStore;
use crate::error::StewardError;
use crate::journal::{Journal, JournalRecord};
use crate::llm_text::{complete_text, extract_json_block};
use crate::subroutine::{StewardSubroutine, SubroutineCtx, SubroutineOutcome};

const SUBROUTINE_NAME: &str = "map";

/// Relations map is allowed to propose. Excludes `Supersedes` (re-shelve's
/// domain) and `ExtractedFrom` (provenance).
fn is_proposable(rel: RelationType) -> bool {
    matches!(
        rel,
        RelationType::RelatesTo
            | RelationType::Supports
            | RelationType::Contradicts
            | RelationType::Mentions
            | RelationType::BelongsTo
    )
}

#[derive(Debug, Clone)]
pub struct MapConfig {
    /// How many touched-since-cursor entities to take as focus.
    pub batch_size: usize,
    /// How many sibling entities to feed the LLM alongside each focus
    /// (drawn from `list_all_ranked` so the LLM sees a mix).
    pub neighbors_per_focus: usize,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            neighbors_per_focus: 6,
        }
    }
}

pub struct MapSubroutine {
    client: Arc<dyn LlmClient>,
    model: String,
    config: MapConfig,
    cursor_factory: Arc<dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync>,
}

impl MapSubroutine {
    pub fn new(
        client: Arc<dyn LlmClient>,
        model: impl Into<String>,
        cursor_factory: Arc<dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync>,
    ) -> Self {
        Self {
            client,
            model: model.into(),
            config: MapConfig::default(),
            cursor_factory,
        }
    }

    pub fn with_config(mut self, config: MapConfig) -> Self {
        self.config = config;
        self
    }
}

#[derive(Debug, Deserialize)]
struct ProposedEdge {
    from_id: String,
    rel: String,
    to_id: String,
    #[serde(default)]
    reason: String,
}

#[async_trait]
impl StewardSubroutine for MapSubroutine {
    fn name(&self) -> &str {
        SUBROUTINE_NAME
    }

    fn is_mutating(&self) -> bool {
        false
    }

    async fn run(&self, ctx: &SubroutineCtx) -> Result<SubroutineOutcome, StewardError> {
        let cursor_store = (self.cursor_factory)(&ctx.workstream.name)?;
        let cursor = cursor_store.get(SUBROUTINE_NAME)?;

        let mut all = ctx
            .memory
            .workstream
            .list_all_ranked(500)
            .map_err(StewardError::from)?;
        let mut focus_set: Vec<Entity> = all
            .iter()
            .filter(|e| cursor.is_none_or(|c| e.updated_at > c))
            .cloned()
            .collect();
        focus_set.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
        focus_set.truncate(self.config.batch_size);
        if focus_set.is_empty() {
            return Ok(SubroutineOutcome::default());
        }

        // Neighbor pool — everything currently in the KB (cap to a
        // sane size so the prompt doesn't explode).
        all.truncate(64);
        let mut outcome = SubroutineOutcome::default();
        let mut latest_seen = cursor;

        for focus in &focus_set {
            if outcome.proposals_recorded >= ctx.cap {
                outcome.cap_hit = true;
                break;
            }
            // Sample N neighbors — skip self + already-proposed-against
            // pairs. v1 is naive: take the top of the ranked list.
            let neighbors: Vec<&Entity> = all
                .iter()
                .filter(|e| e.id != focus.id)
                .take(self.config.neighbors_per_focus)
                .collect();
            if neighbors.is_empty() {
                continue;
            }

            let proposals = match self
                .propose_for(focus, &neighbors, ctx)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    warn!(
                        workstream = %ctx.workstream.name,
                        focus = %focus.id,
                        error = %e,
                        "map: per-focus LLM call failed; skipping"
                    );
                    continue;
                }
            };

            for prop in proposals {
                if outcome.proposals_recorded >= ctx.cap {
                    outcome.cap_hit = true;
                    break;
                }
                if let Err(e) = self.record_proposal(focus, &prop, ctx) {
                    warn!(
                        workstream = %ctx.workstream.name,
                        focus = %focus.id,
                        error = %e,
                        "map: dropped proposal"
                    );
                    continue;
                }
                outcome.proposals_recorded += 1;
                outcome.actions_journaled += 1;
            }
            if latest_seen.map(|p| focus.updated_at > p).unwrap_or(true) {
                latest_seen = Some(focus.updated_at);
            }
        }

        if let Some(ts) = latest_seen {
            cursor_store.advance(SUBROUTINE_NAME, ts)?;
        }
        Ok(outcome)
    }
}

impl MapSubroutine {
    async fn propose_for(
        &self,
        focus: &Entity,
        neighbors: &[&Entity],
        _ctx: &SubroutineCtx,
    ) -> Result<Vec<ProposedEdge>, StewardError> {
        let system = "Given a focus entity and a set of neighboring entities from the same \
                      workstream knowledge base, propose typed relations that would belong \
                      between them but do not yet exist. Output ONLY a JSON array; each item: \
                      {\"from_id\": uuid, \"rel\": one of \
                      [relates_to, supports, contradicts, mentions, belongs_to], \
                      \"to_id\": uuid, \"reason\": short string}. Be conservative — empty \
                      array is fine. Do not propose `supersedes` or `extracted_from`.";
        let mut roster = serde_json::Map::new();
        roster.insert(
            "focus".to_string(),
            serde_json::to_value(brief(focus))?,
        );
        let neighbor_briefs: Vec<serde_json::Value> =
            neighbors.iter().map(|e| brief(e)).collect();
        roster.insert(
            "neighbors".to_string(),
            serde_json::Value::Array(neighbor_briefs),
        );
        let user = serde_json::to_string_pretty(&roster)?;
        let raw = complete_text(&self.client, &self.model, system, &user).await?;
        let json = extract_json_block(&raw)
            .ok_or_else(|| StewardError::Parse(format!("map: no JSON in LLM response: {raw}")))?;
        Ok(serde_json::from_str(json)?)
    }

    fn record_proposal(
        &self,
        focus: &Entity,
        prop: &ProposedEdge,
        ctx: &SubroutineCtx,
    ) -> Result<(), StewardError> {
        let rel = RelationType::from_str(prop.rel.to_lowercase().as_str())
            .ok_or_else(|| StewardError::Parse(format!("unknown rel: {}", prop.rel)))?;
        if !is_proposable(rel) {
            return Err(StewardError::Parse(format!(
                "rel `{}` not proposable by map",
                prop.rel
            )));
        }
        let from = uuid::Uuid::parse_str(&prop.from_id)
            .map_err(|e| StewardError::Parse(format!("from_id: {e}")))?;
        let to = uuid::Uuid::parse_str(&prop.to_id)
            .map_err(|e| StewardError::Parse(format!("to_id: {e}")))?;
        // Drop self-loops and proposals that don't reference the focus
        // (the LLM occasionally hallucinates ids not in the prompt).
        if from == to {
            return Err(StewardError::Parse("self-loop".into()));
        }
        if from != focus.id && to != focus.id {
            return Err(StewardError::Parse("proposal does not involve focus".into()));
        }

        let record = JournalRecord {
            subroutine: SUBROUTINE_NAME.into(),
            action: "propose_relation".into(),
            inputs_json: json!({
                "focus_id": focus.id,
            })
            .to_string(),
            outputs_json: json!({
                "from_id": from,
                "rel": rel.as_str(),
                "to_id": to,
                "reason": prop.reason,
            })
            .to_string(),
            model: self.model.clone(),
            prompt_hash: Journal::prompt_hash(format!(
                "map/{}/{}/{}",
                from, rel.as_str(), to
            )),
            applied: false,
        };
        ctx.journal.write_ahead(&record)?;
        debug!(
            workstream = %ctx.workstream.name,
            from = %from,
            rel = rel.as_str(),
            to = %to,
            "map: proposal recorded"
        );
        Ok(())
    }
}

fn brief(e: &Entity) -> serde_json::Value {
    json!({
        "id": e.id,
        "entity_type": e.entity_type.as_str(),
        "title": e.title,
        "tags": e.tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::pin::Pin;
    use std::sync::Mutex;

    use arawn_core::Workstream;
    use arawn_llm::{
        LlmError,
        types::{ChatChunk, ChatRequest},
    };
    use arawn_memory::{ConfidenceSource, EntityType, MemoryManager};
    use futures::stream;
    use serde_json::Value;

    struct ScriptedMock {
        responses: Mutex<VecDeque<Value>>,
    }
    impl ScriptedMock {
        fn new(resp: Vec<Value>) -> Self {
            Self {
                responses: Mutex::new(resp.into_iter().collect()),
            }
        }
    }
    #[async_trait]
    impl LlmClient for ScriptedMock {
        async fn stream(
            &self,
            _req: ChatRequest,
        ) -> Result<
            Pin<Box<dyn futures::Stream<Item = Result<ChatChunk, LlmError>> + Send>>,
            LlmError,
        > {
            let v = self.responses.lock().unwrap().pop_front().expect("no responses");
            let text = v.to_string();
            Ok(Box::pin(stream::iter(vec![
                Ok(ChatChunk::TextDelta { text }),
                Ok(ChatChunk::Done { usage: None }),
            ])))
        }
    }

    fn setup() -> (tempfile::TempDir, Arc<MemoryManager>, Arc<Journal>, Arc<
        dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync,
    >) {
        let tmp = tempfile::tempdir().unwrap();
        let mem = Arc::new(MemoryManager::open(tmp.path(), "ws", None).unwrap());
        let j = Arc::new(Journal::open(tmp.path(), "ws").unwrap());
        let dir = tmp.path().to_path_buf();
        let f: Arc<
            dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync,
        > = Arc::new(move |n: &str| CursorStore::open(&dir, n));
        (tmp, mem, j, f)
    }

    fn ctx(
        tmp: &tempfile::TempDir,
        mem: &Arc<MemoryManager>,
        j: &Arc<Journal>,
        cap: usize,
    ) -> SubroutineCtx {
        // Map is proposal-only.
        let gate = Arc::new(crate::journal::JournalGate::new(Arc::clone(j), false));
        SubroutineCtx {
            workstream: Workstream::new("ws", tmp.path().join("ws")),
            memory: Arc::clone(mem),
            journal: gate,
            cap,
        }
    }

    #[tokio::test]
    async fn proposes_valid_edges_and_drops_invalid() {
        let (tmp, mem, j, fac) = setup();
        let a = Entity::new(EntityType::Decision, "use postgres")
            .with_confidence(ConfidenceSource::Stated);
        let b = Entity::new(EntityType::Fact, "postgres supports jsonb");
        let c = Entity::new(EntityType::Fact, "team prefers typed schemas");
        mem.workstream.insert_entity(&a).unwrap();
        mem.workstream.insert_entity(&b).unwrap();
        mem.workstream.insert_entity(&c).unwrap();

        // Script: one valid + one self-loop + one bogus rel.
        let payload = json!([
            {"from_id": a.id, "rel": "supports", "to_id": b.id, "reason": "directly evidenced"},
            {"from_id": a.id, "rel": "relates_to", "to_id": a.id, "reason": "self"},
            {"from_id": a.id, "rel": "supersedes", "to_id": b.id, "reason": "wrong rel"}
        ]);
        // Three focus entities → three LLM calls; script empty arrays
        // for the other two.
        let mock = Arc::new(ScriptedMock::new(vec![
            payload,
            json!([]),
            json!([]),
        ]));
        let sub = MapSubroutine::new(mock as Arc<dyn LlmClient>, "mock", Arc::clone(&fac));
        let out = sub.run(&ctx(&tmp, &mem, &j, 10)).await.unwrap();
        // Only the first proposal is valid.
        assert_eq!(out.proposals_recorded, 1);
        let recent = j.recent(10).unwrap();
        assert_eq!(recent[0].subroutine, "map");
        assert!(!recent[0].applied);
    }

    #[tokio::test]
    async fn cap_stops_after_n_proposals() {
        let (tmp, mem, j, fac) = setup();
        let a = Entity::new(EntityType::Fact, "a");
        let b = Entity::new(EntityType::Fact, "b");
        let c = Entity::new(EntityType::Fact, "c");
        mem.workstream.insert_entity(&a).unwrap();
        mem.workstream.insert_entity(&b).unwrap();
        mem.workstream.insert_entity(&c).unwrap();

        let many = json!([
            {"from_id": a.id, "rel": "relates_to", "to_id": b.id, "reason": "x"},
            {"from_id": a.id, "rel": "relates_to", "to_id": c.id, "reason": "y"},
            {"from_id": a.id, "rel": "supports", "to_id": b.id, "reason": "z"},
        ]);
        let mock = Arc::new(ScriptedMock::new(vec![many, json!([]), json!([])]));
        let sub = MapSubroutine::new(mock as Arc<dyn LlmClient>, "mock", Arc::clone(&fac));
        let out = sub.run(&ctx(&tmp, &mem, &j, 2)).await.unwrap();
        assert!(out.cap_hit);
        assert_eq!(out.proposals_recorded, 2);
    }

    #[tokio::test]
    async fn cursor_advances_and_skips_on_rerun() {
        let (tmp, mem, j, fac) = setup();
        let a = Entity::new(EntityType::Fact, "a");
        mem.workstream.insert_entity(&a).unwrap();
        let mock = Arc::new(ScriptedMock::new(vec![json!([])]));
        let sub = MapSubroutine::new(mock as Arc<dyn LlmClient>, "mock", Arc::clone(&fac));
        let _ = sub.run(&ctx(&tmp, &mem, &j, 10)).await.unwrap();
        // Second pass — no new entities; no LLM call should happen.
        let out = sub.run(&ctx(&tmp, &mem, &j, 10)).await.unwrap();
        assert_eq!(out.actions_journaled, 0);
    }
}
