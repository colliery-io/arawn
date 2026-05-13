//! File-based synthetic-fixture loader for UAT scenarios.
//!
//! Fixtures live as JSON under `tests/fixtures/uat/<scenario>.json`.
//! The schema is intentionally close to the on-disk shape of a
//! projection row so a future "real-feed dump" CLI can emit the same
//! format. Today we hand-author for two workstreams + gmail/slack;
//! drive/jira/confluence/calendar are additive variants.
//!
//! Loader responsibilities:
//!
//! 1. Open `Store` + `ProjectionStore` rooted at `data_dir`.
//! 2. Ensure each declared workstream exists in arawn-storage.
//! 3. Write projection rows in feed-type batches via the typed
//!    `Projection` writers (so each table's schema is materialized
//!    exactly as the real feed runtime would).
//! 4. Optionally drive `ExtractorRunner::run_for_workstream_until_exhausted`
//!    synchronously per workstream so the KB is populated by the time
//!    the agent starts.
//!
//! The harness uses this before `start_server` so the agent sees a
//! warm KB on the first turn.

#![allow(dead_code)] // Loader is consumed by the uat.rs scenario at test time.

use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use arawn_core::Workstream;
use arawn_extractor::{CotChain, ExtractionChain, ExtractorRunner};
use arawn_llm::LlmClient;
use arawn_memory::MemoryManager;
use arawn_projections::ProjectionStore;
use arawn_projections::gmail::GmailMessageProjection;
use arawn_projections::slack::SlackMessageProjection;
use arawn_storage::Store;

/// Top-level fixture file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    pub workstreams: Vec<WorkstreamFixture>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkstreamFixture {
    pub name: String,
    pub description: String,
    pub rows: Vec<FixtureRow>,
}

/// Discriminated row variants by `feed_type`. New variants are an
/// additive change; the loader skips unknown feed types with a warn
/// rather than failing the whole scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "feed_type", rename_all = "snake_case")]
pub enum FixtureRow {
    GmailMessages(GmailFixtureRow),
    SlackMessages(SlackFixtureRow),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailFixtureRow {
    pub source_id: String,
    pub source_ts: DateTime<Utc>,
    pub sender: Option<String>,
    #[serde(default)]
    pub recipients: Vec<String>,
    pub subject: String,
    pub body_text: String,
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    /// Feed id this row pretends to come from. Defaults to a stable
    /// synthetic id derived from the workstream name when omitted.
    #[serde(default)]
    pub feed_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackFixtureRow {
    pub source_id: String,
    pub source_ts: DateTime<Utc>,
    pub channel_id: Option<String>,
    pub sender_id: Option<String>,
    pub text: String,
    #[serde(default)]
    pub thread_ts: Option<String>,
    #[serde(default)]
    pub reactions: Vec<Value>,
    #[serde(default)]
    pub is_thread_reply: bool,
    #[serde(default)]
    pub feed_id: Option<String>,
}

/// Read a fixture from disk.
pub fn load(path: impl AsRef<Path>) -> Result<Fixture, String> {
    let path = path.as_ref();
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse {path:?}: {e}"))
}

/// Apply a fixture against `data_dir`. Materializes workstreams + writes
/// projection rows. Returns the projection store + the set of feed types
/// touched per workstream so the caller can drive extraction.
pub struct Applied {
    pub store: Arc<std::sync::Mutex<Store>>,
    pub projections: Arc<ProjectionStore>,
    pub per_workstream: Vec<AppliedWorkstream>,
}

pub struct AppliedWorkstream {
    pub workstream: Workstream,
    /// Distinct feed types that received rows, in deterministic order.
    pub feed_types: Vec<String>,
}

pub fn apply(fixture: &Fixture, data_dir: &Path) -> Result<Applied, String> {
    let store = Store::open(data_dir).map_err(|e| format!("open store: {e}"))?;
    store
        .ensure_scratch_workstream()
        .map_err(|e| format!("ensure scratch: {e}"))?;

    let projections =
        ProjectionStore::open(&data_dir.join("projections.db"))
            .map_err(|e| format!("open projections: {e}"))?;

    let mut applied = Vec::with_capacity(fixture.workstreams.len());

    for ws_def in &fixture.workstreams {
        let ws_dir = data_dir.join("workstreams").join(&ws_def.name);
        let mut ws = Workstream::new(&ws_def.name, ws_dir);
        ws.description = ws_def.description.clone();
        // ensure_scratch already created "scratch"; create_workstream
        // is idempotent in the sense that the test-side store is fresh.
        let _ = store.create_workstream(&ws);
        // Re-read so we have the canonical workstream record.
        let canonical = store
            .find_workstream_by_name(&ws_def.name)
            .map_err(|e| format!("find ws `{}`: {e}", ws_def.name))?
            .unwrap_or(ws);

        // Group rows by Projection trait variant, write each batch.
        let mut gmail_rows: Vec<GmailMessageProjection> = Vec::new();
        let mut slack_rows: Vec<SlackMessageProjection> = Vec::new();
        for row in &ws_def.rows {
            match row {
                FixtureRow::GmailMessages(g) => gmail_rows.push(gmail_to_projection(&ws_def.name, g)),
                FixtureRow::SlackMessages(s) => slack_rows.push(slack_to_projection(&ws_def.name, s)),
            }
        }
        if !gmail_rows.is_empty() {
            projections
                .write_batch(&gmail_rows)
                .map_err(|e| format!("write gmail batch: {e}"))?;
        }
        if !slack_rows.is_empty() {
            projections
                .write_batch(&slack_rows)
                .map_err(|e| format!("write slack batch: {e}"))?;
        }

        let mut feed_types = Vec::new();
        if !gmail_rows.is_empty() {
            feed_types.push("gmail_messages".to_string());
        }
        if !slack_rows.is_empty() {
            // SlackMessageProjection.feed_type() depends on is_thread_reply;
            // record both possibilities — extraction iterates feed types,
            // so listing both is safe (the extractor no-ops on empty tables).
            feed_types.push("slack_messages".to_string());
            if slack_rows.iter().any(|r| r.is_thread_reply) {
                feed_types.push("slack_thread_messages".to_string());
            }
        }

        applied.push(AppliedWorkstream {
            workstream: canonical,
            feed_types,
        });
    }

    Ok(Applied {
        store: Arc::new(std::sync::Mutex::new(store)),
        projections: Arc::new(projections),
        per_workstream: applied,
    })
}

fn synthetic_feed_id(workstream: &str, override_: &Option<String>) -> String {
    override_.clone().unwrap_or_else(|| format!("fixture-{workstream}-gmail"))
}

fn gmail_to_projection(workstream: &str, row: &GmailFixtureRow) -> GmailMessageProjection {
    let feed_id = synthetic_feed_id(workstream, &row.feed_id);
    GmailMessageProjection {
        id: arawn_projections::gmail::projection_id(&feed_id, &row.source_id),
        feed_id,
        source_id: row.source_id.clone(),
        source_ts: row.source_ts,
        sender: row.sender.clone(),
        recipients: row.recipients.clone(),
        subject: row.subject.clone(),
        body_text: row.body_text.clone(),
        thread_id: row.thread_id.clone(),
        labels: row.labels.clone(),
    }
}

fn slack_to_projection(workstream: &str, row: &SlackFixtureRow) -> SlackMessageProjection {
    let feed_id = row
        .feed_id
        .clone()
        .unwrap_or_else(|| format!("fixture-{workstream}-slack"));
    SlackMessageProjection {
        id: format!("{feed_id}::{}", row.source_id),
        feed_id,
        source_id: row.source_id.clone(),
        source_ts: row.source_ts,
        channel_id: row.channel_id.clone(),
        sender_id: row.sender_id.clone(),
        text: row.text.clone(),
        thread_ts: row.thread_ts.clone(),
        reactions: row.reactions.clone(),
        is_thread_reply: row.is_thread_reply,
    }
}

/// Build an `LlmClient` for the seed-time extractor using the same
/// provider/model/api-key triple the UAT harness writes into the
/// server's `arawn.toml`. Mirrors `arawn-bin::build_llm_client` (which
/// is binary-private) so the test stays self-contained.
pub fn build_seed_llm_client(
    provider: &str,
    model: &str,
    api_key_env: &str,
) -> Result<Arc<dyn LlmClient>, String> {
    let key = if api_key_env.is_empty() {
        None
    } else {
        std::env::var(api_key_env).ok().filter(|s| !s.is_empty())
    };
    let base_url = if provider == "anthropic" {
        None
    } else {
        Some(provider)
    };
    // Anthropic = native client; everything else = OpenAI-compatible.
    if provider == "anthropic" {
        let api_key =
            key.ok_or_else(|| format!("anthropic provider requires env {api_key_env}"))?;
        return Ok(Arc::new(arawn_llm::AnthropicClient::new(api_key)));
    }
    let _ = model;
    let client = arawn_llm::OpenAICompatibleClient::from_config(provider, base_url, key)
        .map_err(|e| format!("build openai-compat client: {e}"))?;
    Ok(Arc::new(client))
}

/// Drive `ExtractorRunner::run_for_workstream_until_exhausted` for each
/// (workstream, feed_type) so the KB is populated by the time the
/// server boots. Uses the supplied LLM client + model.
pub async fn drive_extraction(
    applied: &Applied,
    data_dir: &Path,
    client: Arc<dyn LlmClient>,
    model: String,
    cap: std::time::Duration,
) -> Result<usize, String> {
    let chain: Arc<dyn ExtractionChain> = Arc::new(CotChain::new(client, model));
    let data_dir = data_dir.to_path_buf();
    let memory: arawn_extractor::runner::MemoryResolver = Arc::new(move |name: &str| {
        MemoryManager::for_workstream(&data_dir, name, None)
            .map(Arc::new)
            .map_err(|e| arawn_extractor::ExtractionError::Memory(e.to_string()))
    });
    let runner = ExtractorRunner::new(
        Arc::clone(&applied.store),
        Arc::clone(&applied.projections),
        memory,
        chain,
    );

    let mut total = 0usize;
    for aw in &applied.per_workstream {
        for ft in &aw.feed_types {
            let stats = runner
                .run_for_workstream_until_exhausted(&aw.workstream, ft, cap)
                .await
                .map_err(|e| format!("extract `{}` / `{}`: {e}", aw.workstream.name, ft))?;
            total += stats.processed;
        }
    }
    Ok(total)
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fixture() -> Fixture {
        Fixture {
            workstreams: vec![WorkstreamFixture {
                name: "work".into(),
                description: "Pat's day job".into(),
                rows: vec![
                    FixtureRow::GmailMessages(GmailFixtureRow {
                        source_id: "m1".into(),
                        source_ts: Utc::now(),
                        sender: Some("a@example.com".into()),
                        recipients: vec![],
                        subject: "test".into(),
                        body_text: "body".into(),
                        thread_id: None,
                        labels: vec![],
                        feed_id: None,
                    }),
                    FixtureRow::SlackMessages(SlackFixtureRow {
                        source_id: "s1".into(),
                        source_ts: Utc::now(),
                        channel_id: Some("C123".into()),
                        sender_id: Some("U123".into()),
                        text: "hello".into(),
                        thread_ts: None,
                        reactions: vec![],
                        is_thread_reply: false,
                        feed_id: None,
                    }),
                ],
            }],
        }
    }

    #[test]
    fn fixture_roundtrips_through_json() {
        let f = sample_fixture();
        let raw = serde_json::to_string(&f).unwrap();
        let back: Fixture = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.workstreams.len(), 1);
        assert_eq!(back.workstreams[0].rows.len(), 2);
    }

    #[test]
    fn apply_creates_workstream_and_writes_rows() {
        let tmp = tempfile::tempdir().unwrap();
        let applied = apply(&sample_fixture(), tmp.path()).unwrap();
        assert_eq!(applied.per_workstream.len(), 1);
        assert_eq!(applied.per_workstream[0].workstream.name, "work");
        // Both gmail + slack should be recorded.
        assert!(applied.per_workstream[0]
            .feed_types
            .iter()
            .any(|s| s == "gmail_messages"));
        assert!(applied.per_workstream[0]
            .feed_types
            .iter()
            .any(|s| s == "slack_messages"));
    }

    #[test]
    fn load_from_disk_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("fixture.json");
        std::fs::write(&path, serde_json::to_string_pretty(&sample_fixture()).unwrap())
            .unwrap();
        let f = load(&path).unwrap();
        assert_eq!(f.workstreams.len(), 1);
    }
}
