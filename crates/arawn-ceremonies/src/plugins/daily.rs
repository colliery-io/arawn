//! Daily ceremony plugin.
//!
//! Runs Mon–Fri at 07:00 local (default). Four sections:
//!
//! 1. **calendar** — events on the day, cited from `CalendarSource`.
//! 2. **todos** — rolling todos still open, cited from
//!    `ceremony_todos_rolling`.
//! 3. **attention** — cross-feed signals worth surfacing since the
//!    last daily generation, cited from `AttentionSource`.
//! 4. **alignment** — connections to the week's confirmed
//!    priorities, cited from `ceremony_priorities`.
//!
//! Mirrors the retro plugin's shape: deterministic gather → LLM
//! compose → engine writes `ComposedItem`s with `citation_id` set.
//! The two-write-path contract from the engine refuses items without
//! a citation.
//!
//! Per the initiative's design decisions, gather caps each section's
//! row count to keep the LLM context bounded: calendar ≤ 12,
//! attention ≤ 10, todos ≤ 20, priorities ≤ 5.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use futures::StreamExt;
use rusqlite::params;
use serde::Serialize;

use crate::CeremonyError;
use crate::plugin::{
    Ceremony, CeremonyCtx, ComposedItem, CronSchedule, InteractiveAction, NewItem, PatternDetector,
};
use crate::plugins::gather_sources::{AttentionSource, CalEvent, CalendarSource, SignalRow};
use crate::types::{GatheredFacts, ItemKind};

const CAP_CALENDAR: usize = 12;
const CAP_ATTENTION: usize = 10;
const CAP_TODOS: usize = 20;
const CAP_PRIORITIES: usize = 5;

/// The daily plugin.
pub struct DailyCeremony {
    llm: Arc<dyn arawn_llm::LlmClient>,
    /// Concrete model string the binary resolved (e.g. from a
    /// `hint:small`/`hint:medium` taxonomy). The plugin doesn't
    /// route; it just hands this through to the LLM client.
    model: String,
    calendar_source: Arc<dyn CalendarSource>,
    attention_source: Arc<dyn AttentionSource>,
    /// Timezone used to interpret `period_key` dates when computing
    /// pinned windows. Defaults to UTC; the binary overrides with
    /// the configured ceremony tz via [`Self::with_timezone`].
    tz: chrono_tz::Tz,
}

impl DailyCeremony {
    pub fn new(
        llm: Arc<dyn arawn_llm::LlmClient>,
        model: impl Into<String>,
        calendar_source: Arc<dyn CalendarSource>,
        attention_source: Arc<dyn AttentionSource>,
    ) -> Self {
        Self {
            llm,
            model: model.into(),
            calendar_source,
            attention_source,
            tz: chrono_tz::UTC,
        }
    }

    /// Override the timezone used for `period_window` boundary math.
    /// The binary feeds this from `[ceremonies.daily] timezone`.
    pub fn with_timezone(mut self, tz: chrono_tz::Tz) -> Self {
        self.tz = tz;
        self
    }

    /// Format a `DateTime<Utc>` as the `YYYY-MM-DD` period key used
    /// by the daily ceremony. Exposed for callers + tests so they
    /// can predict the tablet id without running the plugin.
    pub fn period_date(now: DateTime<Utc>) -> String {
        now.format("%Y-%m-%d").to_string()
    }

    /// Format the current ISO week (`YYYY-Www`) for a moment. Used
    /// when matching against weekly priorities.
    pub fn iso_week(now: DateTime<Utc>) -> String {
        let iso = now.iso_week();
        format!("{:04}-W{:02}", iso.year(), iso.week())
    }
}

// --- Gather payload shapes ---

#[derive(Debug, Clone, Serialize)]
struct DailyGather {
    date: String,
    iso_week: String,
    calendar_events: Vec<CalEvent>,
    rolling_todos: Vec<TodoRow>,
    attention_signals: Vec<SignalRow>,
    weekly_priorities: Vec<PriorityRow>,
}

#[derive(Debug, Clone, Serialize)]
struct TodoRow {
    todo_id: String,
    body: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct PriorityRow {
    id: String,
    body: String,
    rationale: String,
}

#[async_trait]
impl Ceremony for DailyCeremony {
    fn kind(&self) -> &'static str {
        "daily"
    }

    fn period_key(&self, now: DateTime<Utc>) -> String {
        Self::period_date(now)
    }

    fn period_window(
        &self,
        period_key: &str,
    ) -> Result<(DateTime<Utc>, DateTime<Utc>), CeremonyError> {
        let date = NaiveDate::parse_from_str(period_key, "%Y-%m-%d").map_err(|e| {
            CeremonyError::Other(format!("daily period_key '{period_key}' not a date: {e}"))
        })?;
        crate::local_window::day_window_utc(date, self.tz)
    }

    fn default_schedule(&self) -> CronSchedule {
        // Mon–Fri 07:00 local time. Engine wires cloacina with this
        // verbatim.
        CronSchedule::local("0 7 * * MON-FRI")
    }

    fn interactive_actions(&self) -> Vec<InteractiveAction> {
        Vec::new()
    }

    fn patterns(&self) -> Option<&dyn PatternDetector> {
        None
    }

    async fn gather(&self, ctx: &dyn CeremonyCtx) -> Result<GatheredFacts, CeremonyError> {
        let period_key = ctx.period_key().to_string();
        let date: NaiveDate = NaiveDate::parse_from_str(&period_key, "%Y-%m-%d").map_err(|e| {
            CeremonyError::Other(format!("daily period_key '{period_key}' not a date: {e}"))
        })?;
        // Derive the ISO week for weekly priorities lookup.
        let iso = date.iso_week();
        let iso_week = format!("{:04}-W{:02}", iso.year(), iso.week());

        // 1. Calendar — external source.
        let mut calendar_events = self.calendar_source.events_for(date).await?;
        calendar_events.truncate(CAP_CALENDAR);

        // 2. Attention signals inside the day's pinned window.
        //    Bounded query so back-dated dispatch gathers the correct
        //    day's signals rather than "the last 24h from now".
        let conn_handle = ctx
            .conn_handle()
            .ok_or_else(|| CeremonyError::Other("daily plugin requires EngineCtx".into()))?;
        let (win_start, win_end) = ctx.period_window();
        let attention_signals = self
            .attention_source
            .between(win_start, win_end, CAP_ATTENTION)
            .await?;

        // 3. Rolling todos — straight SQL.
        let rolling_todos: Vec<TodoRow> = {
            let conn = conn_handle
                .0
                .lock()
                .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
            let mut stmt = conn
                .prepare(
                    "SELECT todo_id, body, created_at FROM ceremony_todos_rolling \
                     WHERE done_at IS NULL ORDER BY created_at LIMIT ?1",
                )
                .map_err(|e| CeremonyError::Storage(format!("todos prepare: {e}")))?;
            let rows = stmt
                .query_map(params![CAP_TODOS as i64], |row| {
                    Ok(TodoRow {
                        todo_id: row.get(0)?,
                        body: row.get(1)?,
                        created_at: row.get(2)?,
                    })
                })
                .map_err(|e| CeremonyError::Storage(format!("todos query: {e}")))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| CeremonyError::Storage(format!("todos row: {e}")))?);
            }
            out
        };

        // 4. Weekly priorities — confirmed entries on this week's
        //    weekly tablet.
        let weekly_priorities: Vec<PriorityRow> = {
            let conn = conn_handle
                .0
                .lock()
                .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
            let mut stmt = conn
                .prepare(
                    "SELECT p.id, td.body, COALESCE(td.rationale, '') \
                     FROM ceremony_priorities p \
                     JOIN ceremony_tablets t ON p.tablet_id = t.id \
                     JOIN todos td ON td.id = p.todo_id \
                     WHERE t.kind = 'weekly' AND t.period_key = ?1 \
                           AND p.confirmed_at IS NOT NULL \
                     ORDER BY p.ordinal LIMIT ?2",
                )
                .map_err(|e| CeremonyError::Storage(format!("priorities prepare: {e}")))?;
            let rows = stmt
                .query_map(params![&iso_week, CAP_PRIORITIES as i64], |row| {
                    Ok(PriorityRow {
                        id: row.get(0)?,
                        body: row.get(1)?,
                        rationale: row.get(2)?,
                    })
                })
                .map_err(|e| CeremonyError::Storage(format!("priorities query: {e}")))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| CeremonyError::Storage(format!("priorities row: {e}")))?);
            }
            out
        };

        let payload = DailyGather {
            date: period_key.clone(),
            iso_week,
            calendar_events,
            rolling_todos,
            attention_signals,
            weekly_priorities,
        };
        let json = serde_json::to_value(&payload)
            .map_err(|e| CeremonyError::Other(format!("gather payload serialise: {e}")))?;
        Ok(GatheredFacts::new(json))
    }

    async fn compose(
        &self,
        ctx: &dyn CeremonyCtx,
        facts: GatheredFacts,
    ) -> Result<Vec<NewItem>, CeremonyError> {
        let tablet_id = ctx.tablet_id().to_string();

        // Build the registry of valid citation ids the LLM is allowed
        // to use. Anything outside this set is rejected before we
        // construct the ComposedItem — friendlier than letting the
        // engine roll back the whole tablet.
        let valid_ids = collect_valid_ids(&facts.payload);

        let prompt = build_compose_prompt(&facts, &valid_ids);
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
            if spec.citation_id.trim().is_empty() {
                return Err(CeremonyError::missing_citation(format!(
                    "compose item index {i} has empty citation_id"
                )));
            }
            if !valid_ids.contains(&spec.citation_id) {
                return Err(CeremonyError::missing_citation(format!(
                    "compose item index {i} cites unknown id '{}' (not in gather payload)",
                    spec.citation_id
                )));
            }
            if !is_valid_section(&spec.section_key) {
                return Err(CeremonyError::Llm(format!(
                    "compose item index {i} uses unknown section_key '{}'",
                    spec.section_key
                )));
            }
            out.push(NewItem::composed(ComposedItem {
                tablet_id: tablet_id.clone(),
                section_key: spec.section_key,
                ordinal: i as i32,
                kind: ItemKind::Freeform,
                body: spec.body,
                citation_id: spec.citation_id,
            }));
        }
        Ok(out)
    }
}

const SYSTEM_PROMPT: &str = "\
You are arawn's daily composer. Given a JSON payload describing the user's day, \
return a JSON array of items — one per claim. Each item: \
{\"section_key\": \"calendar\" | \"todos\" | \"attention\" | \"alignment\", \
\"citation_id\": \"<row id from the payload>\", \
\"body\": {\"text\": \"<one-sentence claim>\"}}. Never fabricate a citation_id \
that isn't already in the payload. Be concise and grounded.";

fn build_compose_prompt(facts: &GatheredFacts, valid_ids: &HashSet<String>) -> String {
    let mut ids: Vec<&String> = valid_ids.iter().collect();
    ids.sort();
    format!(
        "Compose the daily for {}.\n\nValid citation_ids: {:?}\n\nPayload:\n{}",
        facts
            .payload
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        ids,
        serde_json::to_string_pretty(&facts.payload).unwrap_or_default()
    )
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ComposedItemSpec {
    section_key: String,
    citation_id: String,
    body: serde_json::Value,
}

fn is_valid_section(s: &str) -> bool {
    matches!(s, "calendar" | "todos" | "attention" | "alignment")
}

/// Walk the gather payload and collect every id field the compose
/// phase may legitimately cite. Matches `id` for calendar events,
/// attention signals, and weekly priorities, plus `todo_id` for
/// rolling todos.
fn collect_valid_ids(payload: &serde_json::Value) -> HashSet<String> {
    let mut out = HashSet::new();
    for (key, id_field) in [
        ("calendar_events", "id"),
        ("attention_signals", "id"),
        ("weekly_priorities", "id"),
        ("rolling_todos", "todo_id"),
    ] {
        if let Some(arr) = payload.get(key).and_then(|v| v.as_array()) {
            for row in arr {
                if let Some(id) = row.get(id_field).and_then(|v| v.as_str()) {
                    out.insert(id.to_string());
                }
            }
        }
    }
    out
}

/// Pull the first balanced JSON array out of the LLM's response.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CeremonyDispatcher;
    use crate::PluginRegistry;
    use crate::engine::{ConnHandle, EngineCtx, EngineDispatcher};
    use crate::plugins::gather_sources::{
        NoopCalendarSource, StaticAttentionSource, StaticCalendarSource,
    };
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

    fn seed_daily_history(conn: &ConnHandle, period_key: &str, iso_week: &str) {
        let c = conn.0.lock().unwrap();
        // Origin daily tablet so rolling todos have somewhere to
        // reference (foreign key on ceremony_todos_rolling).
        c.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, lenses_scanned) \
             VALUES (?1, 'daily', ?2, ?3, 'reviewed', '[]')",
            params!["daily-origin", "2026-05-10", "2026-05-10T07:00:00Z"],
        )
        .unwrap();
        // Post-V9: rolling todos live in `todos` (view exposes them
        // back as ceremony_todos_rolling for reads).
        c.execute(
            "INSERT INTO todos (id, body, rationale, kind, lens, created_at, \
                                due_at, done_at, archived_at, attrs) \
             VALUES ('todo-1', 'Finish daily plugin', NULL, 'rollover', NULL, \
                     '2026-05-10T07:00:00Z', NULL, NULL, NULL, \
                     json_object('origin_tablet_id','daily-origin','last_seen_tablet_id','daily-origin'))",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO todos (id, body, rationale, kind, lens, created_at, \
                                due_at, done_at, archived_at, attrs) \
             VALUES ('todo-2', 'Write tests', NULL, 'rollover', NULL, \
                     '2026-05-11T07:00:00Z', NULL, NULL, NULL, \
                     json_object('origin_tablet_id','daily-origin','last_seen_tablet_id','daily-origin'))",
            [],
        )
        .unwrap();
        // Weekly tablet + confirmed priority for this iso_week.
        c.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, lenses_scanned) \
             VALUES (?1, 'weekly', ?2, ?3, 'reviewed', '[]')",
            params!["weekly-tablet", iso_week, "2026-05-11T07:00:00Z"],
        )
        .unwrap();
        c.execute(
            "INSERT INTO todos (id, body, rationale, kind, lens, created_at, \
                                due_at, done_at, archived_at, attrs) \
             VALUES ('td-prio-1', 'Ship daily plugin', 'from last retro', 'weekly_priority', \
                     NULL, '2026-05-11T08:00:00Z', NULL, NULL, NULL, '{}')",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO ceremony_priorities (id, tablet_id, todo_id, confirmed_at, ordinal) \
             VALUES (?1, 'weekly-tablet', 'td-prio-1', ?2, 0)",
            params!["prio-1", "2026-05-11T08:00:00Z"],
        )
        .unwrap();
        // Ensure the period_key under test doesn't already have a
        // daily tablet — drop the origin's idempotency conflict by
        // using a different period_key on the seed (already 2026-05-10).
        let _ = period_key;
    }

    fn sample_calendar_events() -> Vec<CalEvent> {
        vec![
            CalEvent {
                id: "evt-1".into(),
                title: "Standup".into(),
                start: DateTime::parse_from_rfc3339("2026-05-15T14:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                end: DateTime::parse_from_rfc3339("2026-05-15T14:15:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                attendees: vec!["alice@example.com".into()],
                body_excerpt: None,
            },
            CalEvent {
                id: "evt-2".into(),
                title: "Design review".into(),
                start: DateTime::parse_from_rfc3339("2026-05-15T16:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                end: DateTime::parse_from_rfc3339("2026-05-15T17:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                attendees: vec!["bob@example.com".into()],
                body_excerpt: Some("Discuss the daily plugin".into()),
            },
        ]
    }

    fn sample_signals(ts: DateTime<Utc>) -> Vec<SignalRow> {
        vec![SignalRow {
            id: "sig-1".into(),
            source_kind: "email".into(),
            source_id: "msg-42".into(),
            ts,
            summary: "Urgent reply requested".into(),
            lens: Some("proj-a".into()),
        }]
    }

    #[tokio::test]
    async fn period_key_formats_yyyy_mm_dd() {
        let dt = DateTime::parse_from_rfc3339("2026-05-15T16:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(DailyCeremony::period_date(dt), "2026-05-15");
    }

    #[tokio::test]
    async fn gather_collects_four_section_payload() {
        let (_tmp, conn) = open_test_db();
        seed_daily_history(&conn, "2026-05-15", "2026-W20");
        let sig_ts = DateTime::parse_from_rfc3339("2026-05-15T06:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let plugin = DailyCeremony::new(
            make_llm_with_response("[]"),
            "test-model",
            Arc::new(StaticCalendarSource(sample_calendar_events())),
            Arc::new(StaticAttentionSource(sample_signals(sig_ts))),
        );
        let day_start = DateTime::parse_from_rfc3339("2026-05-15T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let day_end = day_start + chrono::Duration::days(1);
        let ctx = EngineCtx::new(
            conn.clone(),
            "daily-2026-05-15".into(),
            "2026-05-15".into(),
            (day_start, day_end),
        );
        let facts = plugin.gather(&ctx).await.unwrap();
        let p = &facts.payload;
        assert_eq!(p.get("date").unwrap().as_str().unwrap(), "2026-05-15");
        assert_eq!(p.get("iso_week").unwrap().as_str().unwrap(), "2026-W20");
        assert_eq!(
            p.get("calendar_events").unwrap().as_array().unwrap().len(),
            2
        );
        assert_eq!(p.get("rolling_todos").unwrap().as_array().unwrap().len(), 2);
        assert_eq!(
            p.get("attention_signals")
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            p.get("weekly_priorities")
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn compose_rejects_unknown_citation() {
        let (_tmp, conn) = open_test_db();
        seed_daily_history(&conn, "2026-05-15", "2026-W20");
        let llm = r#"[
            {"section_key": "todos", "citation_id": "ghost-id",
             "body": {"text": "made up"}}
        ]"#;
        let plugin = DailyCeremony::new(
            make_llm_with_response(llm),
            "test-model",
            Arc::new(NoopCalendarSource),
            Arc::new(StaticAttentionSource(Vec::new())),
        );
        let ctx = EngineCtx::for_test(conn.clone(), "daily-2026-05-15".into(), "2026-05-15".into());
        let facts = plugin.gather(&ctx).await.unwrap();
        let err = plugin.compose(&ctx, facts).await.unwrap_err();
        assert!(matches!(err, CeremonyError::MissingCitation(_)));
    }

    #[tokio::test]
    async fn compose_rejects_empty_citation() {
        let (_tmp, conn) = open_test_db();
        seed_daily_history(&conn, "2026-05-15", "2026-W20");
        let llm = r#"[
            {"section_key": "todos", "citation_id": "",
             "body": {"text": "no source"}}
        ]"#;
        let plugin = DailyCeremony::new(
            make_llm_with_response(llm),
            "test-model",
            Arc::new(NoopCalendarSource),
            Arc::new(StaticAttentionSource(Vec::new())),
        );
        let ctx = EngineCtx::for_test(conn.clone(), "daily-2026-05-15".into(), "2026-05-15".into());
        let facts = plugin.gather(&ctx).await.unwrap();
        let err = plugin.compose(&ctx, facts).await.unwrap_err();
        assert!(matches!(err, CeremonyError::MissingCitation(_)));
    }

    #[tokio::test]
    async fn end_to_end_dispatch_writes_tablet_and_items() {
        let (_tmp, conn) = open_test_db();
        // Seed history. The period_key the dispatcher computes is
        // "today's UTC date" so we pin against that.
        let today = Utc::now();
        let period_key = DailyCeremony::period_date(today);
        let iso_week = DailyCeremony::iso_week(today);
        seed_daily_history(&conn, &period_key, &iso_week);

        // Compose returns one item per section, each citing a real
        // gathered id.
        let llm_response = r#"[
            {"section_key": "calendar",  "citation_id": "evt-1",  "body": {"text": "Standup at 14:00."}},
            {"section_key": "todos",     "citation_id": "todo-1", "body": {"text": "Continue plugin work."}},
            {"section_key": "attention", "citation_id": "sig-1",  "body": {"text": "Respond to urgent email."}},
            {"section_key": "alignment", "citation_id": "prio-1", "body": {"text": "Ties to weekly priority."}}
        ]"#;
        // Pin the attention signal inside today's window so the
        // bounded `between` query picks it up. The dispatcher derives
        // the window from `period_window(today)`.
        let plugin = Arc::new(DailyCeremony::new(
            make_llm_with_response(llm_response),
            "test-model",
            Arc::new(StaticCalendarSource(sample_calendar_events())),
            Arc::new(StaticAttentionSource(sample_signals(
                today - chrono::Duration::minutes(5),
            ))),
        ));
        let reg = PluginRegistry::new();
        reg.register(plugin).unwrap();
        let dispatcher = EngineDispatcher::new(conn.clone(), reg);
        let outcome = dispatcher.dispatch("daily").await.unwrap();
        assert!(matches!(outcome, crate::DispatchOutcome::Generated { .. }));

        let c = conn.0.lock().unwrap();
        let n_tablets: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM ceremony_tablets WHERE kind = 'daily' AND period_key = ?1",
                params![&period_key],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n_tablets, 1);
        // Four items, one per section.
        for section in ["calendar", "todos", "attention", "alignment"] {
            let n: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM ceremony_items i \
                     JOIN ceremony_tablets t ON i.tablet_id = t.id \
                     WHERE t.kind = 'daily' AND t.period_key = ?1 \
                           AND i.section_key = ?2 AND i.citation_id IS NOT NULL",
                    params![&period_key, section],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "expected one item in section {section}");
        }
    }

    #[test]
    fn period_window_utc_matches_calendar_day() {
        let daily = DailyCeremony::new(
            make_llm_with_response("{}"),
            "stub-model",
            Arc::new(crate::plugins::NoopCalendarSource),
            Arc::new(crate::plugins::StaticAttentionSource(Vec::new())),
        );
        let (start, end) = daily.period_window("2026-05-19").unwrap();
        assert_eq!(start.to_rfc3339(), "2026-05-19T00:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-05-20T00:00:00+00:00");
    }

    #[test]
    fn period_window_pacific_offsets_correctly() {
        let daily = DailyCeremony::new(
            make_llm_with_response("{}"),
            "stub-model",
            Arc::new(crate::plugins::NoopCalendarSource),
            Arc::new(crate::plugins::StaticAttentionSource(Vec::new())),
        )
        .with_timezone(chrono_tz::America::Los_Angeles);
        let (start, end) = daily.period_window("2026-05-19").unwrap();
        // PDT = UTC-7 in May.
        assert_eq!(start.to_rfc3339(), "2026-05-19T07:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-05-20T07:00:00+00:00");
    }

    #[test]
    fn period_window_rejects_non_date() {
        let daily = DailyCeremony::new(
            make_llm_with_response("{}"),
            "stub-model",
            Arc::new(crate::plugins::NoopCalendarSource),
            Arc::new(crate::plugins::StaticAttentionSource(Vec::new())),
        );
        assert!(daily.period_window("not-a-date").is_err());
        assert!(daily.period_window("2026-W20").is_err());
    }
}
