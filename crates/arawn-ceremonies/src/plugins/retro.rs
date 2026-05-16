//! Friday retro ceremony plugin.
//!
//! Runs Friday afternoon (default 16:00 local, configurable later).
//! Three sections:
//!
//! 1. **What happened** — grounded summary of the week from daily
//!    tablets + the activity rollup + confirmed priorities. Every
//!    claim cites a gather-payload row id.
//! 2. **Patterns** — observations from the registered detectors
//!    (T-0288). Each pattern item cites its `ceremony_patterns_detected`
//!    row id. The DetectorRegistry skips rules whose lookback
//!    exceeds available history, so a fresh install renders without
//!    a patterns section until ≥ N weeks accumulate.
//! 3. **Your reflection** — empty diary slot. The user writes via
//!    `ceremonies.upsert_diary` (T-0289); the plugin does not touch
//!    section 3 during compose.
//!
//! The compose phase is the only LLM call. We use the `hint:medium`
//! model string so the engine routes through the medium tier
//! (T-0272 hint taxonomy + T-0278 routing policy). The LLM gate
//! around compose lives in the dispatcher (T-0282); the plugin
//! does not gate again.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Utc};
use futures::StreamExt;
use rusqlite::params;
use serde::Serialize;

use crate::CeremonyError;
use crate::patterns::DetectorRegistry;
use crate::plugin::{
    Ceremony, CeremonyCtx, ComposedItem, CronSchedule, InteractiveAction, NewItem,
    PatternDetector,
};
use crate::types::{GatheredFacts, ItemKind};

/// The retro plugin.
pub struct RetroCeremony {
    llm: Arc<dyn arawn_llm::LlmClient>,
    /// Concrete model string the binary resolved from `hint:medium`.
    /// We store the resolved string because the LLM client itself
    /// is not the routing layer; resolution happens at construction
    /// in the binary's pool setup.
    model: String,
    /// Detector registry that runs during pattern detect. Defaults
    /// to empty in this task; T-0288's catalog populates it.
    detectors: DetectorRegistry,
}

impl RetroCeremony {
    pub fn new(llm: Arc<dyn arawn_llm::LlmClient>, model: impl Into<String>) -> Self {
        Self {
            llm,
            model: model.into(),
            detectors: DetectorRegistry::new(),
        }
    }

    /// Attach the detector registry (typically the v1 catalog
    /// from T-0288). Chainable.
    pub fn with_detectors(mut self, detectors: DetectorRegistry) -> Self {
        self.detectors = detectors;
        self
    }

    /// Compute the ISO-week string (`YYYY-Www`) for a given moment.
    /// Exposed so callers + tests can predict the tablet id without
    /// running the plugin.
    pub fn iso_week(now: DateTime<Utc>) -> String {
        let iso = now.iso_week();
        format!("{:04}-W{:02}", iso.year(), iso.week())
    }
}

// --- Gather payload shapes ---

#[derive(Debug, Clone, Serialize)]
struct GatherPayload {
    iso_week: String,
    daily_tablets: Vec<DailyTabletSummary>,
    confirmed_priorities: Vec<PrioritySummary>,
    weekly_rollup: Vec<RollupRow>,
    prior_retro_diaries: Vec<PriorRetro>,
}

#[derive(Debug, Clone, Serialize)]
struct DailyTabletSummary {
    tablet_id: String,
    period_key: String,
    item_count: i64,
}

#[derive(Debug, Clone, Serialize)]
struct PrioritySummary {
    id: String,
    body: String,
    done: bool,
}

#[derive(Debug, Clone, Serialize)]
struct RollupRow {
    workstream: String,
    metric_key: String,
    value: f64,
}

#[derive(Debug, Clone, Serialize)]
struct PriorRetro {
    iso_week: String,
    diary_excerpt: Option<String>,
}

#[async_trait]
impl Ceremony for RetroCeremony {
    fn kind(&self) -> &'static str {
        "retro"
    }

    fn period_key(&self, now: DateTime<Utc>) -> String {
        Self::iso_week(now)
    }

    fn default_schedule(&self) -> CronSchedule {
        // Friday 16:00 in local time. The engine wires cloacina
        // with this verbatim.
        CronSchedule::local("0 16 * * FRI")
    }

    fn interactive_actions(&self) -> Vec<InteractiveAction> {
        vec![InteractiveAction {
            key: "upsert_diary".to_string(),
            label: "Write your reflection".to_string(),
        }]
    }

    fn patterns(&self) -> Option<&dyn PatternDetector> {
        Some(&self.detectors)
    }

    async fn gather(&self, ctx: &dyn CeremonyCtx) -> Result<GatheredFacts, CeremonyError> {
        // Gather is deterministic SQL — no LLM. Pulls four shapes:
        //   1. daily tablets in this ISO week (we use the period_key
        //      `LIKE 'YYYY-MM-DD'` ordering after deriving the week
        //      range from `iso_week`; for v1 we just scan kind=daily
        //      and filter by string range derived from the week).
        //   2. confirmed weekly priorities on this week's tablet.
        //   3. this week's rollup rows.
        //   4. prior retro diaries (last 3).
        let conn = ctx
            .conn_handle()
            .ok_or_else(|| CeremonyError::Other("retro plugin requires EngineCtx".into()))?;
        let conn = conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;

        let iso_week = ctx.period_key().to_string();

        // 1. daily tablets — match the kind=daily rows whose
        // period_key falls within this ISO week. SQLite doesn't have
        // ISO week math, so we filter the date range computed in
        // Rust (Mon..=Sun for `iso_week`).
        let (mon, sun) = monday_sunday_for_iso_week(&iso_week)
            .ok_or_else(|| CeremonyError::Other(format!("invalid iso_week '{iso_week}'")))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, period_key, \
                        (SELECT COUNT(*) FROM ceremony_items WHERE tablet_id = t.id) \
                 FROM ceremony_tablets t \
                 WHERE kind = 'daily' AND period_key BETWEEN ?1 AND ?2 \
                 ORDER BY period_key",
            )
            .map_err(|e| CeremonyError::Storage(format!("daily prepare: {e}")))?;
        let daily_rows = stmt
            .query_map(params![mon, sun], |row| {
                Ok(DailyTabletSummary {
                    tablet_id: row.get(0)?,
                    period_key: row.get(1)?,
                    item_count: row.get(2)?,
                })
            })
            .map_err(|e| CeremonyError::Storage(format!("daily query: {e}")))?;
        let mut daily_tablets = Vec::new();
        for r in daily_rows {
            daily_tablets
                .push(r.map_err(|e| CeremonyError::Storage(format!("daily row: {e}")))?);
        }

        // 2. confirmed weekly priorities. The weekly tablet shares
        // this iso_week's period_key (kind=weekly).
        let mut stmt = conn
            .prepare(
                "SELECT p.id, p.body, p.done_at FROM ceremony_priorities p \
                 JOIN ceremony_tablets t ON p.tablet_id = t.id \
                 WHERE t.kind = 'weekly' AND t.period_key = ?1 \
                       AND p.confirmed_at IS NOT NULL \
                 ORDER BY p.ordinal",
            )
            .map_err(|e| CeremonyError::Storage(format!("priorities prepare: {e}")))?;
        let prio_rows = stmt
            .query_map(params![&iso_week], |row| {
                let done_at: Option<String> = row.get(2)?;
                Ok(PrioritySummary {
                    id: row.get(0)?,
                    body: row.get(1)?,
                    done: done_at.is_some(),
                })
            })
            .map_err(|e| CeremonyError::Storage(format!("priorities query: {e}")))?;
        let mut confirmed_priorities = Vec::new();
        for r in prio_rows {
            confirmed_priorities
                .push(r.map_err(|e| CeremonyError::Storage(format!("priorities row: {e}")))?);
        }

        // 3. this week's rollup rows.
        let mut stmt = conn
            .prepare(
                "SELECT workstream, metric_key, value FROM ceremony_activity_rollup \
                 WHERE iso_week = ?1 ORDER BY workstream, metric_key",
            )
            .map_err(|e| CeremonyError::Storage(format!("rollup prepare: {e}")))?;
        let rollup_rows = stmt
            .query_map(params![&iso_week], |row| {
                Ok(RollupRow {
                    workstream: row.get(0)?,
                    metric_key: row.get(1)?,
                    value: row.get(2)?,
                })
            })
            .map_err(|e| CeremonyError::Storage(format!("rollup query: {e}")))?;
        let mut weekly_rollup = Vec::new();
        for r in rollup_rows {
            weekly_rollup
                .push(r.map_err(|e| CeremonyError::Storage(format!("rollup row: {e}")))?);
        }

        // 4. prior retro diaries — last 3, strictly before this week.
        let mut stmt = conn
            .prepare(
                "SELECT t.period_key, d.body FROM ceremony_tablets t \
                 LEFT JOIN ceremony_diary d ON d.tablet_id = t.id \
                 WHERE t.kind = 'retro' AND t.period_key < ?1 \
                 ORDER BY t.period_key DESC LIMIT 3",
            )
            .map_err(|e| CeremonyError::Storage(format!("prior retros prepare: {e}")))?;
        let prior_rows = stmt
            .query_map(params![&iso_week], |row| {
                let diary: Option<String> = row.get(1)?;
                Ok(PriorRetro {
                    iso_week: row.get(0)?,
                    diary_excerpt: diary.map(|s| s.chars().take(400).collect()),
                })
            })
            .map_err(|e| CeremonyError::Storage(format!("prior retros query: {e}")))?;
        let mut prior_retro_diaries = Vec::new();
        for r in prior_rows {
            prior_retro_diaries
                .push(r.map_err(|e| CeremonyError::Storage(format!("prior retros row: {e}")))?);
        }

        let payload = GatherPayload {
            iso_week: iso_week.clone(),
            daily_tablets,
            confirmed_priorities,
            weekly_rollup,
            prior_retro_diaries,
        };
        let json = serde_json::to_value(&payload).map_err(|e| {
            CeremonyError::Other(format!("gather payload serialise: {e}"))
        })?;
        Ok(GatheredFacts::new(json))
    }

    async fn compose(
        &self,
        ctx: &dyn CeremonyCtx,
        facts: GatheredFacts,
    ) -> Result<Vec<NewItem>, CeremonyError> {
        let tablet_id = ctx.tablet_id().to_string();
        // Compose calls the LLM with the structured payload. The
        // model is asked for a JSON array of items, each with a
        // citation_id taken from the payload. We parse + validate
        // here so the engine's strict path only sees well-formed
        // items.
        let prompt = build_compose_prompt(&facts);

        let request = arawn_llm::types::ChatRequest {
            model: self.model.clone(),
            system_prompt: Some(SYSTEM_PROMPT.to_string()),
            messages: vec![arawn_llm::types::ChatMessage {
                role: "user".to_string(),
                content: arawn_llm::types::ChatContent::Text(prompt),
                tool_calls: Vec::new(),
                tool_call_id: None,
            }],
            tools: Vec::new(),
            max_tokens: Some(4096),
        };

        let mut stream = self
            .llm
            .stream(request)
            .await
            .map_err(|e| CeremonyError::Llm(format!("compose stream: {e}")))?;

        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| CeremonyError::Llm(format!("compose chunk: {e}")))?;
            if let arawn_llm::types::ChatChunk::TextDelta { text: t } = chunk {
                text.push_str(&t);
            }
        }

        let items: Vec<ComposedItemSpec> = parse_llm_items(&text).ok_or_else(|| {
            CeremonyError::Llm(format!(
                "compose returned unparseable output (first 200 chars): {}",
                text.chars().take(200).collect::<String>()
            ))
        })?;

        let mut out = Vec::new();
        for (i, spec) in items.into_iter().enumerate() {
            // Validate citation is non-empty (engine also enforces,
            // but failing here is friendlier than rolling back the
            // whole tablet).
            if spec.citation_id.trim().is_empty() {
                return Err(CeremonyError::missing_citation(format!(
                    "compose item index {i} has empty citation_id"
                )));
            }
            out.push(NewItem::composed(ComposedItem {
                tablet_id: tablet_id.clone(),
                section_key: spec.section.clone(),
                ordinal: i as i32,
                kind: ItemKind::Pattern,
                body: spec.body,
                citation_id: spec.citation_id,
            }));
        }
        Ok(out)
    }
}

const SYSTEM_PROMPT: &str = "\
You are arawn's retro composer. Given a JSON payload describing a user's week, \
return a JSON array of items — one per claim you make about the week. Each item: \
{\"section\": \"what_happened\" | \"patterns\", \"citation_id\": \"<row id from the payload>\", \
\"body\": {\"text\": \"<one-sentence claim>\"}}. Never fabricate a citation_id that \
isn't already in the payload. Be concise and grounded.";

fn build_compose_prompt(facts: &GatheredFacts) -> String {
    format!(
        "Compose the retro for ISO week {}. Payload:\n{}",
        facts.payload.get("iso_week").and_then(|v| v.as_str()).unwrap_or(""),
        serde_json::to_string_pretty(&facts.payload).unwrap_or_default()
    )
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ComposedItemSpec {
    section: String,
    citation_id: String,
    body: serde_json::Value,
}

/// Pull the first balanced JSON array out of the LLM's response.
/// LLMs sometimes prepend prose; this finds the array no matter
/// where it appears.
fn parse_llm_items(text: &str) -> Option<Vec<ComposedItemSpec>> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut start: Option<usize> = None;
    for (i, &b) in bytes.iter().enumerate() {
        match (start, b) {
            (None, b'[') => {
                start = Some(i);
                depth = 1;
            }
            (Some(_), b'[') => depth += 1,
            (Some(s), b']') => {
                depth -= 1;
                if depth == 0 {
                    let end = i + 1;
                    let slice = &text[s..end];
                    return serde_json::from_str(slice).ok();
                }
            }
            _ => {}
        }
    }
    None
}

/// Compute Monday and Sunday `YYYY-MM-DD` strings that bracket an
/// ISO week. Used to filter daily tablets by period_key. Returns
/// `None` when the input is malformed.
fn monday_sunday_for_iso_week(iso_week: &str) -> Option<(String, String)> {
    use chrono::{NaiveDate, Weekday};
    let parts: Vec<&str> = iso_week.split("-W").collect();
    if parts.len() != 2 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let week: u32 = parts[1].parse().ok()?;
    let monday = NaiveDate::from_isoywd_opt(year, week, Weekday::Mon)?;
    let sunday = NaiveDate::from_isoywd_opt(year, week, Weekday::Sun)?;
    Some((monday.format("%Y-%m-%d").to_string(), sunday.format("%Y-%m-%d").to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CeremonyDispatcher;
    use crate::PluginRegistry;
    use crate::engine::{ConnHandle, EngineCtx, EngineDispatcher};
    use rusqlite::params;
    use tempfile::TempDir;

    fn open_test_db() -> (TempDir, ConnHandle) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let _db = arawn_storage::Database::open(&db_path).expect("migrations");
        drop(_db);
        let conn = rusqlite::Connection::open(&db_path).expect("open conn");
        (tmp, ConnHandle::new(conn))
    }

    fn make_llm_with_response(text: &str) -> Arc<dyn arawn_llm::LlmClient> {
        Arc::new(arawn_llm::MockLlmClient::new(vec![
            arawn_llm::MockResponse::text(text),
        ]))
    }

    fn seed_minimal_history(conn: &ConnHandle, iso_week: &str) {
        let (mon, _sun) = monday_sunday_for_iso_week(iso_week).unwrap();
        let c = conn.0.lock().unwrap();
        // A daily tablet within this week.
        c.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
             VALUES (?1, 'daily', ?2, ?3, 'reviewed', '[]')",
            params!["daily-day1", mon, "2026-05-18T07:00:00Z"],
        )
        .unwrap();
        // An item under it.
        c.execute(
            "INSERT INTO ceremony_items (id, tablet_id, section_key, ordinal, kind, body, citation_id, created_at) \
             VALUES (?1, ?2, 'todo', 0, 'todo', ?3, NULL, ?4)",
            params![
                "item-1",
                "daily-day1",
                "{\"text\":\"finish docs\"}",
                "2026-05-18T07:00:00Z",
            ],
        )
        .unwrap();
        // Rollup rows.
        c.execute(
            "INSERT INTO ceremony_activity_rollup (iso_week, workstream, metric_key, value) \
             VALUES (?1, ?2, ?3, ?4)",
            params![iso_week, "proj-a", "emails_sent", 12.0],
        )
        .unwrap();
        // A weekly tablet + a confirmed priority.
        c.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
             VALUES (?1, 'weekly', ?2, ?3, 'reviewed', '[]')",
            params!["weekly-W20", iso_week, "2026-05-11T07:00:00Z"],
        )
        .unwrap();
        c.execute(
            "INSERT INTO ceremony_priorities (id, tablet_id, body, rationale, citation_id, confirmed_at, done_at, ordinal) \
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, NULL, 0)",
            params!["prio-1", "weekly-W20", "Ship retro plugin", "carry-over", "2026-05-11T08:00:00Z"],
        )
        .unwrap();
    }

    #[tokio::test]
    async fn iso_week_format_is_yyyy_w_ww() {
        // 2026-05-15 is a Friday in ISO week 20.
        let dt = DateTime::parse_from_rfc3339("2026-05-15T16:00:00Z").unwrap().with_timezone(&Utc);
        assert_eq!(RetroCeremony::iso_week(dt), "2026-W20");
    }

    #[tokio::test]
    async fn monday_sunday_brackets_iso_week_20() {
        let (mon, sun) = monday_sunday_for_iso_week("2026-W20").unwrap();
        assert_eq!(mon, "2026-05-11");
        assert_eq!(sun, "2026-05-17");
    }

    #[tokio::test]
    async fn gather_collects_week_payload() {
        let (_tmp, conn) = open_test_db();
        seed_minimal_history(&conn, "2026-W20");
        let plugin = RetroCeremony::new(make_llm_with_response("[]"), "test-model");
        let ctx = EngineCtx::new(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
        let facts = plugin.gather(&ctx).await.unwrap();
        let payload = facts.payload;
        assert_eq!(payload.get("iso_week").unwrap().as_str().unwrap(), "2026-W20");
        let daily = payload.get("daily_tablets").unwrap().as_array().unwrap();
        assert_eq!(daily.len(), 1);
        let prios = payload.get("confirmed_priorities").unwrap().as_array().unwrap();
        assert_eq!(prios.len(), 1);
        let rollup = payload.get("weekly_rollup").unwrap().as_array().unwrap();
        assert_eq!(rollup.len(), 1);
        let prior = payload.get("prior_retro_diaries").unwrap().as_array().unwrap();
        assert_eq!(prior.len(), 0);
    }

    #[tokio::test]
    async fn compose_parses_llm_array_into_composed_items() {
        let (_tmp, conn) = open_test_db();
        seed_minimal_history(&conn, "2026-W20");
        // The LLM returns one composed item citing the seeded daily
        // item id.
        let llm_response = r#"[
            {"section": "what_happened", "citation_id": "item-1",
             "body": {"text": "Shipped the doc."}}
        ]"#;
        let plugin = RetroCeremony::new(make_llm_with_response(llm_response), "test-model");
        let ctx = EngineCtx::new(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
        let facts = plugin.gather(&ctx).await.unwrap();
        let items = plugin.compose(&ctx, facts).await.unwrap();
        assert_eq!(items.len(), 1);
        match &items[0] {
            NewItem::Composed(c) => {
                assert_eq!(c.section_key, "what_happened");
                assert_eq!(c.citation_id, "item-1");
            }
            other => panic!("expected Composed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn compose_rejects_empty_citation_with_missing_citation_error() {
        let (_tmp, conn) = open_test_db();
        seed_minimal_history(&conn, "2026-W20");
        let llm_response = r#"[
            {"section": "what_happened", "citation_id": "",
             "body": {"text": "Made stuff up."}}
        ]"#;
        let plugin = RetroCeremony::new(make_llm_with_response(llm_response), "test-model");
        let ctx = EngineCtx::new(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
        let facts = plugin.gather(&ctx).await.unwrap();
        let err = plugin.compose(&ctx, facts).await.unwrap_err();
        assert!(matches!(err, CeremonyError::MissingCitation(_)));
    }

    #[tokio::test]
    async fn compose_parses_array_with_surrounding_prose() {
        // LLMs sometimes prepend explanatory text. Make sure we
        // still extract the JSON array.
        let (_tmp, conn) = open_test_db();
        seed_minimal_history(&conn, "2026-W20");
        let llm_response = r#"Here's the retro:
[
    {"section": "what_happened", "citation_id": "item-1",
     "body": {"text": "ok"}}
]
Hope that helps."#;
        let plugin = RetroCeremony::new(make_llm_with_response(llm_response), "test-model");
        let ctx = EngineCtx::new(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
        let facts = plugin.gather(&ctx).await.unwrap();
        let items = plugin.compose(&ctx, facts).await.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[tokio::test]
    async fn end_to_end_dispatch_against_real_engine() {
        // End-to-end: register the plugin, dispatch via
        // EngineDispatcher, assert tablet + item written.
        let (_tmp, conn) = open_test_db();
        seed_minimal_history(&conn, &RetroCeremony::iso_week(Utc::now()));
        let llm_response = r#"[
            {"section": "what_happened", "citation_id": "item-1",
             "body": {"text": "happy week"}}
        ]"#;
        let plugin = Arc::new(RetroCeremony::new(
            make_llm_with_response(llm_response),
            "test-model",
        ));
        let reg = PluginRegistry::new();
        reg.register(plugin).unwrap();
        let dispatcher = EngineDispatcher::new(conn.clone(), reg);
        let outcome = dispatcher.dispatch("retro").await.unwrap();
        assert!(matches!(outcome, crate::DispatchOutcome::Generated { .. }));
        // Tablet + item rows.
        let c = conn.0.lock().unwrap();
        let n_tablets: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM ceremony_tablets WHERE kind = 'retro'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n_tablets, 1);
        let n_items: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM ceremony_items \
                 WHERE tablet_id LIKE 'retro-%' AND citation_id = 'item-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n_items, 1);
    }
}
