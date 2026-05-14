//! Tag-promoter steward subroutine — Suggest stage of the
//! Extract→Suggest→Add cycle from ADR-0004.
//!
//! Walks the active entities in a workstream's KB, counts
//! `tags_discovered` frequencies, and proposes promotion when a
//! discovered tag crosses a threshold AND is not already in the
//! ontology AND doesn't already have a pending proposal.
//!
//! Proposal-only — never mutates the KB or the ontology table.
//! Acceptance lives in `accept::apply_forward` for
//! `(tag-promoter, promote_tag)` (T-0266).

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::json;
use tracing::{debug, warn};
use uuid::Uuid;

use arawn_memory::TagOntologyStore;

use crate::error::StewardError;
use crate::journal::{Journal, JournalRecord};
use crate::subroutine::{StewardSubroutine, SubroutineCtx, SubroutineOutcome};

pub const SUBROUTINE_NAME: &str = "tag-promoter";
pub const ACTION_NAME: &str = "promote_tag";

#[derive(Debug, Clone)]
pub struct TagPromoterConfig {
    /// Minimum number of entities a discovered tag must appear on
    /// before the subroutine proposes promotion. Default 3 — same
    /// "appeared in three independent contexts" intuition as dust's
    /// `min_cluster_size`.
    pub min_count: usize,
    /// How many sample entity ids to attach to each proposal payload.
    /// Lets the reviewer see *which* entities motivated the promotion.
    pub sample_entities: usize,
    /// How many entities to scan per pass. Soft cap on the work the
    /// subroutine does — even at 2000 the frequency tally is cheap,
    /// but bounding it keeps cost predictable on large KBs.
    pub max_entities_scanned: usize,
}

impl Default for TagPromoterConfig {
    fn default() -> Self {
        Self {
            min_count: 3,
            sample_entities: 5,
            max_entities_scanned: 2_000,
        }
    }
}

pub struct TagPromoterSubroutine {
    config: TagPromoterConfig,
}

impl Default for TagPromoterSubroutine {
    fn default() -> Self {
        Self::new(TagPromoterConfig::default())
    }
}

impl TagPromoterSubroutine {
    pub fn new(config: TagPromoterConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl StewardSubroutine for TagPromoterSubroutine {
    fn name(&self) -> &str {
        SUBROUTINE_NAME
    }

    fn is_mutating(&self) -> bool {
        false
    }

    async fn run(&self, ctx: &SubroutineCtx) -> Result<SubroutineOutcome, StewardError> {
        // Tally `tags_discovered` frequencies and record up to
        // `sample_entities` entity ids per tag for the proposal payload.
        struct TagStats {
            count: usize,
            samples: Vec<Uuid>,
        }
        let entities = ctx
            .memory
            .workstream
            .list_all_ranked(self.config.max_entities_scanned)
            .map_err(StewardError::from)?;
        let mut tally: HashMap<String, TagStats> = HashMap::new();
        for e in &entities {
            for raw in &e.tags {
                let tag = raw.trim().to_lowercase();
                if tag.is_empty() {
                    continue;
                }
                // Skip steward-internal markers (e.g. `steward:dust`)
                // — they're operational metadata, not vocabulary.
                if tag.starts_with("steward:") {
                    continue;
                }
                let stats = tally.entry(tag).or_insert(TagStats {
                    count: 0,
                    samples: Vec::new(),
                });
                stats.count += 1;
                if stats.samples.len() < self.config.sample_entities {
                    stats.samples.push(e.id);
                }
            }
        }

        // Filter out tags that are already in the ontology — promoting
        // again is meaningless.
        let ontology = TagOntologyStore::open_at(&ctx.workstream.root_dir)
            .map_err(StewardError::from)?;
        tally.retain(|tag, _| !ontology.contains(tag).unwrap_or(false));

        // Filter out tags with a pending tag-promoter proposal still
        // open (applied=false, reverted_at=null). Rerunning the
        // subroutine shouldn't accumulate duplicate proposals.
        let pending = ctx
            .journal
            .inner_journal()
            .pending_proposals(500)
            .unwrap_or_default();
        for row in &pending {
            if row.subroutine == SUBROUTINE_NAME && row.action == ACTION_NAME
                && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&row.outputs_json)
                    && let Some(t) = parsed.get("tag").and_then(|v| v.as_str()) {
                        tally.remove(t);
                    }
        }

        // Threshold + cap. Sort by (count desc, tag alpha) so the
        // strongest candidates surface first when the cap bites.
        let mut candidates: Vec<(String, TagStats)> = tally
            .into_iter()
            .filter(|(_, s)| s.count >= self.config.min_count)
            .collect();
        candidates.sort_by(|(ta, sa), (tb, sb)| {
            sb.count.cmp(&sa.count).then(ta.cmp(tb))
        });

        let mut outcome = SubroutineOutcome::default();
        for (tag, stats) in candidates {
            if outcome.proposals_recorded >= ctx.cap {
                outcome.cap_hit = true;
                break;
            }
            if let Err(e) = record_proposal(ctx, &tag, &stats.samples, stats.count) {
                warn!(workstream = %ctx.workstream.name, tag = %tag, error = %e, "tag-promoter: dropped proposal");
                continue;
            }
            outcome.proposals_recorded += 1;
            outcome.actions_journaled += 1;
            debug!(
                workstream = %ctx.workstream.name,
                tag = %tag,
                count = stats.count,
                "tag-promoter: proposal recorded"
            );
        }
        Ok(outcome)
    }
}

fn record_proposal(
    ctx: &SubroutineCtx,
    tag: &str,
    samples: &[Uuid],
    count: usize,
) -> Result<i64, StewardError> {
    let record = JournalRecord {
        subroutine: SUBROUTINE_NAME.into(),
        action: ACTION_NAME.into(),
        inputs_json: json!({"tag": tag, "count": count}).to_string(),
        outputs_json: json!({
            "tag": tag,
            "count": count,
            "sample_entity_ids": samples,
        })
        .to_string(),
        model: "n/a".into(),
        prompt_hash: Journal::prompt_hash(format!("tag-promoter/{tag}")),
        applied: false,
    };
    ctx.journal.write_ahead(&record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use arawn_core::Workstream;
    use arawn_memory::{AddedVia, Entity, EntityType, MemoryManager};

    use crate::journal::{Journal, JournalGate};
    use crate::subroutine::SubroutineCtx;

    fn setup() -> (
        tempfile::TempDir,
        Arc<MemoryManager>,
        Arc<JournalGate>,
        Workstream,
    ) {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = Arc::new(MemoryManager::open(tmp.path(), "ws", None).unwrap());
        let journal = Arc::new(Journal::open(tmp.path(), "ws").unwrap());
        let gate = Arc::new(JournalGate::new(journal, false));
        // Initialize ontology table with one pre-existing tag.
        let ont = TagOntologyStore::open(tmp.path(), "ws").unwrap();
        ont.add("preseeded", AddedVia::Manual).unwrap();

        let mut ws = Workstream::new("ws", tmp.path().join("workstreams/ws"));
        ws.description = "test".into();
        (tmp, mgr, gate, ws)
    }

    fn entity_with_discovered(title: &str, tags: &[&str]) -> Entity {
        Entity::new(EntityType::Fact, title)
            .with_tags(tags.iter().map(|s| s.to_string()).collect())
    }

    fn ctx(
        mem: &Arc<MemoryManager>,
        gate: &Arc<JournalGate>,
        workstream: &Workstream,
        cap: usize,
    ) -> SubroutineCtx {
        SubroutineCtx {
            workstream: workstream.clone(),
            memory: Arc::clone(mem),
            journal: Arc::clone(gate),
            cap,
        }
    }

    #[tokio::test]
    async fn promotes_tag_at_threshold() {
        let (_tmp, mem, gate, ws) = setup();
        for i in 0..3 {
            mem.workstream
                .insert_entity(&entity_with_discovered(
                    &format!("e{i}"),
                    &["calidor"],
                ))
                .unwrap();
        }
        let sub = TagPromoterSubroutine::default();
        let out = sub.run(&ctx(&mem, &gate, &ws, 10)).await.unwrap();
        assert_eq!(out.proposals_recorded, 1);

        let inner = gate.inner_journal();
        let recent = inner.recent(10).unwrap();
        let row = recent.iter().find(|r| r.subroutine == "tag-promoter").unwrap();
        assert_eq!(row.action, "promote_tag");
        assert!(!row.applied);
        assert!(row.outputs_json.contains("calidor"));
    }

    #[tokio::test]
    async fn below_threshold_no_proposal() {
        let (_tmp, mem, gate, ws) = setup();
        for i in 0..2 {
            mem.workstream
                .insert_entity(&entity_with_discovered(
                    &format!("e{i}"),
                    &["barely"],
                ))
                .unwrap();
        }
        let sub = TagPromoterSubroutine::default();
        let out = sub.run(&ctx(&mem, &gate, &ws, 10)).await.unwrap();
        assert_eq!(out.proposals_recorded, 0);
    }

    #[tokio::test]
    async fn skips_tags_already_in_ontology() {
        let (_tmp, mem, gate, ws) = setup();
        for i in 0..5 {
            mem.workstream
                .insert_entity(&entity_with_discovered(
                    &format!("e{i}"),
                    &["preseeded"],
                ))
                .unwrap();
        }
        let sub = TagPromoterSubroutine::default();
        let out = sub.run(&ctx(&mem, &gate, &ws, 10)).await.unwrap();
        assert_eq!(out.proposals_recorded, 0);
    }

    #[tokio::test]
    async fn skips_steward_internal_markers() {
        let (_tmp, mem, gate, ws) = setup();
        for i in 0..5 {
            mem.workstream
                .insert_entity(&entity_with_discovered(
                    &format!("e{i}"),
                    &["steward:dust"],
                ))
                .unwrap();
        }
        let sub = TagPromoterSubroutine::default();
        let out = sub.run(&ctx(&mem, &gate, &ws, 10)).await.unwrap();
        assert_eq!(out.proposals_recorded, 0);
    }

    #[tokio::test]
    async fn dedupes_against_pending_proposals() {
        let (_tmp, mem, gate, ws) = setup();
        for i in 0..3 {
            mem.workstream
                .insert_entity(&entity_with_discovered(
                    &format!("e{i}"),
                    &["recur"],
                ))
                .unwrap();
        }
        let sub = TagPromoterSubroutine::default();
        // First run proposes; second run sees the pending proposal
        // and skips the same tag.
        let first = sub.run(&ctx(&mem, &gate, &ws, 10)).await.unwrap();
        assert_eq!(first.proposals_recorded, 1);
        let second = sub.run(&ctx(&mem, &gate, &ws, 10)).await.unwrap();
        assert_eq!(second.proposals_recorded, 0);
    }

    #[tokio::test]
    async fn cap_caps_proposals_per_pass() {
        let (_tmp, mem, gate, ws) = setup();
        // Three different tags, each hitting threshold.
        for tag in ["alpha", "beta", "gamma"] {
            for i in 0..3 {
                mem.workstream
                    .insert_entity(&entity_with_discovered(
                        &format!("{tag}-{i}"),
                        &[tag],
                    ))
                    .unwrap();
            }
        }
        let sub = TagPromoterSubroutine::default();
        // Cap to 2 — should propose 2 of 3 tags.
        let out = sub.run(&ctx(&mem, &gate, &ws, 2)).await.unwrap();
        assert!(out.cap_hit);
        assert_eq!(out.proposals_recorded, 2);
    }

    #[tokio::test]
    async fn normalizes_case_and_whitespace_during_tally() {
        let (_tmp, mem, gate, ws) = setup();
        // Same tag emitted in three case/whitespace variants — should
        // tally as one tag.
        mem.workstream
            .insert_entity(&entity_with_discovered("a", &["Falcon"]))
            .unwrap();
        mem.workstream
            .insert_entity(&entity_with_discovered("b", &["falcon "]))
            .unwrap();
        mem.workstream
            .insert_entity(&entity_with_discovered("c", &["FALCON"]))
            .unwrap();
        let sub = TagPromoterSubroutine::default();
        let out = sub.run(&ctx(&mem, &gate, &ws, 10)).await.unwrap();
        assert_eq!(out.proposals_recorded, 1);
        let inner = gate.inner_journal();
        let recent = inner.recent(5).unwrap();
        let row = recent.iter().find(|r| r.subroutine == "tag-promoter").unwrap();
        // Normalized form is what gets stored.
        assert!(row.outputs_json.contains("\"falcon\""));
    }
}
