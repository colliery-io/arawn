//! Door-watch subroutine — proposal-only cross-workstream identity
//! candidates. For each focus entity in the current workstream
//! (touched since cursor), look at a sample of entities in *every
//! other* active workstream and ask the LLM "is any of these the same
//! thing as the focus?".
//!
//! Scans *all* entity types (per user direction). Per ARAWN-A-0003
//! proposals are journaled in the source workstream and never mutate
//! either side.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tracing::{debug, warn};

use arawn_core::Workstream;
use arawn_llm::LlmClient;
use arawn_memory::Entity;
use arawn_storage::Store;

use crate::cursor::CursorStore;
use crate::error::StewardError;
use crate::journal::{Journal, JournalRecord};
use crate::llm_text::{complete_text, extract_json_block};
use crate::runner::MemoryResolver;
use crate::subroutine::{StewardSubroutine, SubroutineCtx, SubroutineOutcome};

const SUBROUTINE_NAME: &str = "doorwatch";

#[derive(Debug, Clone)]
pub struct DoorWatchConfig {
    /// Recent entities to consider as focus per pass.
    pub focus_batch: usize,
    /// Per-other-workstream sample size of candidates to compare against.
    pub neighbors_per_workstream: usize,
}

impl Default for DoorWatchConfig {
    fn default() -> Self {
        Self {
            focus_batch: 5,
            neighbors_per_workstream: 10,
        }
    }
}

pub struct DoorWatchSubroutine {
    client: Arc<dyn LlmClient>,
    model: String,
    config: DoorWatchConfig,
    cursor_factory: Arc<dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync>,
    store: Arc<Mutex<Store>>,
    memory_resolver: MemoryResolver,
}

impl DoorWatchSubroutine {
    pub fn new(
        client: Arc<dyn LlmClient>,
        model: impl Into<String>,
        cursor_factory: Arc<dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync>,
        store: Arc<Mutex<Store>>,
        memory_resolver: MemoryResolver,
    ) -> Self {
        Self {
            client,
            model: model.into(),
            config: DoorWatchConfig::default(),
            cursor_factory,
            store,
            memory_resolver,
        }
    }

    pub fn with_config(mut self, config: DoorWatchConfig) -> Self {
        self.config = config;
        self
    }
}

#[derive(Debug, Deserialize)]
struct IdentityMatch {
    to_workstream: String,
    to_id: String,
    #[serde(default)]
    reason: String,
}

#[async_trait]
impl StewardSubroutine for DoorWatchSubroutine {
    fn name(&self) -> &str {
        SUBROUTINE_NAME
    }

    fn is_mutating(&self) -> bool {
        false
    }

    async fn run(&self, ctx: &SubroutineCtx) -> Result<SubroutineOutcome, StewardError> {
        let cursor_store = (self.cursor_factory)(&ctx.workstream.name)?;
        let cursor = cursor_store.get(SUBROUTINE_NAME)?;

        // Focus = entities in *this* workstream touched since cursor.
        let mut focus_set: Vec<Entity> = ctx
            .memory
            .workstream
            .list_all_ranked(500)
            .map_err(StewardError::from)?
            .into_iter()
            .filter(|e| cursor.is_none_or(|c| e.updated_at > c))
            .collect();
        focus_set.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
        focus_set.truncate(self.config.focus_batch);
        if focus_set.is_empty() {
            return Ok(SubroutineOutcome::default());
        }

        // Other active workstreams.
        let others: Vec<Workstream> = {
            let s = self.store.lock().unwrap();
            s.list_workstreams()
                .map_err(|e| StewardError::Storage(e.to_string()))?
                .into_iter()
                .filter(|w| w.name != ctx.workstream.name)
                .collect()
        };
        if others.is_empty() {
            // Cursor still advances so we don't keep scanning the same
            // focus set if a second workstream is later added.
            if let Some(latest) = focus_set.last() {
                cursor_store.advance(SUBROUTINE_NAME, latest.updated_at)?;
            }
            return Ok(SubroutineOutcome::default());
        }

        // Build the comparison roster: one bundle per other workstream
        // with its top-N most-active entities.
        let mut other_buckets: Vec<(String, Vec<Entity>)> = Vec::new();
        for w in &others {
            match (self.memory_resolver)(&w.name) {
                Ok(mgr) => {
                    let xs = mgr
                        .workstream
                        .list_all_ranked(self.config.neighbors_per_workstream)
                        .unwrap_or_default();
                    if !xs.is_empty() {
                        other_buckets.push((w.name.clone(), xs));
                    }
                }
                Err(e) => warn!(
                    workstream = %w.name,
                    error = %e,
                    "doorwatch: failed to open neighbor KB; skipping"
                ),
            }
        }
        if other_buckets.is_empty() {
            if let Some(latest) = focus_set.last() {
                cursor_store.advance(SUBROUTINE_NAME, latest.updated_at)?;
            }
            return Ok(SubroutineOutcome::default());
        }

        let mut outcome = SubroutineOutcome::default();
        let mut latest_seen = cursor;
        for focus in &focus_set {
            if outcome.proposals_recorded >= ctx.cap {
                outcome.cap_hit = true;
                break;
            }
            let matches = match self.classify(focus, &other_buckets).await {
                Ok(m) => m,
                Err(e) => {
                    warn!(
                        workstream = %ctx.workstream.name,
                        focus = %focus.id,
                        error = %e,
                        "doorwatch: LLM call failed; skipping focus"
                    );
                    continue;
                }
            };
            for m in matches {
                if outcome.proposals_recorded >= ctx.cap {
                    outcome.cap_hit = true;
                    break;
                }
                if let Err(e) = self.record(focus, &m, ctx, &other_buckets) {
                    warn!(
                        workstream = %ctx.workstream.name,
                        focus = %focus.id,
                        error = %e,
                        "doorwatch: dropped proposal"
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

impl DoorWatchSubroutine {
    async fn classify(
        &self,
        focus: &Entity,
        buckets: &[(String, Vec<Entity>)],
    ) -> Result<Vec<IdentityMatch>, StewardError> {
        let system = "You are looking for cross-workstream identity matches. Given a focus \
                      entity from one workstream and a set of entities from other workstreams, \
                      identify any candidates that refer to the *same underlying thing* \
                      (same person, same project, same external object). Output ONLY a JSON \
                      array; each item: {\"to_workstream\": string, \"to_id\": uuid, \
                      \"reason\": short string}. Be conservative — coincidental similar names \
                      are NOT matches. Empty array is fine.";
        let buckets_json: Vec<serde_json::Value> = buckets
            .iter()
            .map(|(ws, ents)| {
                json!({
                    "workstream": ws,
                    "entities": ents
                        .iter()
                        .map(brief)
                        .collect::<Vec<_>>(),
                })
            })
            .collect();
        let user = serde_json::to_string_pretty(&json!({
            "focus_workstream": "(this)",
            "focus": brief(focus),
            "buckets": buckets_json,
        }))?;
        let raw = complete_text(&self.client, &self.model, system, &user).await?;
        let json = extract_json_block(&raw)
            .ok_or_else(|| StewardError::Parse(format!("doorwatch: no JSON in: {raw}")))?;
        Ok(serde_json::from_str(json)?)
    }

    fn record(
        &self,
        focus: &Entity,
        m: &IdentityMatch,
        ctx: &SubroutineCtx,
        buckets: &[(String, Vec<Entity>)],
    ) -> Result<(), StewardError> {
        let to_id = uuid::Uuid::parse_str(&m.to_id)
            .map_err(|e| StewardError::Parse(format!("to_id: {e}")))?;
        // Verify the target id actually appeared in the workstream the
        // LLM cited — guards against hallucinated ids.
        let valid = buckets
            .iter()
            .find(|(ws, _)| ws == &m.to_workstream)
            .map(|(_, ents)| ents.iter().any(|e| e.id == to_id))
            .unwrap_or(false);
        if !valid {
            return Err(StewardError::Parse(format!(
                "match references unknown ({}, {})",
                m.to_workstream, m.to_id
            )));
        }
        let record = JournalRecord {
            subroutine: SUBROUTINE_NAME.into(),
            action: "propose_identity".into(),
            inputs_json: json!({
                "focus_id": focus.id,
            })
            .to_string(),
            outputs_json: json!({
                "from_workstream": ctx.workstream.name,
                "from_id": focus.id,
                "to_workstream": m.to_workstream,
                "to_id": to_id,
                "reason": m.reason,
            })
            .to_string(),
            model: self.model.clone(),
            prompt_hash: Journal::prompt_hash(format!(
                "doorwatch/{}/{}/{}",
                focus.id, m.to_workstream, to_id
            )),
            applied: false,
        };
        ctx.journal.write_ahead(&record)?;
        debug!(
            from = %focus.id,
            to_ws = %m.to_workstream,
            to_id = %to_id,
            "doorwatch: identity proposal recorded"
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

    use arawn_llm::{
        LlmError,
        types::{ChatChunk, ChatRequest},
    };
    use arawn_memory::{EntityType, MemoryManager};
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
            Ok(Box::pin(stream::iter(vec![
                Ok(ChatChunk::TextDelta { text: v.to_string() }),
                Ok(ChatChunk::Done { usage: None }),
            ])))
        }
    }

    fn setup_multi_workstream() -> (
        tempfile::TempDir,
        Arc<Mutex<Store>>,
        MemoryResolver,
        Arc<dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync>,
    ) {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        store.ensure_scratch_workstream().unwrap();
        store
            .create_workstream(&Workstream::new("pat", tmp.path().join("pat")))
            .unwrap();
        store
            .create_workstream(&Workstream::new("dnd", tmp.path().join("dnd")))
            .unwrap();
        let store = Arc::new(Mutex::new(store));
        let dir = tmp.path().to_path_buf();
        let resolver: MemoryResolver = Arc::new(move |name: &str| {
            MemoryManager::for_workstream(&dir, name, None)
                .map(Arc::new)
                .map_err(|e| StewardError::Memory(e.to_string()))
        });
        let dir2 = tmp.path().to_path_buf();
        let cf: Arc<dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync> =
            Arc::new(move |n: &str| CursorStore::open(&dir2, n));
        (tmp, store, resolver, cf)
    }

    #[tokio::test]
    async fn proposes_identity_when_match_found() {
        let (tmp, store, resolver, cf) = setup_multi_workstream();
        // Seed pat with `alice`; seed dnd with `alice the bard` (same person).
        let pat = (resolver)("pat").unwrap();
        let alice_pat = Entity::new(EntityType::Person, "alice cooper");
        pat.workstream.insert_entity(&alice_pat).unwrap();
        let dnd = (resolver)("dnd").unwrap();
        let alice_dnd = Entity::new(EntityType::Person, "alice the bard");
        dnd.workstream.insert_entity(&alice_dnd).unwrap();

        let mock = Arc::new(ScriptedMock::new(vec![json!([
            {"to_workstream": "dnd", "to_id": alice_dnd.id, "reason": "same person"}
        ])]));
        let sub = DoorWatchSubroutine::new(
            mock as Arc<dyn LlmClient>,
            "mock",
            Arc::clone(&cf),
            Arc::clone(&store),
            Arc::clone(&resolver),
        );
        let journal = Arc::new(Journal::open(tmp.path(), "pat").unwrap());
        let ctx = SubroutineCtx {
            workstream: Workstream::new("pat", tmp.path().join("pat")),
            memory: Arc::clone(&pat),
            journal: Arc::clone(&journal),
            cap: 10,
        };
        let out = sub.run(&ctx).await.unwrap();
        assert_eq!(out.proposals_recorded, 1);
        let recent = journal.recent(10).unwrap();
        assert_eq!(recent[0].action, "propose_identity");
        assert!(!recent[0].applied);
    }

    #[tokio::test]
    async fn hallucinated_target_id_is_dropped() {
        let (tmp, store, resolver, cf) = setup_multi_workstream();
        let pat = (resolver)("pat").unwrap();
        let alice_pat = Entity::new(EntityType::Person, "alice cooper");
        pat.workstream.insert_entity(&alice_pat).unwrap();
        let dnd = (resolver)("dnd").unwrap();
        let alice_dnd = Entity::new(EntityType::Person, "alice the bard");
        dnd.workstream.insert_entity(&alice_dnd).unwrap();

        // LLM points at a uuid that doesn't exist in the dnd bucket.
        let fake = uuid::Uuid::new_v4();
        let mock = Arc::new(ScriptedMock::new(vec![json!([
            {"to_workstream": "dnd", "to_id": fake, "reason": "wrong id"}
        ])]));
        let sub = DoorWatchSubroutine::new(
            mock as Arc<dyn LlmClient>,
            "mock",
            Arc::clone(&cf),
            Arc::clone(&store),
            Arc::clone(&resolver),
        );
        let journal = Arc::new(Journal::open(tmp.path(), "pat").unwrap());
        let ctx = SubroutineCtx {
            workstream: Workstream::new("pat", tmp.path().join("pat")),
            memory: Arc::clone(&pat),
            journal: Arc::clone(&journal),
            cap: 10,
        };
        let out = sub.run(&ctx).await.unwrap();
        assert_eq!(out.proposals_recorded, 0);
    }

    #[tokio::test]
    async fn no_other_workstreams_means_zero_proposals() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        store.ensure_scratch_workstream().unwrap();
        let store = Arc::new(Mutex::new(store));
        let dir = tmp.path().to_path_buf();
        let resolver: MemoryResolver = Arc::new(move |name: &str| {
            MemoryManager::for_workstream(&dir, name, None)
                .map(Arc::new)
                .map_err(|e| StewardError::Memory(e.to_string()))
        });
        let dir2 = tmp.path().to_path_buf();
        let cf: Arc<dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync> =
            Arc::new(move |n: &str| CursorStore::open(&dir2, n));

        let mem = (resolver)("scratch").unwrap();
        mem.workstream
            .insert_entity(&Entity::new(EntityType::Fact, "lonely"))
            .unwrap();

        // No LLM call expected — script empty so a stray call would panic.
        let mock = Arc::new(ScriptedMock::new(vec![]));
        let sub = DoorWatchSubroutine::new(
            mock as Arc<dyn LlmClient>,
            "mock",
            Arc::clone(&cf),
            Arc::clone(&store),
            Arc::clone(&resolver),
        );
        let journal = Arc::new(Journal::open(tmp.path(), "scratch").unwrap());
        let ctx = SubroutineCtx {
            workstream: Workstream::new("scratch", tmp.path().join("scratch")),
            memory: Arc::clone(&mem),
            journal: Arc::clone(&journal),
            cap: 10,
        };
        let out = sub.run(&ctx).await.unwrap();
        assert_eq!(out.proposals_recorded, 0);
    }
}
