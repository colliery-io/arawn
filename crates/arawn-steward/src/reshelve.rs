//! Re-shelve subroutine — dedupe + supersede + erroneous-deletion.
//!
//! Per ARAWN-A-0003 the allowed verbs are:
//!   - mark superseded + add SUPERSEDES relation
//!   - combine content fields (copy non-empty fields from deprecated into survivor)
//!   - delete entity (only when judged *erroneous*, not merely duplicate)
//!
//! Survivor rule (T-0257 design): the entity with the higher
//! `reinforcement_count` wins; ties break on newer `created_at`. The
//! LLM proposes the action; Rust picks the survivor.

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

const SUBROUTINE_NAME: &str = "reshelve";

#[derive(Debug, Clone)]
pub struct ReshelveConfig {
    /// How many touched-since-cursor entities to scan per pass.
    pub batch_size: usize,
    /// How many FTS candidates to consider per focus entity.
    pub candidates_per_focus: usize,
}

impl Default for ReshelveConfig {
    fn default() -> Self {
        Self {
            batch_size: 20,
            candidates_per_focus: 3,
        }
    }
}

pub struct ReshelveSubroutine {
    client: Arc<dyn LlmClient>,
    model: String,
    config: ReshelveConfig,
    /// Resolves the workstream's cursor store. Returning a fresh handle
    /// per call is fine — `Connection::open` is cheap on the same path.
    cursor_factory: Arc<dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync>,
}

impl ReshelveSubroutine {
    pub fn new(
        client: Arc<dyn LlmClient>,
        model: impl Into<String>,
        cursor_factory: Arc<dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync>,
    ) -> Self {
        Self {
            client,
            model: model.into(),
            config: ReshelveConfig::default(),
            cursor_factory,
        }
    }

    pub fn with_config(mut self, config: ReshelveConfig) -> Self {
        self.config = config;
        self
    }
}

/// LLM verdict on a (focus, candidate) pair. The model is asked for
/// exactly one of these actions per pair.
#[derive(Debug, Deserialize)]
struct PairVerdict {
    /// One of: "none", "duplicate", "erroneous".
    /// - "none": leave both untouched.
    /// - "duplicate": merge — Rust picks the survivor, the LLM proposes
    ///   combined content fields.
    /// - "erroneous": delete the *focus* (the newer / just-touched one)
    ///   because it's wrong on its face.
    action: String,
    /// Free-form rationale recorded into the journal.
    #[serde(default)]
    reason: String,
    /// When action = "duplicate", an optional `combined_content` string
    /// the LLM thinks the survivor should carry going forward.
    #[serde(default)]
    combined_content: Option<String>,
    /// When action = "erroneous", which side of the pair to delete —
    /// `"focus"` (default) or `"candidate"`.
    #[serde(default)]
    delete_target: Option<String>,
}

#[async_trait]
impl StewardSubroutine for ReshelveSubroutine {
    fn name(&self) -> &str {
        SUBROUTINE_NAME
    }

    fn is_mutating(&self) -> bool {
        true
    }

    async fn run(&self, ctx: &SubroutineCtx) -> Result<SubroutineOutcome, StewardError> {
        let cursor_store = (self.cursor_factory)(&ctx.workstream.name)?;
        let cursor = cursor_store.get(SUBROUTINE_NAME)?;

        // Pull a candidate batch of recently-touched entities. We
        // list_all_ranked (fetches all non-superseded), filter by
        // updated_at > cursor in Rust, and sort ascending by
        // updated_at so we always advance the cursor monotonically.
        let mut all = ctx
            .memory
            .workstream
            .list_all_ranked(1_000)
            .map_err(StewardError::from)?;
        if let Some(c) = cursor {
            all.retain(|e| e.updated_at > c);
        }
        all.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
        all.truncate(self.config.batch_size);
        if all.is_empty() {
            return Ok(SubroutineOutcome::default());
        }

        let mut outcome = SubroutineOutcome::default();
        let mut latest_seen = cursor;
        for focus in &all {
            // A prior merge / delete in this same pass may have made
            // this entity stale — re-check current state in the KB.
            let current = ctx
                .memory
                .workstream
                .get_entity(focus.id)
                .map_err(StewardError::from)?;
            let Some(current) = current else {
                continue;
            };
            if current.superseded {
                continue;
            }
            let focus = &current;
            // Cap is on *applied actions*, not on entities considered.
            if outcome.mutations_applied >= ctx.cap {
                outcome.cap_hit = true;
                debug!(
                    workstream = %ctx.workstream.name,
                    cap = ctx.cap,
                    "reshelve cap hit; stopping pass"
                );
                break;
            }
            if let Err(e) = self
                .process_focus(focus, ctx, &mut outcome)
                .await
            {
                warn!(
                    workstream = %ctx.workstream.name,
                    focus_id = %focus.id,
                    error = %e,
                    "reshelve: per-entity error; continuing"
                );
            }
            if latest_seen.map(|p| focus.updated_at > p).unwrap_or(true) {
                latest_seen = Some(focus.updated_at);
            }
        }

        // Always advance the cursor past whatever we considered, even
        // if the LLM decided "none" for every pair. Otherwise we
        // re-scan the same entities forever.
        if let Some(ts) = latest_seen {
            cursor_store.advance(SUBROUTINE_NAME, ts)?;
        }

        Ok(outcome)
    }
}

impl ReshelveSubroutine {
    async fn process_focus(
        &self,
        focus: &Entity,
        ctx: &SubroutineCtx,
        outcome: &mut SubroutineOutcome,
    ) -> Result<(), StewardError> {
        // FTS-find similar entities by title. Skip self + superseded.
        let raw = ctx
            .memory
            .workstream
            .search(&fts_quote(&focus.title), self.config.candidates_per_focus * 2)
            .map_err(StewardError::from)?;
        let candidates: Vec<Entity> = raw
            .into_iter()
            .filter(|e| e.id != focus.id && !e.superseded)
            .take(self.config.candidates_per_focus)
            .collect();
        if candidates.is_empty() {
            return Ok(());
        }

        for cand in &candidates {
            if outcome.mutations_applied >= ctx.cap {
                outcome.cap_hit = true;
                break;
            }
            let verdict = match self.classify_pair(focus, cand).await {
                Ok(v) => v,
                Err(e) => {
                    warn!(
                        focus_id = %focus.id,
                        cand_id = %cand.id,
                        error = %e,
                        "reshelve: classify failed; skipping pair"
                    );
                    continue;
                }
            };
            match verdict.action.to_lowercase().as_str() {
                "duplicate" => {
                    self.apply_merge(focus, cand, &verdict, ctx, outcome)?;
                }
                "erroneous" => {
                    let target = match verdict.delete_target.as_deref() {
                        Some("candidate") => cand,
                        _ => focus,
                    };
                    let target_is_focus = target.id == focus.id;
                    self.apply_delete(target, &verdict, ctx, outcome)?;
                    if target_is_focus {
                        // No point comparing further candidates: focus is gone.
                        return Ok(());
                    }
                }
                _ => {
                    debug!(
                        focus_id = %focus.id,
                        cand_id = %cand.id,
                        action = %verdict.action,
                        "reshelve: no merge"
                    );
                }
            }
        }
        Ok(())
    }

    async fn classify_pair(
        &self,
        focus: &Entity,
        cand: &Entity,
    ) -> Result<PairVerdict, StewardError> {
        let system = "You are deciding whether two knowledge-base entries refer to the same \
                      thing or whether one of them is erroneous. Return ONLY a JSON object: \
                      {\"action\": \"none\" | \"duplicate\" | \"erroneous\", \
                       \"reason\": short string, \
                       \"combined_content\": optional string when action=duplicate, \
                       \"delete_target\": \"focus\" | \"candidate\" — required when action=erroneous}. \
                      Be conservative — when in doubt, action=\"none\".";
        let user = format!(
            "Entity A (the just-touched one, focus):\n\
             id: {fid}\nentity_type: {fty}\ntitle: {ftitle}\ncontent: {fcontent}\n\
             tags: {ftags}\n\n\
             Entity B (candidate):\n\
             id: {bid}\nentity_type: {bty}\ntitle: {btitle}\ncontent: {bcontent}\n\
             tags: {btags}\n",
            fid = focus.id,
            fty = focus.entity_type.as_str(),
            ftitle = focus.title,
            fcontent = focus.content.as_deref().unwrap_or("(none)"),
            ftags = focus.tags.join(", "),
            bid = cand.id,
            bty = cand.entity_type.as_str(),
            btitle = cand.title,
            bcontent = cand.content.as_deref().unwrap_or("(none)"),
            btags = cand.tags.join(", "),
        );
        let raw = complete_text(&self.client, &self.model, system, &user).await?;
        let json = extract_json_block(&raw).ok_or_else(|| StewardError::Parse(format!(
            "reshelve: no JSON in LLM response: {raw}"
        )))?;
        serde_json::from_str(json).map_err(|e| StewardError::Parse(format!("reshelve verdict: {e}")))
    }

    fn apply_merge(
        &self,
        focus: &Entity,
        cand: &Entity,
        verdict: &PairVerdict,
        ctx: &SubroutineCtx,
        outcome: &mut SubroutineOutcome,
    ) -> Result<(), StewardError> {
        // Most-reinforced survives; ties → newer created_at.
        let (survivor, deprecated) =
            if (cand.reinforcement_count, cand.created_at) >= (focus.reinforcement_count, focus.created_at) {
                (cand, focus)
            } else {
                (focus, cand)
            };

        // Build the post-merge survivor entity:
        //  - tag union
        //  - content = LLM-proposed combined_content if present, else
        //    survivor's existing content
        //  - reinforcement_count summed (capture that "we saw this twice")
        let mut merged = survivor.clone();
        for t in &deprecated.tags {
            if !merged.tags.contains(t) {
                merged.tags.push(t.clone());
            }
        }
        if let Some(ref c) = verdict.combined_content
            && !c.trim().is_empty()
        {
            merged.content = Some(c.clone());
        } else if merged.content.is_none() {
            merged.content = deprecated.content.clone();
        }
        merged.reinforcement_count = survivor
            .reinforcement_count
            .saturating_add(deprecated.reinforcement_count)
            .saturating_add(1);
        merged.updated_at = chrono::Utc::now();

        // Mark deprecated superseded.
        let mut dep_after = deprecated.clone();
        dep_after.superseded = true;
        dep_after.updated_at = chrono::Utc::now();

        // Write-ahead journal with everything needed for revert.
        let payload = json!({
            "action": "merge",
            "survivor_id": survivor.id,
            "deprecated_id": deprecated.id,
            "pre_survivor": survivor,
            "pre_deprecated": deprecated,
            "post_survivor": &merged,
            "llm_reason": verdict.reason,
        });
        let record = JournalRecord {
            subroutine: SUBROUTINE_NAME.into(),
            action: "merge".into(),
            inputs_json: json!({
                "focus_id": focus.id,
                "candidate_id": cand.id,
            })
            .to_string(),
            outputs_json: payload.to_string(),
            model: self.model.clone(),
            prompt_hash: Journal::prompt_hash(format!(
                "reshelve/{}/{}",
                focus.id, cand.id
            )),
            applied: true,
        };
        ctx.journal.write_ahead(&record)?;

        // Apply the mutation:
        //  1. update survivor with merged content/tags.
        //  2. update deprecated with superseded=true.
        //  3. add Supersedes relation: survivor -> deprecated.
        ctx.memory
            .workstream
            .update_entity(&merged)
            .map_err(StewardError::from)?;
        ctx.memory
            .workstream
            .update_entity(&dep_after)
            .map_err(StewardError::from)?;
        ctx.memory
            .workstream
            .add_relation(survivor.id, RelationType::Supersedes, deprecated.id)
            .map_err(StewardError::from)?;

        outcome.actions_journaled += 1;
        outcome.mutations_applied += 1;
        debug!(
            survivor = %survivor.id,
            deprecated = %deprecated.id,
            "reshelve: merged"
        );
        Ok(())
    }

    fn apply_delete(
        &self,
        focus: &Entity,
        verdict: &PairVerdict,
        ctx: &SubroutineCtx,
        outcome: &mut SubroutineOutcome,
    ) -> Result<(), StewardError> {
        let payload = json!({
            "action": "delete",
            "entity": focus,
            "llm_reason": verdict.reason,
        });
        let record = JournalRecord {
            subroutine: SUBROUTINE_NAME.into(),
            action: "delete".into(),
            inputs_json: json!({"focus_id": focus.id}).to_string(),
            outputs_json: payload.to_string(),
            model: self.model.clone(),
            prompt_hash: Journal::prompt_hash(format!("reshelve/delete/{}", focus.id)),
            applied: true,
        };
        ctx.journal.write_ahead(&record)?;
        ctx.memory
            .workstream
            .delete_entity(focus.id)
            .map_err(StewardError::from)?;
        outcome.actions_journaled += 1;
        outcome.mutations_applied += 1;
        debug!(focus = %focus.id, "reshelve: deleted erroneous entity");
        Ok(())
    }
}

/// FTS5 phrase-quote helper. Wraps the term in double quotes and
/// escapes embedded quotes so titles like `O'Brien's "memo"` don't
/// break the parser.
fn fts_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
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

    use crate::subroutine::SubroutineCtx;

    /// Queue-based mock that returns scripted JSON for each call.
    struct ScriptedMock {
        responses: Mutex<VecDeque<Value>>,
    }

    impl ScriptedMock {
        fn new(responses: Vec<Value>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().collect()),
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
            let v = self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .expect("ScriptedMock: no more responses scripted");
            let text = v.to_string();
            let items: Vec<Result<ChatChunk, LlmError>> = vec![
                Ok(ChatChunk::TextDelta { text }),
                Ok(ChatChunk::Done { usage: None }),
            ];
            Ok(Box::pin(stream::iter(items)))
        }
    }

    struct Fixture {
        tmp: tempfile::TempDir,
        memory: Arc<MemoryManager>,
        journal: Arc<Journal>,
        cursor_factory: Arc<
            dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync,
        >,
    }

    fn setup() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let mem =
            Arc::new(MemoryManager::open(tmp.path(), "ws-pat", None).unwrap());
        let j = Arc::new(Journal::open(tmp.path(), "ws-pat").unwrap());
        let dir = tmp.path().to_path_buf();
        let cursor_factory: Arc<
            dyn Fn(&str) -> Result<CursorStore, StewardError> + Send + Sync,
        > = Arc::new(move |name: &str| CursorStore::open(&dir, name));
        Fixture {
            tmp,
            memory: mem,
            journal: j,
            cursor_factory,
        }
    }

    fn ctx(fx: &Fixture, cap: usize) -> SubroutineCtx {
        // Reshelve is a mutating subroutine — gate lets through
        // applied=true journal writes.
        let gate = Arc::new(crate::journal::JournalGate::new(
            Arc::clone(&fx.journal),
            true,
        ));
        SubroutineCtx {
            workstream: Workstream::new("ws-pat", fx.tmp.path().join("ws-pat")),
            memory: Arc::clone(&fx.memory),
            journal: gate,
            cap,
        }
    }

    fn fact(title: &str, content: &str, reinforce: u32) -> Entity {
        let mut e = Entity::new(EntityType::Fact, title)
            .with_content(content)
            .with_confidence(ConfidenceSource::Stated);
        e.reinforcement_count = reinforce;
        e
    }

    #[tokio::test]
    async fn merge_picks_most_reinforced_survivor() {
        let fx = setup();
        // Older, heavily-reinforced entity = survivor
        let mut older = fact("postgres is the db", "decision made in Q1", 5);
        older.created_at = chrono::Utc::now() - chrono::Duration::days(30);
        older.updated_at = older.created_at;
        // Newer, weakly-reinforced entity = focus (just touched)
        let newer = fact("postgres is the db", "we use postgres", 0);

        fx.memory.workstream.insert_entity(&older).unwrap();
        fx.memory.workstream.insert_entity(&newer).unwrap();

        let mock = Arc::new(ScriptedMock::new(vec![json!({
            "action": "duplicate",
            "reason": "same decision",
            "combined_content": "postgres chosen Q1; reaffirmed."
        })]));
        let sub = ReshelveSubroutine::new(
            mock as Arc<dyn LlmClient>,
            "mock",
            Arc::clone(&fx.cursor_factory),
        )
        .with_config(ReshelveConfig {
            batch_size: 20,
            candidates_per_focus: 3,
        });

        let out = sub.run(&ctx(&fx, 10)).await.unwrap();
        assert_eq!(out.mutations_applied, 1);
        assert_eq!(out.actions_journaled, 1);

        // Older should still be active; newer should be superseded.
        let survivor = fx.memory.workstream.get_entity(older.id).unwrap().unwrap();
        let deprecated = fx.memory.workstream.get_entity(newer.id).unwrap().unwrap();
        assert!(!survivor.superseded, "older (reinforced) must survive");
        assert!(deprecated.superseded, "newer must be marked superseded");
        assert_eq!(
            survivor.content.as_deref().unwrap(),
            "postgres chosen Q1; reaffirmed.",
            "LLM-proposed combined_content should be installed on the survivor"
        );
        assert!(survivor.reinforcement_count >= 6);
    }

    #[tokio::test]
    async fn erroneous_deletes_focus() {
        let fx = setup();
        let trustworthy = fact("alice is on parental leave", "until june", 2);
        let bogus = fact("alice is on parental leave", "bogus claim", 0);
        fx.memory.workstream.insert_entity(&trustworthy).unwrap();
        fx.memory.workstream.insert_entity(&bogus).unwrap();

        let mock = Arc::new(ScriptedMock::new(vec![json!({
            "action": "erroneous",
            "reason": "contradicts stated fact",
            "delete_target": "candidate"
        })]));
        let sub = ReshelveSubroutine::new(
            mock as Arc<dyn LlmClient>,
            "mock",
            Arc::clone(&fx.cursor_factory),
        );

        // Focus = whichever was touched last; insert order makes `bogus`
        // newer so it's the focus.
        let out = sub.run(&ctx(&fx, 10)).await.unwrap();
        assert!(out.mutations_applied >= 1);
        // bogus must be gone
        assert!(fx.memory.workstream.get_entity(bogus.id).unwrap().is_none());
        // trustworthy still there
        assert!(fx.memory.workstream.get_entity(trustworthy.id).unwrap().is_some());
    }

    #[tokio::test]
    async fn none_verdict_leaves_kb_untouched_but_advances_cursor() {
        let fx = setup();
        let a = fact("rust async runtime", "tokio", 0);
        let b = fact("rust ownership model", "borrow checker", 0);
        fx.memory.workstream.insert_entity(&a).unwrap();
        fx.memory.workstream.insert_entity(&b).unwrap();

        // Two candidates (each entity is the focus once, candidate of
        // the other), so worst case we need 2 LLM calls — script both.
        let mock = Arc::new(ScriptedMock::new(vec![
            json!({"action": "none", "reason": "different topics"}),
            json!({"action": "none", "reason": "different topics"}),
        ]));
        let sub = ReshelveSubroutine::new(
            mock as Arc<dyn LlmClient>,
            "mock",
            Arc::clone(&fx.cursor_factory),
        );
        let out = sub.run(&ctx(&fx, 10)).await.unwrap();
        assert_eq!(out.mutations_applied, 0);

        // Cursor must have advanced even with zero mutations.
        let cs = (fx.cursor_factory)("ws-pat").unwrap();
        assert!(cs.get(SUBROUTINE_NAME).unwrap().is_some());
    }

    #[tokio::test]
    async fn second_pass_skips_already_processed_entities() {
        let fx = setup();
        let a = fact("postgres is the db", "v1", 0);
        let b = fact("postgres is the db", "v2", 0);
        fx.memory.workstream.insert_entity(&a).unwrap();
        fx.memory.workstream.insert_entity(&b).unwrap();

        let mock = Arc::new(ScriptedMock::new(vec![
            json!({"action": "none", "reason": "first pass"}),
            json!({"action": "none", "reason": "first pass"}),
        ]));
        let sub = ReshelveSubroutine::new(
            mock as Arc<dyn LlmClient>,
            "mock",
            Arc::clone(&fx.cursor_factory),
        );
        let _ = sub.run(&ctx(&fx, 10)).await.unwrap();
        // Second pass should not call the LLM at all (no scripted
        // responses remain) — if reshelve re-scanned the same
        // entities it would panic on the empty queue.
        let out = sub.run(&ctx(&fx, 10)).await.unwrap();
        assert_eq!(out.actions_journaled, 0);
    }

    #[tokio::test]
    async fn cap_stops_after_n_applied() {
        let fx = setup();
        // Make three duplicate pairs.
        for i in 0..6 {
            let mut e = fact("identical title", &format!("c{i}"), 0);
            // stagger created_at so most-reinforced tie-break is stable
            e.created_at = chrono::Utc::now()
                - chrono::Duration::seconds(60 - i as i64);
            e.updated_at = e.created_at;
            fx.memory.workstream.insert_entity(&e).unwrap();
        }
        // Script enough "duplicate" verdicts that cap is the only thing
        // stopping us.
        let mut responses = Vec::new();
        for _ in 0..20 {
            responses.push(json!({"action": "duplicate", "reason": "ok"}));
        }
        let mock = Arc::new(ScriptedMock::new(responses));
        let sub = ReshelveSubroutine::new(
            mock as Arc<dyn LlmClient>,
            "mock",
            Arc::clone(&fx.cursor_factory),
        );
        let out = sub.run(&ctx(&fx, 2)).await.unwrap();
        assert!(out.cap_hit);
        assert_eq!(out.mutations_applied, 2);
    }
}
