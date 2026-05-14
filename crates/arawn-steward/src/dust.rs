//! Dust subroutine — manual trigger (T-0260).
//!
//! Per user direction this is *not* on the steward's auto cadence. The
//! `workstream_dust` agent tool wraps a single run. Dust writes
//! proposals as `applied = false` journal rows; the user reviews via
//! `workstream_refine` and commits via `workstream_apply <id>`.
//!
//! Allowed verbs per ARAWN-A-0003: insert summary entity + add
//! SUMMARIZES edges. Sources are preserved.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use tracing::{debug, warn};
use uuid::Uuid;

use arawn_llm::LlmClient;
use arawn_memory::{ConfidenceSource, Entity, EntityType, MemoryManager, RelationType};

use crate::error::StewardError;
use crate::journal::{Journal, JournalRecord};
use crate::llm_text::{complete_text, extract_json_block};

pub const SUBROUTINE_NAME: &str = "dust";

#[derive(Debug, Clone, Copy)]
pub enum ClusterMode {
    /// Group by shared tag.
    Tag,
    /// Group by shared EXTRACTED_FROM provenance target.
    Provenance,
}

impl ClusterMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tag" => Some(Self::Tag),
            "provenance" => Some(Self::Provenance),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DustOpts {
    pub cluster_by: ClusterMode,
    pub min_cluster_size: usize,
    pub idle_days: i64,
    /// Maximum summaries to propose in this run.
    pub limit: usize,
    /// Optional restriction: only consider clusters whose key matches
    /// one of these (tags for `Tag` mode; ignored for `Provenance`).
    pub tag_filter: Option<Vec<String>>,
    /// Cap on entities fed to the LLM per cluster — big clusters get
    /// sampled; the journal records the total count.
    pub max_members_in_prompt: usize,
}

impl Default for DustOpts {
    fn default() -> Self {
        Self {
            cluster_by: ClusterMode::Tag,
            min_cluster_size: 3,
            idle_days: 30,
            limit: 5,
            tag_filter: None,
            max_members_in_prompt: 30,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct DustOutcome {
    pub clusters_found: usize,
    pub proposals_written: usize,
    pub proposal_ids: Vec<i64>,
    pub limit_hit: bool,
}

pub struct DustEngine {
    client: Arc<dyn LlmClient>,
    model: String,
}

impl DustEngine {
    pub fn new(client: Arc<dyn LlmClient>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
        }
    }

    /// Run one dust pass on `kb`, writing proposals into `journal`.
    /// Returns proposal ids the caller can surface to the user.
    pub async fn run(
        &self,
        kb: &Arc<MemoryManager>,
        journal: &Journal,
        opts: &DustOpts,
    ) -> Result<DustOutcome, StewardError> {
        let mut outcome = DustOutcome::default();
        let now = Utc::now();
        let threshold = now - Duration::days(opts.idle_days);

        let active = kb
            .workstream
            .list_all_ranked(2_000)
            .map_err(StewardError::from)?;
        let clusters = match opts.cluster_by {
            ClusterMode::Tag => cluster_by_tag(&active, opts),
            ClusterMode::Provenance => cluster_by_provenance(&active, kb, opts)?,
        };
        let stale_clusters: Vec<(String, Vec<Entity>)> = clusters
            .into_iter()
            .filter(|(_, members)| {
                members.len() >= opts.min_cluster_size
                    && members.iter().all(|e| e.updated_at < threshold)
            })
            .collect();
        outcome.clusters_found = stale_clusters.len();

        for (key, members) in stale_clusters {
            if outcome.proposals_written >= opts.limit {
                outcome.limit_hit = true;
                break;
            }
            let (proposal_id, _) = match self.summarize_cluster(&key, &members, kb, journal, opts).await
            {
                Ok(v) => v,
                Err(e) => {
                    warn!(cluster = %key, error = %e, "dust: cluster summarization failed");
                    continue;
                }
            };
            outcome.proposal_ids.push(proposal_id);
            outcome.proposals_written += 1;
        }
        Ok(outcome)
    }

    async fn summarize_cluster(
        &self,
        cluster_key: &str,
        members: &[Entity],
        _kb: &Arc<MemoryManager>,
        journal: &Journal,
        opts: &DustOpts,
    ) -> Result<(i64, Uuid), StewardError> {
        // Sample at most `max_members_in_prompt` members; journal stores
        // the full id list so apply / revert see everything.
        let sample: Vec<&Entity> = members.iter().take(opts.max_members_in_prompt).collect();
        let summary_payload = self.ask_for_summary(cluster_key, &sample).await?;

        // Construct the summary entity. The cluster key (ontology tag
        // when clustering by tag) goes on `tags_ontology` so future
        // signal_query / dust passes find the summary alongside the
        // sources. The `steward:dust` marker lives on the discovered
        // set so cluster-by-ontology won't accidentally include
        // prior summaries as members of a new cluster.
        let mut tags_ontology = Vec::new();
        if matches!(opts.cluster_by, ClusterMode::Tag) {
            tags_ontology.push(cluster_key.to_string());
        }
        let mut tags_discovered = vec!["steward:dust".to_string()];
        for t in summary_payload.tags {
            if !tags_discovered.contains(&t) && !tags_ontology.contains(&t) {
                tags_discovered.push(t);
            }
        }
        let summary = Entity::new(EntityType::Note, summary_payload.title)
            .with_content(summary_payload.content)
            .with_tags(tags_discovered)
            .with_tags_ontology(tags_ontology)
            .with_confidence(ConfidenceSource::Inferred);

        let source_ids: Vec<Uuid> = members.iter().map(|e| e.id).collect();
        let payload = json!({
            "cluster_key": cluster_key,
            "cluster_mode": match opts.cluster_by {
                ClusterMode::Tag => "tag",
                ClusterMode::Provenance => "provenance",
            },
            "summary": &summary,
            "source_ids": source_ids,
            "members_total": members.len(),
            "members_in_prompt": sample.len(),
        });
        let record = JournalRecord {
            subroutine: SUBROUTINE_NAME.into(),
            action: "summarize".into(),
            inputs_json: json!({"cluster_key": cluster_key}).to_string(),
            outputs_json: payload.to_string(),
            model: self.model.clone(),
            prompt_hash: Journal::prompt_hash(format!("dust/{cluster_key}")),
            applied: false,
        };
        let id = journal.write_ahead(&record)?;
        debug!(
            cluster = %cluster_key,
            summary = %summary.id,
            members = members.len(),
            journal_id = id,
            "dust: proposal written"
        );
        Ok((id, summary.id))
    }

    async fn ask_for_summary(
        &self,
        cluster_key: &str,
        members: &[&Entity],
    ) -> Result<ProposedSummary, StewardError> {
        let system = "You compress a cluster of related but cold knowledge-base entities into \
                      one short summary entity. Output ONLY a JSON object: \
                      {\"title\": short, \"content\": longer paragraph, \
                       \"tags\": optional array of short slugs}. The summary should let a \
                      future reader recognize the gist without re-reading each source.";
        let body = json!({
            "cluster_key": cluster_key,
            "members": members
                .iter()
                .map(|e| {
                    json!({
                        "id": e.id,
                        "entity_type": e.entity_type.as_str(),
                        "title": e.title,
                        "content": e.content.as_deref().unwrap_or(""),
                        "tags": e.tags,
                    })
                })
                .collect::<Vec<_>>(),
        });
        let user = serde_json::to_string_pretty(&body)?;
        let raw = complete_text(&self.client, &self.model, system, &user).await?;
        let json = extract_json_block(&raw)
            .ok_or_else(|| StewardError::Parse(format!("dust: no JSON in LLM response: {raw}")))?;
        Ok(serde_json::from_str(json)?)
    }
}

#[derive(Debug, Deserialize)]
struct ProposedSummary {
    title: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    tags: Vec<String>,
}

fn cluster_by_tag(active: &[Entity], opts: &DustOpts) -> Vec<(String, Vec<Entity>)> {
    // Per ADR-0004 dust clusters on `tags_ontology` only — that's the
    // deterministic substrate, drawn from the workstream's declared
    // closed list. `tags_discovered` is too noisy to cluster on
    // directly (variants like `falcon` vs `falcon-project` defeat
    // exact-string grouping).
    let mut by_tag: HashMap<String, Vec<Entity>> = HashMap::new();
    for e in active {
        // Skip prior dust outputs so we don't re-summarize summaries.
        // The marker still lives on the discovered set since it's a
        // steward-internal annotation, not part of any user ontology.
        if e.tags.iter().any(|t| t == "steward:dust") {
            continue;
        }
        for t in &e.tags_ontology {
            if let Some(filter) = &opts.tag_filter
                && !filter.iter().any(|f| f == t)
            {
                continue;
            }
            by_tag
                .entry(t.clone())
                .or_default()
                .push(e.clone());
        }
    }
    by_tag.into_iter().collect()
}

fn cluster_by_provenance(
    active: &[Entity],
    kb: &Arc<MemoryManager>,
    _opts: &DustOpts,
) -> Result<Vec<(String, Vec<Entity>)>, StewardError> {
    let mut by_src: HashMap<Uuid, Vec<Entity>> = HashMap::new();
    for e in active {
        if e.tags.iter().any(|t| t == "steward:dust") {
            continue;
        }
        let rels = kb
            .workstream
            .get_relations(e.id)
            .map_err(StewardError::from)?;
        for r in rels {
            if matches!(r.relation_type, RelationType::ExtractedFrom) && r.source_id == e.id {
                by_src.entry(r.target_id).or_default().push(e.clone());
            }
        }
    }
    Ok(by_src
        .into_iter()
        .map(|(target, ents)| (target.to_string(), ents))
        .collect())
}

// Touch warn-unused: chrono is used; this helper is a no-op alias to
// keep the import set obvious to readers.
#[allow(dead_code)]
fn _ts() -> DateTime<Utc> {
    Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::pin::Pin;
    use std::sync::Mutex;

    use arawn_llm::{
        LlmError,
        types::{ChatChunk, ChatRequest},
    };
    use async_trait::async_trait;
    use futures::stream;
    use serde_json::Value;

    struct ScriptedMock {
        responses: Mutex<VecDeque<Value>>,
    }
    impl ScriptedMock {
        fn new(v: Vec<Value>) -> Self {
            Self {
                responses: Mutex::new(v.into_iter().collect()),
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

    fn make_stale_entity(title: &str, tag: &str, days_old: i64) -> Entity {
        // Dust clusters on tags_ontology — populate that field directly.
        let mut e = Entity::new(EntityType::Fact, title)
            .with_tags_ontology(vec![tag.into()]);
        e.created_at = Utc::now() - Duration::days(days_old);
        e.updated_at = e.created_at;
        e
    }

    fn setup() -> (tempfile::TempDir, Arc<MemoryManager>, Journal) {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = Arc::new(MemoryManager::open(tmp.path(), "ws", None).unwrap());
        let j = Journal::open(tmp.path(), "ws").unwrap();
        (tmp, mgr, j)
    }

    #[tokio::test]
    async fn tag_cluster_writes_proposal_when_all_idle() {
        let (_tmp, mgr, j) = setup();
        for i in 0..3 {
            let e = make_stale_entity(&format!("t{i}"), "project-x", 60);
            mgr.workstream.insert_entity(&e).unwrap();
        }
        let mock = Arc::new(ScriptedMock::new(vec![json!({
            "title": "project x — closed",
            "content": "three notes about project x, all idle ~60d",
            "tags": ["wrap-up"]
        })]));
        let eng = DustEngine::new(mock as Arc<dyn LlmClient>, "mock");
        let out = eng.run(&mgr, &j, &DustOpts::default()).await.unwrap();
        assert_eq!(out.proposals_written, 1);
        assert_eq!(out.proposal_ids.len(), 1);
        let row = j.get(out.proposal_ids[0]).unwrap().unwrap();
        assert_eq!(row.subroutine, "dust");
        assert!(!row.applied, "proposals must be applied=false");
    }

    #[tokio::test]
    async fn cluster_with_one_fresh_member_is_skipped() {
        let (_tmp, mgr, j) = setup();
        // 2 stale + 1 fresh — cluster size 3, but not all idle.
        mgr.workstream.insert_entity(&make_stale_entity("a", "p", 60)).unwrap();
        mgr.workstream.insert_entity(&make_stale_entity("b", "p", 60)).unwrap();
        mgr.workstream.insert_entity(&make_stale_entity("c", "p", 1)).unwrap();
        let mock = Arc::new(ScriptedMock::new(vec![]));
        let eng = DustEngine::new(mock as Arc<dyn LlmClient>, "mock");
        let out = eng.run(&mgr, &j, &DustOpts::default()).await.unwrap();
        assert_eq!(out.proposals_written, 0);
    }

    #[tokio::test]
    async fn min_cluster_size_filters_out_small_clusters() {
        let (_tmp, mgr, j) = setup();
        mgr.workstream.insert_entity(&make_stale_entity("a", "tiny", 60)).unwrap();
        mgr.workstream.insert_entity(&make_stale_entity("b", "tiny", 60)).unwrap();
        let mock = Arc::new(ScriptedMock::new(vec![]));
        let eng = DustEngine::new(mock as Arc<dyn LlmClient>, "mock");
        let opts = DustOpts {
            min_cluster_size: 3,
            ..DustOpts::default()
        };
        let out = eng.run(&mgr, &j, &opts).await.unwrap();
        assert_eq!(out.proposals_written, 0);
    }

    #[tokio::test]
    async fn limit_caps_proposals_per_run() {
        let (_tmp, mgr, j) = setup();
        for tag in ["a", "b", "c"] {
            for i in 0..3 {
                mgr.workstream
                    .insert_entity(&make_stale_entity(&format!("{tag}-{i}"), tag, 60))
                    .unwrap();
            }
        }
        let mock = Arc::new(ScriptedMock::new(vec![
            json!({"title": "t1", "content": "c1"}),
            json!({"title": "t2", "content": "c2"}),
        ]));
        let eng = DustEngine::new(mock as Arc<dyn LlmClient>, "mock");
        let opts = DustOpts {
            limit: 2,
            ..DustOpts::default()
        };
        let out = eng.run(&mgr, &j, &opts).await.unwrap();
        assert!(out.limit_hit);
        assert_eq!(out.proposals_written, 2);
    }

    #[tokio::test]
    async fn prior_dust_summaries_are_excluded_from_new_clusters() {
        let (_tmp, mgr, j) = setup();
        // Three real entities — stale.
        for i in 0..3 {
            mgr.workstream.insert_entity(&make_stale_entity(&format!("e{i}"), "p", 60)).unwrap();
        }
        // A prior dust summary with the same ontology tag — should
        // be skipped via the steward:dust marker on the discovered set
        // even though it shares the cluster key on ontology.
        let mut prior = Entity::new(EntityType::Note, "old summary")
            .with_tags_ontology(vec!["p".into()])
            .with_tags(vec!["steward:dust".into()]);
        prior.created_at = Utc::now() - Duration::days(60);
        prior.updated_at = prior.created_at;
        mgr.workstream.insert_entity(&prior).unwrap();

        let mock = Arc::new(ScriptedMock::new(vec![json!({
            "title": "p — closed",
            "content": "summary of three entities (excluding prior dust)"
        })]));
        let eng = DustEngine::new(mock as Arc<dyn LlmClient>, "mock");
        let out = eng.run(&mgr, &j, &DustOpts::default()).await.unwrap();
        assert_eq!(out.proposals_written, 1);
        // Confirm the proposal's source_ids does NOT include the prior summary.
        let row = j.get(out.proposal_ids[0]).unwrap().unwrap();
        assert!(!row.outputs_json.contains(&prior.id.to_string()));
    }
}
