//! Monday weekly ceremony plugin.
//!
//! Runs Monday 07:00 local (default). Five sections:
//!
//! 1. **priorities** — 5–7 candidate priorities for the user to
//!    confirm. Compose emits these with `kind = priority` in body;
//!    the confirmation flow (T-0302) writes the actual
//!    `ceremony_priorities` rows.
//! 2. **calendar_shape** — meeting count, deep-work hours, busiest
//!    day, free afternoons across Mon..Sun. Citation is a synthetic
//!    `summary-<iso_week>` row.
//! 3. **deadlines** — attention signals since Monday with
//!    deadline-flavoured summaries.
//! 4. **from_last_retro** — diary excerpts + top patterns from the
//!    most recent retro tablet.
//! 5. **inbound** — open items from the previous weekly tablet that
//!    were not confirmed as priorities.
//!
//! Also surfaces "hot" rolling todos (created > 7d ago, still open)
//! as candidate citations.
//!
//! All gather data is JSON-payload-fed to the LLM with a strict
//! citation registry; compose output is validated before any
//! `NewItem::Composed` is constructed.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use futures::StreamExt;
use rusqlite::params;
use serde::Serialize;

use crate::CeremonyError;
use crate::plugin::{
    Ceremony, CeremonyCtx, ComposedItem, CronSchedule, InteractiveAction, NewItem,
    PatternDetector,
};
use crate::plugins::gather_sources::{
    AttentionSource, CalendarSource, SignalRow,
};
use crate::types::{GatheredFacts, ItemKind};

const CAP_DEADLINES: usize = 10;
const CAP_LAST_RETRO: usize = 4;
const CAP_PRIOR_WEEKLY: usize = 10;
const CAP_ROLLING_HOT: usize = 10;
const ROLLING_HOT_AGE_DAYS: i64 = 7;
const WORKDAY_HOURS_PER_DAY: f64 = 8.0;
const AFTERNOON_HOUR_UTC: u32 = 13;

/// The weekly plugin.
pub struct WeeklyCeremony {
    llm: Arc<dyn arawn_llm::LlmClient>,
    /// Concrete model string the binary resolved (typically from a
    /// `hint:medium` taxonomy entry). Plugin doesn't route; it hands
    /// this through to the LLM client.
    model: String,
    calendar_source: Arc<dyn CalendarSource>,
    attention_source: Arc<dyn AttentionSource>,
}

impl WeeklyCeremony {
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
        }
    }
}

/// Format the ISO-week string (`YYYY-Www`) for a moment. Exposed so
/// callers + tests can predict the tablet id without running the
/// plugin.
pub fn iso_week(now: DateTime<Utc>) -> String {
    let iso = now.iso_week();
    format!("{:04}-W{:02}", iso.year(), iso.week())
}

// --- Gather payload shapes ---

#[derive(Debug, Clone, Serialize)]
struct WeeklyGather {
    iso_week: String,
    calendar_summary: Vec<CalendarSummary>,
    deadlines: Vec<SignalRow>,
    last_retro_excerpts: Vec<RetroExcerpt>,
    prior_weekly_inbound: Vec<InboundRow>,
    rolling_todo_hot: Vec<HotTodoRow>,
}

#[derive(Debug, Clone, Serialize)]
struct CalendarSummary {
    id: String,
    iso_week: String,
    meeting_count: u32,
    deep_work_hours: f64,
    busiest_day_label: String,
    free_afternoons: u32,
}

#[derive(Debug, Clone, Serialize)]
struct RetroExcerpt {
    id: String,
    kind: &'static str, // "diary" | "pattern"
    body: String,
}

#[derive(Debug, Clone, Serialize)]
struct InboundRow {
    id: String,
    body: String,
}

#[derive(Debug, Clone, Serialize)]
struct HotTodoRow {
    todo_id: String,
    body: String,
}

#[async_trait]
impl Ceremony for WeeklyCeremony {
    fn kind(&self) -> &'static str {
        "weekly"
    }

    fn period_key(&self, now: DateTime<Utc>) -> String {
        iso_week(now)
    }

    fn default_schedule(&self) -> CronSchedule {
        CronSchedule::local("0 7 * * MON")
    }

    fn interactive_actions(&self) -> Vec<InteractiveAction> {
        Vec::new()
    }

    fn patterns(&self) -> Option<&dyn PatternDetector> {
        None
    }

    async fn gather(&self, ctx: &dyn CeremonyCtx) -> Result<GatheredFacts, CeremonyError> {
        let period_key = ctx.period_key().to_string();
        let (mon_date, sun_date) = monday_sunday_for_iso_week(&period_key).ok_or_else(|| {
            CeremonyError::Other(format!("weekly period_key '{period_key}' not an iso week"))
        })?;

        // 1. Calendar summary — aggregate over Mon..Sun.
        let mut meeting_count: u32 = 0;
        let mut total_meeting_hours: f64 = 0.0;
        let mut per_day_counts: [(Weekday, u32); 7] = [
            (Weekday::Mon, 0),
            (Weekday::Tue, 0),
            (Weekday::Wed, 0),
            (Weekday::Thu, 0),
            (Weekday::Fri, 0),
            (Weekday::Sat, 0),
            (Weekday::Sun, 0),
        ];
        let mut free_afternoons: u32 = 0;

        let mut day = mon_date;
        let mut day_idx = 0usize;
        while day <= sun_date && day_idx < 7 {
            let events = self.calendar_source.events_for(day).await?;
            let count = events.len() as u32;
            meeting_count += count;
            per_day_counts[day_idx].1 = count;
            for e in &events {
                let dur = (e.end - e.start).num_minutes().max(0) as f64 / 60.0;
                total_meeting_hours += dur;
            }
            // Free afternoon: no event starting at or after 13:00 UTC.
            let has_afternoon = events
                .iter()
                .any(|e| e.start.hour() >= AFTERNOON_HOUR_UTC);
            if !has_afternoon {
                free_afternoons += 1;
            }
            day = day.succ_opt().unwrap_or(day);
            day_idx += 1;
        }

        // Clamp deep-work to 8h/day workday window over 7 days.
        let total_workday_hours = WORKDAY_HOURS_PER_DAY * 7.0;
        let deep_work_hours =
            (total_workday_hours - total_meeting_hours).clamp(0.0, total_workday_hours);

        let busiest_day_label = per_day_counts
            .iter()
            .max_by_key(|(_, c)| *c)
            .map(|(wd, _)| weekday_label(*wd).to_string())
            .unwrap_or_else(|| "Mon".to_string());

        let summary_id = format!("summary-{period_key}");
        let calendar_summary = vec![CalendarSummary {
            id: summary_id.clone(),
            iso_week: period_key.clone(),
            meeting_count,
            deep_work_hours,
            busiest_day_label,
            free_afternoons,
        }];

        // 2. Deadlines — attention signals since Monday, filtered to
        //    deadline-flavoured summaries (fallback: most recent).
        let monday_dt: DateTime<Utc> = mon_date
            .and_hms_opt(0, 0, 0)
            .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
            .unwrap_or_else(|| Utc::now() - Duration::days(7));
        let raw_signals = self
            .attention_source
            .since(monday_dt, CAP_DEADLINES * 4)
            .await?;
        let mut filtered: Vec<SignalRow> = raw_signals
            .iter()
            .filter(|r| is_deadline_flavoured(&r.summary))
            .cloned()
            .collect();
        if filtered.is_empty() {
            filtered = raw_signals.into_iter().take(CAP_DEADLINES).collect();
        } else {
            filtered.truncate(CAP_DEADLINES);
        }
        let deadlines = filtered;

        // 3-5. SQL-backed sources.
        let conn_handle = ctx
            .conn_handle()
            .ok_or_else(|| CeremonyError::Other("weekly plugin requires EngineCtx".into()))?;

        // 3. Last retro excerpts — diary body + top 3 patterns for
        //    the most-recent retro tablet (strictly before this week).
        let last_retro_excerpts: Vec<RetroExcerpt> = {
            let conn = conn_handle
                .0
                .lock()
                .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
            // Most-recent retro tablet whose period_key < this week.
            let last: Option<(String, String)> = conn
                .query_row(
                    "SELECT id, period_key FROM ceremony_tablets \
                     WHERE kind = 'retro' AND period_key < ?1 \
                     ORDER BY period_key DESC LIMIT 1",
                    params![&period_key],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .ok();
            let mut out: Vec<RetroExcerpt> = Vec::new();
            if let Some((tablet_id, retro_week)) = last {
                // Diary body.
                let diary: Option<String> = conn
                    .query_row(
                        "SELECT body FROM ceremony_diary WHERE tablet_id = ?1",
                        params![&tablet_id],
                        |row| row.get(0),
                    )
                    .ok();
                if let Some(body) = diary {
                    out.push(RetroExcerpt {
                        id: format!("diary-{tablet_id}"),
                        kind: "diary",
                        body: body.chars().take(400).collect(),
                    });
                }
                // Top 3 patterns for that retro's iso_week.
                let mut stmt = conn
                    .prepare(
                        "SELECT id, pattern_key, magnitude FROM ceremony_patterns_detected \
                         WHERE iso_week = ?1 ORDER BY magnitude DESC LIMIT 3",
                    )
                    .map_err(|e| CeremonyError::Storage(format!("patterns prepare: {e}")))?;
                let rows = stmt
                    .query_map(params![&retro_week], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, f64>(2)?,
                        ))
                    })
                    .map_err(|e| CeremonyError::Storage(format!("patterns query: {e}")))?;
                for r in rows {
                    let (pid, key, mag) = r
                        .map_err(|e| CeremonyError::Storage(format!("patterns row: {e}")))?;
                    out.push(RetroExcerpt {
                        id: format!("pattern-{pid}"),
                        kind: "pattern",
                        body: format!("{key} (magnitude {mag:.2})"),
                    });
                }
            }
            out.truncate(CAP_LAST_RETRO);
            out
        };

        // 4. Prior weekly inbound — open items from the most-recent
        //    weekly tablet (strictly before this week).
        let prior_weekly_inbound: Vec<InboundRow> = {
            let conn = conn_handle
                .0
                .lock()
                .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
            let prior_tablet: Option<String> = conn
                .query_row(
                    "SELECT id FROM ceremony_tablets \
                     WHERE kind = 'weekly' AND period_key < ?1 \
                     ORDER BY period_key DESC LIMIT 1",
                    params![&period_key],
                    |row| row.get(0),
                )
                .ok();
            let mut out: Vec<InboundRow> = Vec::new();
            if let Some(tablet_id) = prior_tablet {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, body FROM ceremony_items \
                         WHERE tablet_id = ?1 AND done_at IS NULL \
                         ORDER BY ordinal LIMIT ?2",
                    )
                    .map_err(|e| CeremonyError::Storage(format!("inbound prepare: {e}")))?;
                let rows = stmt
                    .query_map(params![&tablet_id, CAP_PRIOR_WEEKLY as i64], |row| {
                        Ok(InboundRow {
                            id: row.get(0)?,
                            body: row.get(1)?,
                        })
                    })
                    .map_err(|e| CeremonyError::Storage(format!("inbound query: {e}")))?;
                for r in rows {
                    out.push(
                        r.map_err(|e| CeremonyError::Storage(format!("inbound row: {e}")))?,
                    );
                }
            }
            out
        };

        // 5. Rolling todo hot — open, created > 7d ago.
        let cutoff = Utc::now() - Duration::days(ROLLING_HOT_AGE_DAYS);
        let cutoff_str = cutoff.to_rfc3339();
        let rolling_todo_hot: Vec<HotTodoRow> = {
            let conn = conn_handle
                .0
                .lock()
                .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
            let mut stmt = conn
                .prepare(
                    "SELECT todo_id, body FROM ceremony_todos_rolling \
                     WHERE done_at IS NULL AND created_at < ?1 \
                     ORDER BY created_at LIMIT ?2",
                )
                .map_err(|e| CeremonyError::Storage(format!("hot todos prepare: {e}")))?;
            let rows = stmt
                .query_map(params![&cutoff_str, CAP_ROLLING_HOT as i64], |row| {
                    Ok(HotTodoRow {
                        todo_id: row.get(0)?,
                        body: row.get(1)?,
                    })
                })
                .map_err(|e| CeremonyError::Storage(format!("hot todos query: {e}")))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(
                    r.map_err(|e| CeremonyError::Storage(format!("hot todos row: {e}")))?,
                );
            }
            out
        };

        let payload = WeeklyGather {
            iso_week: period_key,
            calendar_summary,
            deadlines,
            last_retro_excerpts,
            prior_weekly_inbound,
            rolling_todo_hot,
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
            if !is_valid_section(&spec.section) {
                return Err(CeremonyError::Llm(format!(
                    "compose item index {i} uses unknown section '{}'",
                    spec.section
                )));
            }
            out.push(NewItem::composed(ComposedItem {
                tablet_id: tablet_id.clone(),
                section_key: spec.section,
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
You are arawn's weekly composer. Given a JSON payload describing the user's \
upcoming week (calendar shape, deadlines, last retro, last weekly inbound, hot \
rolling todos), return a JSON array of items — one per claim. Each item: \
{\"section\": \"priorities\" | \"calendar_shape\" | \"deadlines\" | \
\"from_last_retro\" | \"inbound\", \"citation_id\": \"<row id from the payload>\", \
\"body\": {\"text\": \"<one-sentence claim>\"}}. The priorities section should \
have 5-7 candidates. Never fabricate a citation_id that isn't in the payload. \
Be concise and grounded.";

fn build_compose_prompt(facts: &GatheredFacts, valid_ids: &HashSet<String>) -> String {
    let mut ids: Vec<&String> = valid_ids.iter().collect();
    ids.sort();
    format!(
        "Compose the weekly for ISO week {}.\n\nValid citation_ids: {:?}\n\nPayload:\n{}",
        facts
            .payload
            .get("iso_week")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        ids,
        serde_json::to_string_pretty(&facts.payload).unwrap_or_default()
    )
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ComposedItemSpec {
    section: String,
    citation_id: String,
    body: serde_json::Value,
}

fn is_valid_section(s: &str) -> bool {
    matches!(
        s,
        "priorities" | "calendar_shape" | "deadlines" | "from_last_retro" | "inbound"
    )
}

fn is_deadline_flavoured(summary: &str) -> bool {
    let s = summary.to_lowercase();
    s.contains("due")
        || s.contains("deadline")
        || s.contains("by eod")
        || s.contains("by end of")
        || s.contains("expires")
}

fn weekday_label(wd: Weekday) -> &'static str {
    match wd {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

/// Walk the gather payload and collect every id the compose phase may
/// legitimately cite.
fn collect_valid_ids(payload: &serde_json::Value) -> HashSet<String> {
    let mut out = HashSet::new();
    for (key, id_field) in [
        ("calendar_summary", "id"),
        ("deadlines", "id"),
        ("last_retro_excerpts", "id"),
        ("prior_weekly_inbound", "id"),
        ("rolling_todo_hot", "todo_id"),
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

/// Compute Monday and Sunday `YYYY-MM-DD` dates bracketing an ISO
/// week. Returns `None` when the input is malformed.
fn monday_sunday_for_iso_week(iso_week_str: &str) -> Option<(NaiveDate, NaiveDate)> {
    let parts: Vec<&str> = iso_week_str.split("-W").collect();
    if parts.len() != 2 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let week: u32 = parts[1].parse().ok()?;
    let monday = NaiveDate::from_isoywd_opt(year, week, Weekday::Mon)?;
    let sunday = NaiveDate::from_isoywd_opt(year, week, Weekday::Sun)?;
    Some((monday, sunday))
}

// `Timelike` is needed for `hour()` on DateTime<Utc>.
use chrono::Timelike;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CeremonyDispatcher;
    use crate::PluginRegistry;
    use crate::engine::{ConnHandle, EngineDispatcher};
    use crate::plugins::gather_sources::{
        CalEvent, StaticAttentionSource,
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

    fn sample_calendar_events_for(date: NaiveDate) -> Vec<CalEvent> {
        let start = date
            .and_hms_opt(14, 0, 0)
            .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
            .unwrap();
        let end = date
            .and_hms_opt(15, 0, 0)
            .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
            .unwrap();
        vec![CalEvent {
            id: format!("evt-{}", date),
            title: "Sync".into(),
            start,
            end,
            attendees: vec!["alice@example.com".into()],
            body_excerpt: None,
        }]
    }

    /// Calendar source that returns one afternoon event per day.
    struct PerDayCalendar;

    #[async_trait]
    impl CalendarSource for PerDayCalendar {
        async fn events_for(
            &self,
            date: NaiveDate,
        ) -> Result<Vec<CalEvent>, CeremonyError> {
            Ok(sample_calendar_events_for(date))
        }
    }

    fn sample_signals(monday: NaiveDate) -> Vec<SignalRow> {
        let ts = monday
            .and_hms_opt(10, 0, 0)
            .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
            .unwrap();
        vec![SignalRow {
            id: "sig-due-1".into(),
            source_kind: "email".into(),
            source_id: "msg-77".into(),
            ts,
            summary: "Tax form due Friday".into(),
            workstream: Some("personal".into()),
        }]
    }

    fn seed_weekly_history(conn: &ConnHandle, this_iso_week: &str) {
        let c = conn.0.lock().unwrap();
        // Prior retro tablet + diary + a pattern row.
        c.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
             VALUES (?1, 'retro', ?2, ?3, 'reviewed', '[]')",
            params!["retro-prev", "2026-W19", "2026-05-08T16:00:00Z"],
        )
        .unwrap();
        c.execute(
            "INSERT INTO ceremony_diary (tablet_id, body, written_at, word_count) \
             VALUES (?1, ?2, ?3, ?4)",
            params![
                "retro-prev",
                "Felt scattered; meetings ate the deep-work blocks.",
                "2026-05-08T18:00:00Z",
                10
            ],
        )
        .unwrap();
        c.execute(
            "INSERT INTO ceremony_patterns_detected (id, iso_week, pattern_key, magnitude, payload, surfaced_in_retro) \
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            params!["pat-1", "2026-W19", "rollover_heat", 0.75, "{}"],
        )
        .unwrap();
        // Prior weekly tablet + an open inbound item.
        c.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
             VALUES (?1, 'weekly', ?2, ?3, 'reviewed', '[]')",
            params!["weekly-prev", "2026-W19", "2026-05-04T07:00:00Z"],
        )
        .unwrap();
        c.execute(
            "INSERT INTO ceremony_items (id, tablet_id, section_key, ordinal, kind, body, citation_id, created_at) \
             VALUES (?1, ?2, ?3, 0, 'freeform', ?4, NULL, ?5)",
            params![
                "wk-prev-item-1",
                "weekly-prev",
                "inbound",
                "{\"text\":\"follow up with vendor\"}",
                "2026-05-04T07:00:00Z",
            ],
        )
        .unwrap();
        // A daily tablet (only used as FK target for rolling todos).
        c.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
             VALUES (?1, 'daily', ?2, ?3, 'reviewed', '[]')",
            params!["daily-origin", "2026-05-04", "2026-05-04T07:00:00Z"],
        )
        .unwrap();
        // A hot rolling todo created > 7d ago.
        let old = (Utc::now() - Duration::days(14)).to_rfc3339();
        c.execute(
            "INSERT INTO ceremony_todos_rolling (todo_id, body, origin_tablet_id, created_at, done_at, last_seen_tablet_id) \
             VALUES (?1, ?2, 'daily-origin', ?3, NULL, 'daily-origin')",
            params!["hot-todo-1", "Wire OAuth refresh", old],
        )
        .unwrap();
        // Make sure this week doesn't pre-exist.
        let _ = this_iso_week;
    }

    #[tokio::test]
    async fn iso_week_format_is_yyyy_w_ww() {
        let dt = DateTime::parse_from_rfc3339("2026-05-15T16:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(iso_week(dt), "2026-W20");
    }

    #[tokio::test]
    async fn end_to_end_dispatch_writes_tablet_and_items() {
        let (_tmp, conn) = open_test_db();
        let today = Utc::now();
        let period_key = iso_week(today);
        seed_weekly_history(&conn, &period_key);

        let (mon, _sun) = monday_sunday_for_iso_week(&period_key).unwrap();

        // Compose returns one item per section. Each citation_id will
        // be filled in dynamically after we know the gather output —
        // but we can predict the stable ids:
        //   - calendar_summary: "summary-<iso_week>"
        //   - deadlines:        "sig-due-1"
        //   - from_last_retro:  "diary-retro-prev"
        //   - inbound:          "wk-prev-item-1"
        //   - rolling hot:      "hot-todo-1"
        // Priorities can cite any of these; we use the hot todo.
        let llm_response = format!(
            r#"[
                {{"section": "priorities",      "citation_id": "hot-todo-1",       "body": {{"text": "Wire OAuth refresh this week."}}}},
                {{"section": "calendar_shape",  "citation_id": "summary-{period_key}", "body": {{"text": "7 meetings, mostly afternoons."}}}},
                {{"section": "deadlines",       "citation_id": "sig-due-1",         "body": {{"text": "Tax form due Friday."}}}},
                {{"section": "from_last_retro", "citation_id": "diary-retro-prev",  "body": {{"text": "Scattered last week — protect mornings."}}}},
                {{"section": "inbound",         "citation_id": "wk-prev-item-1",    "body": {{"text": "Vendor follow-up still open."}}}}
            ]"#
        );

        let plugin = Arc::new(WeeklyCeremony::new(
            make_llm_with_response(&llm_response),
            "test-model",
            Arc::new(PerDayCalendar),
            Arc::new(StaticAttentionSource(sample_signals(mon))),
        ));
        let reg = PluginRegistry::new();
        reg.register(plugin).unwrap();
        let dispatcher = EngineDispatcher::new(conn.clone(), reg);
        let outcome = dispatcher.dispatch("weekly").await.unwrap();
        assert!(
            matches!(outcome, crate::DispatchOutcome::Generated { .. }),
            "expected Generated, got {outcome:?}"
        );

        let c = conn.0.lock().unwrap();
        let n_tablets: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM ceremony_tablets WHERE kind = 'weekly' AND period_key = ?1",
                params![&period_key],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n_tablets, 1);
        for section in [
            "priorities",
            "calendar_shape",
            "deadlines",
            "from_last_retro",
            "inbound",
        ] {
            let n: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM ceremony_items i \
                     JOIN ceremony_tablets t ON i.tablet_id = t.id \
                     WHERE t.kind = 'weekly' AND t.period_key = ?1 \
                           AND i.section_key = ?2 AND i.citation_id IS NOT NULL \
                           AND i.citation_id != ''",
                    params![&period_key, section],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "expected one item in section {section}");
        }
    }
}
