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
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use futures::StreamExt;
use rusqlite::params;
use serde::Serialize;

use crate::CeremonyError;
use crate::engine::ConnHandle;
use crate::patterns::DetectorRegistry;
use crate::plugin::{
    Ceremony, CeremonyCtx, ComposedItem, CronSchedule, InteractiveAction, NewItem, PatternDetector,
};
use crate::types::{GatheredFacts, ItemKind};

/// How often retro should run. Defaults to weekly; the binary
/// overrides via [`RetroCeremony::with_cadence`] from
/// `[ceremonies.retro] cadence` or from a runtime override stored
/// in the `ceremony_config` table by the `retro_set_cadence` agent
/// tool.
///
/// Biweekly and monthly cadences anchor on the first time retro
/// runs after enablement — the anchor date is persisted in
/// `ceremony_config` so the cycle is stable across restarts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetroCadence {
    Weekly,
    Biweekly,
    Monthly,
}

impl RetroCadence {
    /// Parse a config string. Case-insensitive. Returns `None` for
    /// unrecognised values so callers can warn-and-fall-back rather
    /// than abort startup.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "weekly" => Some(Self::Weekly),
            "biweekly" | "bi-weekly" | "fortnightly" => Some(Self::Biweekly),
            "monthly" => Some(Self::Monthly),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Weekly => "weekly",
            Self::Biweekly => "biweekly",
            Self::Monthly => "monthly",
        }
    }
}

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
    /// Timezone used to interpret `period_key` weeks when computing
    /// pinned windows. Defaults to UTC.
    tz: chrono_tz::Tz,
    /// Cadence: weekly (default) / biweekly / monthly. Biweekly +
    /// monthly require `anchor` to be set.
    cadence: RetroCadence,
    /// Anchor date for biweekly cadence (the first Monday a retro
    /// runs under the new cadence). `None` for weekly. Monthly
    /// doesn't need an anchor — months are absolute.
    anchor: Option<NaiveDate>,
}

impl RetroCeremony {
    pub fn new(llm: Arc<dyn arawn_llm::LlmClient>, model: impl Into<String>) -> Self {
        Self {
            llm,
            model: model.into(),
            detectors: DetectorRegistry::new(),
            tz: chrono_tz::UTC,
            cadence: RetroCadence::Weekly,
            anchor: None,
        }
    }

    /// Attach the detector registry (typically the v1 catalog
    /// from T-0288). Chainable.
    pub fn with_detectors(mut self, detectors: DetectorRegistry) -> Self {
        self.detectors = detectors;
        self
    }

    /// Override the timezone used for `period_window` boundary math.
    pub fn with_timezone(mut self, tz: chrono_tz::Tz) -> Self {
        self.tz = tz;
        self
    }

    /// Override the cadence + anchor. For biweekly the anchor must
    /// be `Some(monday)`; for monthly + weekly the anchor is
    /// ignored.
    pub fn with_cadence(mut self, cadence: RetroCadence, anchor: Option<NaiveDate>) -> Self {
        self.cadence = cadence;
        self.anchor = anchor;
        self
    }

    /// Read the persisted cadence + anchor from `ceremony_config`,
    /// initialising defaults if the rows are absent. Returns
    /// `(cadence, anchor)` ready to feed [`Self::with_cadence`].
    ///
    /// The binary calls this at registration time so the live
    /// plugin reflects whatever the user (or the
    /// `retro_set_cadence` agent tool) last wrote.
    pub fn load_persisted_cadence(
        conn: &ConnHandle,
        config_default: Option<RetroCadence>,
    ) -> (RetroCadence, Option<NaiveDate>) {
        let conn = match conn.0.lock() {
            Ok(g) => g,
            Err(_) => {
                tracing::warn!("retro cadence: conn mutex poisoned, falling back to weekly");
                return (RetroCadence::Weekly, None);
            }
        };
        // 1. Cadence: DB > config_default > Weekly.
        let db_cadence: Option<String> = conn
            .query_row(
                "SELECT value FROM ceremony_config WHERE kind='retro' AND key='cadence'",
                [],
                |row| row.get(0),
            )
            .ok();
        let cadence = db_cadence
            .as_deref()
            .and_then(RetroCadence::parse)
            .or(config_default)
            .unwrap_or(RetroCadence::Weekly);
        // 2. Anchor: only meaningful for biweekly. If absent for a
        //    biweekly cadence we'll initialise to "today's Monday"
        //    the first time a dispatch happens; storing the anchor
        //    eagerly here is the binary's responsibility once the
        //    cadence has been confirmed.
        let anchor: Option<NaiveDate> = conn
            .query_row(
                "SELECT value FROM ceremony_config WHERE kind='retro' AND key='cadence_anchor'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
        (cadence, anchor)
    }

    /// Persist cadence + anchor to `ceremony_config`. Used by the
    /// `retro_set_cadence` agent tool. Idempotent — UPSERTs each
    /// row.
    pub fn save_cadence(
        conn: &ConnHandle,
        cadence: RetroCadence,
        anchor: Option<NaiveDate>,
    ) -> Result<(), CeremonyError> {
        let conn = conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        conn.execute(
            "INSERT INTO ceremony_config (kind, key, value) VALUES ('retro', 'cadence', ?1) \
             ON CONFLICT(kind, key) DO UPDATE SET value = excluded.value",
            params![cadence.as_str()],
        )
        .map_err(|e| CeremonyError::Storage(format!("save retro cadence: {e}")))?;
        if let Some(d) = anchor {
            conn.execute(
                "INSERT INTO ceremony_config (kind, key, value) \
                 VALUES ('retro', 'cadence_anchor', ?1) \
                 ON CONFLICT(kind, key) DO UPDATE SET value = excluded.value",
                params![d.format("%Y-%m-%d").to_string()],
            )
            .map_err(|e| CeremonyError::Storage(format!("save retro anchor: {e}")))?;
        } else {
            conn.execute(
                "DELETE FROM ceremony_config WHERE kind='retro' AND key='cadence_anchor'",
                [],
            )
            .map_err(|e| CeremonyError::Storage(format!("clear retro anchor: {e}")))?;
        }
        Ok(())
    }

    /// Compute the ISO-week string (`YYYY-Www`) for a given moment.
    /// Exposed so callers + tests can predict the tablet id without
    /// running the plugin.
    pub fn iso_week(now: DateTime<Utc>) -> String {
        let iso = now.iso_week();
        format!("{:04}-W{:02}", iso.year(), iso.week())
    }
}

// --- Cadence helpers ---

/// Weekly window: `[Monday 00:00 local, next Monday 00:00 local)`.
fn weekly_window(
    period_key: &str,
    tz: chrono_tz::Tz,
) -> Result<(DateTime<Utc>, DateTime<Utc>), CeremonyError> {
    let parts: Vec<&str> = period_key.split("-W").collect();
    if parts.len() != 2 {
        return Err(CeremonyError::Other(format!(
            "weekly period_key '{period_key}' not 'YYYY-Www'"
        )));
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|e| CeremonyError::Other(format!("weekly period_key year: {e}")))?;
    let week: u32 = parts[1]
        .parse()
        .map_err(|e| CeremonyError::Other(format!("weekly period_key week: {e}")))?;
    let monday = NaiveDate::from_isoywd_opt(year, week, chrono::Weekday::Mon).ok_or_else(|| {
        CeremonyError::Other(format!("weekly period_key '{period_key}' invalid iso week"))
    })?;
    crate::local_window::iso_week_window_utc(monday, tz)
}

/// Biweekly period_key: `"B{N}"` where N is the count of complete
/// 14-day periods between the anchor and `date`. The anchor itself
/// is biweek 0.
///
/// We deliberately don't use a year-prefixed key — biweekly cycles
/// can straddle year boundaries, and a flat counter sidesteps the
/// "which year owns this biweek?" question.
fn biweekly_key(date: NaiveDate, anchor: Option<NaiveDate>) -> String {
    let anchor = match anchor {
        Some(a) => a,
        // No anchor → fall back to "B0" so the period_key is at
        // least stable; callers should ensure an anchor exists.
        None => return "B0".to_string(),
    };
    let delta = (date - anchor).num_days();
    // Floor division so dates before the anchor get negative
    // biweeks (rare but well-defined).
    let biweek = delta.div_euclid(14);
    format!("B{biweek}")
}

/// Biweekly window: `[anchor + 14N days, anchor + 14(N+1) days)`
/// in local tz.
fn biweekly_window(
    period_key: &str,
    anchor: NaiveDate,
    tz: chrono_tz::Tz,
) -> Result<(DateTime<Utc>, DateTime<Utc>), CeremonyError> {
    let n: i64 = period_key
        .strip_prefix('B')
        .ok_or_else(|| {
            CeremonyError::Other(format!("biweekly period_key '{period_key}' missing 'B' prefix"))
        })?
        .parse()
        .map_err(|e| CeremonyError::Other(format!("biweekly period_key index: {e}")))?;
    let start_date = anchor + Duration::days(14 * n);
    let end_date = anchor + Duration::days(14 * (n + 1));
    let start = crate::local_window::local_midnight_utc(start_date, tz)?;
    let end = crate::local_window::local_midnight_utc(end_date, tz)?;
    Ok((start, end))
}

/// Monthly period_key: `"YYYY-MM"`.
fn monthly_key(date: NaiveDate) -> String {
    format!("{:04}-{:02}", date.year(), date.month())
}

/// Monthly window: `[first-of-month 00:00 local, first-of-next-month
/// 00:00 local)`.
fn monthly_window(
    period_key: &str,
    tz: chrono_tz::Tz,
) -> Result<(DateTime<Utc>, DateTime<Utc>), CeremonyError> {
    let parts: Vec<&str> = period_key.split('-').collect();
    if parts.len() != 2 {
        return Err(CeremonyError::Other(format!(
            "monthly period_key '{period_key}' not 'YYYY-MM'"
        )));
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|e| CeremonyError::Other(format!("monthly period_key year: {e}")))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|e| CeremonyError::Other(format!("monthly period_key month: {e}")))?;
    let first = NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| {
        CeremonyError::Other(format!("monthly period_key '{period_key}' invalid month"))
    })?;
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| CeremonyError::Other("monthly next-month overflow".into()))?;
    let start = crate::local_window::local_midnight_utc(first, tz)?;
    let end = crate::local_window::local_midnight_utc(next, tz)?;
    Ok((start, end))
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

    fn period_window(
        &self,
        period_key: &str,
    ) -> Result<(DateTime<Utc>, DateTime<Utc>), CeremonyError> {
        match self.cadence {
            RetroCadence::Weekly => weekly_window(period_key, self.tz),
            RetroCadence::Biweekly => {
                let anchor = self.anchor.ok_or_else(|| {
                    CeremonyError::Other("biweekly retro requires an anchor date".into())
                })?;
                biweekly_window(period_key, anchor, self.tz)
            }
            RetroCadence::Monthly => monthly_window(period_key, self.tz),
        }
    }

    fn period_key(&self, now: DateTime<Utc>) -> String {
        match self.cadence {
            RetroCadence::Weekly => Self::iso_week(now),
            RetroCadence::Biweekly => biweekly_key(now.date_naive(), self.anchor),
            RetroCadence::Monthly => monthly_key(now.date_naive()),
        }
    }

    fn period_key_for_date(&self, date: NaiveDate) -> String {
        match self.cadence {
            RetroCadence::Weekly => {
                let iso = date.iso_week();
                format!("{:04}-W{:02}", iso.year(), iso.week())
            }
            RetroCadence::Biweekly => biweekly_key(date, self.anchor),
            RetroCadence::Monthly => monthly_key(date),
        }
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
            daily_tablets.push(r.map_err(|e| CeremonyError::Storage(format!("daily row: {e}")))?);
        }

        // 2. confirmed weekly priorities. The weekly tablet shares
        // this iso_week's period_key (kind=weekly).
        let mut stmt = conn
            .prepare(
                "SELECT p.id, td.body, td.done_at FROM ceremony_priorities p \
                 JOIN ceremony_tablets t ON p.tablet_id = t.id \
                 JOIN todos td ON td.id = p.todo_id \
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
            weekly_rollup.push(r.map_err(|e| CeremonyError::Storage(format!("rollup row: {e}")))?);
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
isn't already in the payload. Be concise and grounded. **IMPORTANT**: every pattern \
row in `payload.patterns_detected` MUST be surfaced as a `patterns`-section item — \
detected patterns are pre-vetted signal, not optional. Cite each one's `id` verbatim \
as the item's `citation_id`. Skipping a detected pattern is incorrect output.";

fn build_compose_prompt(facts: &GatheredFacts) -> String {
    format!(
        "Compose the retro for ISO week {}. Payload:\n{}",
        facts
            .payload
            .get("iso_week")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
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

/// Re-export the iso-week date helper so the catalog (T-0288) can
/// use the same Mon..Sun math without duplicating it. The public
/// name is intentionally noisy so it doesn't get mistaken for part
/// of the public crate API.
pub(crate) fn monday_sunday_for_iso_week_public(iso_week: &str) -> Option<(String, String)> {
    monday_sunday_for_iso_week(iso_week)
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
    Some((
        monday.format("%Y-%m-%d").to_string(),
        sunday.format("%Y-%m-%d").to_string(),
    ))
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
        // Post-V9: priorities are thin links to todos. Insert the
        // todo first, then the priority row.
        c.execute(
            "INSERT INTO todos (id, body, rationale, kind, workstream, created_at, \
                                due_at, done_at, archived_at, attrs) \
             VALUES ('td-prio-1', 'Ship retro plugin', 'carry-over', 'weekly_priority', \
                     NULL, '2026-05-11T08:00:00Z', NULL, NULL, NULL, '{}')",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO ceremony_priorities (id, tablet_id, todo_id, confirmed_at, ordinal) \
             VALUES (?1, ?2, ?3, ?4, 0)",
            params!["prio-1", "weekly-W20", "td-prio-1", "2026-05-11T08:00:00Z"],
        )
        .unwrap();
    }

    #[tokio::test]
    async fn iso_week_format_is_yyyy_w_ww() {
        // 2026-05-15 is a Friday in ISO week 20.
        let dt = DateTime::parse_from_rfc3339("2026-05-15T16:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
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
        let ctx = EngineCtx::for_test(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
        let facts = plugin.gather(&ctx).await.unwrap();
        let payload = facts.payload;
        assert_eq!(
            payload.get("iso_week").unwrap().as_str().unwrap(),
            "2026-W20"
        );
        let daily = payload.get("daily_tablets").unwrap().as_array().unwrap();
        assert_eq!(daily.len(), 1);
        let prios = payload
            .get("confirmed_priorities")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(prios.len(), 1);
        let rollup = payload.get("weekly_rollup").unwrap().as_array().unwrap();
        assert_eq!(rollup.len(), 1);
        let prior = payload
            .get("prior_retro_diaries")
            .unwrap()
            .as_array()
            .unwrap();
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
        let ctx = EngineCtx::for_test(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
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
        let ctx = EngineCtx::for_test(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
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
        let ctx = EngineCtx::for_test(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
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

    #[test]
    fn period_window_utc_iso_week() {
        let retro = RetroCeremony::new(make_llm_with_response("{}"), "stub-model");
        let (start, end) = retro.period_window("2026-W21").unwrap();
        assert_eq!(start.to_rfc3339(), "2026-05-18T00:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-05-25T00:00:00+00:00");
    }

    #[test]
    fn period_window_pacific_iso_week() {
        let retro = RetroCeremony::new(make_llm_with_response("{}"), "stub-model")
            .with_timezone(chrono_tz::America::Los_Angeles);
        let (start, end) = retro.period_window("2026-W21").unwrap();
        assert_eq!(start.to_rfc3339(), "2026-05-18T07:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-05-25T07:00:00+00:00");
    }

    #[test]
    fn period_window_rejects_non_iso_week() {
        let retro = RetroCeremony::new(make_llm_with_response("{}"), "stub-model");
        assert!(retro.period_window("not-a-week").is_err());
        assert!(retro.period_window("2026-05-19").is_err());
    }

    #[test]
    fn cadence_parse_accepts_aliases() {
        assert_eq!(RetroCadence::parse("weekly"), Some(RetroCadence::Weekly));
        assert_eq!(RetroCadence::parse("BIWEEKLY"), Some(RetroCadence::Biweekly));
        assert_eq!(
            RetroCadence::parse("fortnightly"),
            Some(RetroCadence::Biweekly)
        );
        assert_eq!(RetroCadence::parse("Monthly"), Some(RetroCadence::Monthly));
        assert_eq!(RetroCadence::parse("hourly"), None);
    }

    #[test]
    fn biweekly_period_key_counts_from_anchor() {
        let anchor = NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(); // Mon W21
        // Anchor day → B0
        assert_eq!(biweekly_key(anchor, Some(anchor)), "B0");
        // 13 days in → still B0 (window is [anchor, anchor+14d))
        assert_eq!(
            biweekly_key(anchor + Duration::days(13), Some(anchor)),
            "B0"
        );
        // 14 days in → B1
        assert_eq!(
            biweekly_key(anchor + Duration::days(14), Some(anchor)),
            "B1"
        );
        assert_eq!(
            biweekly_key(anchor + Duration::days(27), Some(anchor)),
            "B1"
        );
        assert_eq!(
            biweekly_key(anchor + Duration::days(28), Some(anchor)),
            "B2"
        );
        // Before the anchor → negative biweek (well-defined; floor div).
        assert_eq!(
            biweekly_key(anchor - Duration::days(1), Some(anchor)),
            "B-1"
        );
    }

    #[test]
    fn biweekly_period_window_covers_two_weeks() {
        let anchor = NaiveDate::from_ymd_opt(2026, 5, 18).unwrap();
        let retro = RetroCeremony::new(make_llm_with_response("{}"), "stub-model")
            .with_cadence(RetroCadence::Biweekly, Some(anchor));
        let (start, end) = retro.period_window("B0").unwrap();
        assert_eq!(start.to_rfc3339(), "2026-05-18T00:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-06-01T00:00:00+00:00");
        let (start1, end1) = retro.period_window("B1").unwrap();
        assert_eq!(start1.to_rfc3339(), "2026-06-01T00:00:00+00:00");
        assert_eq!(end1.to_rfc3339(), "2026-06-15T00:00:00+00:00");
    }

    #[test]
    fn biweekly_period_window_errors_without_anchor() {
        let retro = RetroCeremony::new(make_llm_with_response("{}"), "stub-model")
            .with_cadence(RetroCadence::Biweekly, None);
        assert!(retro.period_window("B0").is_err());
    }

    #[test]
    fn biweekly_idempotency_skips_within_window() {
        // Within the same biweek window, period_key_for_date is
        // identical → dispatch_for would find an existing tablet
        // and return Skipped. (Validated here at the key layer.)
        let anchor = NaiveDate::from_ymd_opt(2026, 5, 18).unwrap();
        let retro = RetroCeremony::new(make_llm_with_response("{}"), "stub-model")
            .with_cadence(RetroCadence::Biweekly, Some(anchor));
        let key_mon = retro.period_key_for_date(anchor);
        let key_fri = retro.period_key_for_date(anchor + Duration::days(4));
        let key_next_thu = retro.period_key_for_date(anchor + Duration::days(10));
        assert_eq!(key_mon, "B0");
        assert_eq!(key_fri, "B0");
        assert_eq!(key_next_thu, "B0");
        let key_off_week = retro.period_key_for_date(anchor + Duration::days(14));
        assert_eq!(key_off_week, "B1");
    }

    #[test]
    fn monthly_period_key_and_window() {
        let retro = RetroCeremony::new(make_llm_with_response("{}"), "stub-model")
            .with_cadence(RetroCadence::Monthly, None);
        let d = NaiveDate::from_ymd_opt(2026, 5, 19).unwrap();
        assert_eq!(retro.period_key_for_date(d), "2026-05");
        let (start, end) = retro.period_window("2026-05").unwrap();
        assert_eq!(start.to_rfc3339(), "2026-05-01T00:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-06-01T00:00:00+00:00");
        // December rolls into next year.
        let (start12, end12) = retro.period_window("2026-12").unwrap();
        assert_eq!(start12.to_rfc3339(), "2026-12-01T00:00:00+00:00");
        assert_eq!(end12.to_rfc3339(), "2027-01-01T00:00:00+00:00");
    }

    #[test]
    fn monthly_idempotency_skips_within_month() {
        let retro = RetroCeremony::new(make_llm_with_response("{}"), "stub-model")
            .with_cadence(RetroCadence::Monthly, None);
        let early = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let late = NaiveDate::from_ymd_opt(2026, 5, 31).unwrap();
        assert_eq!(retro.period_key_for_date(early), "2026-05");
        assert_eq!(retro.period_key_for_date(late), "2026-05");
        let next = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        assert_eq!(retro.period_key_for_date(next), "2026-06");
    }

    #[test]
    fn save_and_load_cadence_round_trips() {
        // In-memory DB via storage::Database.
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let _db = arawn_storage::Database::open(&db_path).expect("migrations");
        drop(_db);
        let conn = rusqlite::Connection::open(&db_path).expect("open conn");
        let handle = ConnHandle::new(conn);

        let anchor = NaiveDate::from_ymd_opt(2026, 5, 18).unwrap();
        RetroCeremony::save_cadence(&handle, RetroCadence::Biweekly, Some(anchor)).unwrap();
        let (loaded_cadence, loaded_anchor) =
            RetroCeremony::load_persisted_cadence(&handle, None);
        assert_eq!(loaded_cadence, RetroCadence::Biweekly);
        assert_eq!(loaded_anchor, Some(anchor));

        // Switch to monthly → anchor should be cleared.
        RetroCeremony::save_cadence(&handle, RetroCadence::Monthly, None).unwrap();
        let (loaded_cadence2, loaded_anchor2) =
            RetroCeremony::load_persisted_cadence(&handle, None);
        assert_eq!(loaded_cadence2, RetroCadence::Monthly);
        assert_eq!(loaded_anchor2, None);
    }

    #[test]
    fn load_falls_back_to_config_default_then_weekly() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let _db = arawn_storage::Database::open(&db_path).expect("migrations");
        drop(_db);
        let conn = rusqlite::Connection::open(&db_path).expect("open conn");
        let handle = ConnHandle::new(conn);

        // Empty DB + no config default → Weekly.
        let (c1, _) = RetroCeremony::load_persisted_cadence(&handle, None);
        assert_eq!(c1, RetroCadence::Weekly);

        // Empty DB + config default → use the default.
        let (c2, _) =
            RetroCeremony::load_persisted_cadence(&handle, Some(RetroCadence::Monthly));
        assert_eq!(c2, RetroCadence::Monthly);
    }
}
