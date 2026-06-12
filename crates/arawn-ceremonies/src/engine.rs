//! Concrete [`CeremonyDispatcher`] implementation: the
//! gather → pattern_detect → compose → write pipeline.
//!
//! This is the load-bearing piece of the ceremony engine. The
//! contract:
//!
//! 1. Look up the plugin by kind.
//! 2. Compute the period key via `plugin.period_key(now)`.
//! 3. Short-circuit if a tablet already exists for `(kind,
//!    period_key)` with `status != "open"`.
//! 4. Open a single transaction for the whole run. Every row
//!    written during the run rides this transaction; mid-run
//!    failure rolls everything back.
//! 5. Insert the tablet row.
//! 6. Construct [`EngineCtx`] (sharing the transaction-bound
//!    connection) and call `plugin.gather()`.
//! 7. If the plugin returns a [`PatternDetector`], run it now and
//!    write each [`DetectedPattern`] via `ctx.write_pattern_row`.
//!    The returned ids become valid `citation_id`s for the compose
//!    phase.
//! 8. Acquire an `arawn_llm::gate::acquire_local` permit and call
//!    `plugin.compose()`.
//! 9. Iterate the returned [`NewItem`]s. Each variant routes to the
//!    matching write path — `Composed` requires a non-empty
//!    `citation_id` (refused with [`CeremonyError::MissingCitation`]
//!    when empty); `User` writes without one.
//! 10. Commit on success; rollback on any error.
//!
//! The two-write-path citation contract from I-0043 §Design
//! Decisions #4 is enforced via the [`NewItem`] enum variants from
//! T-0279 plus a runtime check on `citation_id.is_empty()` inside
//! step 9.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use tracing::warn;
use uuid::Uuid;

use crate::CeremonyError;
use crate::events::{CeremonyEvent, CeremonyEventSender, emit as emit_event};
use crate::plugin::{Ceremony, CeremonyCtx, ComposedItem, NewItem, UserItem};
use crate::registry::PluginRegistry;
use crate::runner::{CeremonyDispatcher, DispatchOutcome};
use crate::types::{DetectedPattern, GatheredFacts, ItemKind, TabletStatus};

/// Wraps a shared SQLite connection. Used by the dispatcher and by
/// the per-run context.
#[derive(Clone)]
pub struct ConnHandle(pub Arc<Mutex<Connection>>);

impl ConnHandle {
    pub fn new(conn: Connection) -> Self {
        Self(Arc::new(Mutex::new(conn)))
    }
}

/// The concrete [`CeremonyDispatcher`]. One per process.
#[derive(Clone)]
pub struct EngineDispatcher {
    conn: ConnHandle,
    registry: PluginRegistry,
    /// Optional event channel. When `None`, state-change events are
    /// dropped silently (used by tests that don't care). Production
    /// always wires this so the WS layer can forward to clients.
    events: Option<CeremonyEventSender>,
}

impl EngineDispatcher {
    pub fn new(conn: ConnHandle, registry: PluginRegistry) -> Self {
        Self {
            conn,
            registry,
            events: None,
        }
    }

    /// Attach the event sender. Cloned senders share the channel —
    /// the dispatcher, service, and any plugin that wants to emit
    /// all hold one. Chainable.
    pub fn with_events(mut self, sender: CeremonyEventSender) -> Self {
        self.events = Some(sender);
        self
    }
}

#[async_trait]
impl CeremonyDispatcher for EngineDispatcher {
    async fn dispatch(&self, kind: &str) -> Result<DispatchOutcome, CeremonyError> {
        self.dispatch_for(kind, Utc::now().date_naive()).await
    }

    async fn dispatch_for(
        &self,
        kind: &str,
        target: chrono::NaiveDate,
    ) -> Result<DispatchOutcome, CeremonyError> {
        // 1–2: plugin + period (derived from target, not from "now").
        let plugin = self.registry.get(kind).ok_or_else(|| {
            CeremonyError::Other(format!("no plugin registered for kind '{kind}'"))
        })?;
        let now = Utc::now();
        let period_key = plugin.period_key_for_date(target);
        // `recovered` is set when the target date's period_key
        // differs from today's. Using period_key equivalence (not
        // raw date equality) means a weekly back-fill for a
        // mid-week date doesn't get flagged when "today" is also
        // in the same ISO week.
        let live_period_key = plugin.period_key(now);
        let recovered = period_key != live_period_key;

        // The dispatch proper runs in an inner block so its terminal
        // outcome (skip / generate / error) can be recorded to the
        // persisted run history (ARAWN-T-0477) at a single point below.
        let result: Result<DispatchOutcome, CeremonyError> = async {
            // 3: idempotency — skip if tablet already exists with non-`open` status.
            if let Some(status) = current_tablet_status(&self.conn, kind, &period_key)? {
                if status != TabletStatus::Open {
                    return Ok(DispatchOutcome::Skipped {
                        reason: format!(
                            "tablet for ({kind}, {period_key}) already exists with status '{}'",
                            status.as_str()
                        ),
                    });
                }
                // status == Open: caller is rerunning a tablet that was
                // never reviewed. Conservative choice: skip so we don't
                // overwrite in-flight content. Production may want to
                // re-open this for explicit `force` runs — defer to a
                // follow-up.
                return Ok(DispatchOutcome::Skipped {
                    reason: format!(
                        "tablet for ({kind}, {period_key}) already open — refusing to overwrite"
                    ),
                });
            }

            // 4: Run the pipeline with auto-commit writes. Earlier
            // revisions wrapped the whole pipeline in BEGIN IMMEDIATE
            // …COMMIT, which held a SQLite write lock across the LLM
            // compose call (up to several seconds) and starved every
            // other writer on `arawn.db` — including `create_session`
            // via WS-RPC. UAT exposed this when boot-time back-fill
            // composed 15 tablets in sequence and the test client's
            // first `create_session` hit `busy_timeout` (5s) and
            // failed.
            //
            // Each insert below is its own SQLite auto-commit
            // transaction; the LLM compose call sits between writes
            // with no lock held. On any error after the tablet row is
            // inserted we clean up by deleting that row so the next
            // dispatch can retry.
            let pipeline_result = self
                .run_pipeline(plugin.as_ref(), &period_key, now, recovered)
                .await;
            match pipeline_result {
                Ok(tablet_id) => {
                    if let Some(events) = &self.events {
                        emit_event(
                            events,
                            CeremonyEvent::TabletGenerated {
                                tablet_id: tablet_id.clone(),
                                kind: kind.to_string(),
                                period_key: period_key.clone(),
                            },
                        );
                    }
                    Ok(DispatchOutcome::Generated { tablet_id })
                }
                Err(e) => {
                    // Best-effort cleanup so a failed compose doesn't
                    // leave an `open`-status tablet that idempotency
                    // would later refuse to overwrite.
                    let tablet_id = format!("{}-{period_key}", kind);
                    if let Err(cleanup_err) = delete_tablet(&self.conn, &tablet_id) {
                        warn!(
                            tablet_id,
                            error = %cleanup_err,
                            "failed to clean up tablet after pipeline error"
                        );
                    }
                    Err(e)
                }
            }
        }
        .await;

        // ARAWN-T-0477: record the dispatch outcome to the persisted run
        // history (best-effort — a history write must never fail the run).
        self.record_run(kind, &period_key, &result);

        result
    }
}

impl EngineDispatcher {
    /// Persist a single dispatch outcome to `ceremony_run_history`
    /// (ARAWN-T-0477). Best-effort: a failure here is logged, never
    /// propagated — recording history must not break the ceremony run.
    fn record_run(
        &self,
        kind: &str,
        period_key: &str,
        result: &Result<DispatchOutcome, CeremonyError>,
    ) {
        let (outcome, error) = match result {
            Ok(DispatchOutcome::Generated { .. }) => ("ok", None),
            Ok(DispatchOutcome::Skipped { .. }) => ("skipped", None),
            Err(e) => ("error", Some(e.to_string())),
        };
        let ran_at = Utc::now().to_rfc3339();
        let conn = match self.conn.0.lock() {
            Ok(c) => c,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Err(e) = arawn_storage::failure_history::record_ceremony_run(
            &conn,
            kind,
            period_key,
            outcome,
            error.as_deref(),
            &ran_at,
        ) {
            warn!(kind, error = %e, "failed to record ceremony run history");
        }
    }

    async fn run_pipeline(
        &self,
        plugin: &dyn Ceremony,
        period_key: &str,
        now: chrono::DateTime<Utc>,
        recovered: bool,
    ) -> Result<String, CeremonyError> {
        // 5: insert the tablet.
        let tablet_id = format!("{}-{period_key}", plugin.kind());
        insert_tablet(
            &self.conn,
            &tablet_id,
            plugin.kind(),
            period_key,
            now,
            recovered,
        )?;

        // 6: construct ctx. Pin the gather window now so gather/
        // compose see a stable [start, end) regardless of when this
        // dispatch fires (live cron, manual run, or back-fill).
        let period_window = plugin.period_window(period_key)?;
        let ctx = EngineCtx::new(
            self.conn.clone(),
            tablet_id.clone(),
            period_key.to_string(),
            period_window,
        );

        // 7: gather (deterministic).
        let mut facts: GatheredFacts = plugin.gather(&ctx).await?;

        // 8: pattern detector (optional). Detected patterns get
        // written to `ceremony_patterns_detected` AND injected back
        // into `facts.payload.patterns_detected` so the compose LLM
        // can cite them. Without this re-injection, compose has no
        // way to surface a freshly-detected pattern — its prompt only
        // sees the gather payload. (I-0049 T-0316 surfaced this gap
        // when priority_completion_ratio fired in the DB but never
        // landed in the retro's patterns section.)
        if let Some(detector) = plugin.patterns() {
            let patterns = detector.detect(&ctx).await?;
            let mut written: Vec<serde_json::Value> = Vec::with_capacity(patterns.len());
            for pattern in patterns {
                let iso_week = pattern.iso_week.clone();
                let pattern_key = pattern.pattern_key.clone();
                let magnitude = pattern.magnitude;
                let payload = pattern.payload.clone();
                let id = ctx.write_pattern_row(pattern).await?;
                if let Some(events) = &self.events {
                    emit_event(
                        events,
                        CeremonyEvent::PatternDetected {
                            pattern_id: id.clone(),
                            iso_week: iso_week.clone(),
                            pattern_key: pattern_key.clone(),
                        },
                    );
                }
                written.push(serde_json::json!({
                    "id": id,
                    "iso_week": iso_week,
                    "pattern_key": pattern_key,
                    "magnitude": magnitude,
                    "payload": payload,
                }));
            }
            if let Some(payload_obj) = facts.payload.as_object_mut() {
                payload_obj.insert(
                    "patterns_detected".to_string(),
                    serde_json::Value::Array(written),
                );
            }
        }

        // 9: compose, gated through the process-wide LLM resource gate.
        let _permit = arawn_llm::gate::acquire_local()
            .await
            .map_err(|e| CeremonyError::Llm(format!("llm gate refused acquire: {e:?}")))?;
        let new_items = plugin.compose(&ctx, facts).await?;

        // 10: dispatch each item to the right write path.
        let mut ordinal_by_section: std::collections::HashMap<String, i32> =
            std::collections::HashMap::new();
        for item in new_items {
            match item {
                NewItem::Composed(c) => {
                    write_composed_item(&self.conn, &c, &mut ordinal_by_section)?
                }
                NewItem::User(u) => write_user_item(&self.conn, &u, &mut ordinal_by_section)?,
            }
        }
        Ok(tablet_id)
    }
}

/// Per-run [`CeremonyCtx`]. Holds the shared connection so the
/// plugin's gather/compose phases can issue reads + writes through
/// the same transaction.
pub struct EngineCtx {
    conn: ConnHandle,
    tablet_id: String,
    period_key: String,
    period_window: (DateTime<Utc>, DateTime<Utc>),
}

impl EngineCtx {
    /// Construct an EngineCtx with an explicit pinned window.
    /// The dispatcher passes the plugin-computed window so gather
    /// queries see a stable `[start, end)` regardless of when the
    /// dispatch actually fires.
    pub fn new(
        conn: ConnHandle,
        tablet_id: String,
        period_key: String,
        period_window: (DateTime<Utc>, DateTime<Utc>),
    ) -> Self {
        Self {
            conn,
            tablet_id,
            period_key,
            period_window,
        }
    }

    /// Test-only constructor that synthesises a placeholder window
    /// `(now, now + 1d)`. Tests that exercise gather paths should
    /// use [`Self::new`] with a real window; this helper exists so
    /// detector and write-path tests don't need to invent timestamps
    /// they don't care about.
    #[doc(hidden)]
    pub fn for_test(conn: ConnHandle, tablet_id: String, period_key: String) -> Self {
        let now = Utc::now();
        Self::new(
            conn,
            tablet_id,
            period_key,
            (now, now + chrono::Duration::days(1)),
        )
    }

    /// Access to the underlying connection for plugins that need to
    /// run their own gather SQL. Plugins should treat this as
    /// read-mostly — every write goes through the trait methods so
    /// the engine knows about it.
    pub fn conn(&self) -> &ConnHandle {
        &self.conn
    }
}

#[async_trait]
impl CeremonyCtx for EngineCtx {
    fn period_key(&self) -> &str {
        &self.period_key
    }
    fn tablet_id(&self) -> &str {
        &self.tablet_id
    }
    fn period_window(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        self.period_window
    }
    fn conn_handle(&self) -> Option<&ConnHandle> {
        Some(&self.conn)
    }

    async fn write_pattern_row(&self, pattern: DetectedPattern) -> Result<String, CeremonyError> {
        let id = Uuid::new_v4().to_string();
        let payload = pattern.payload.to_string();
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
        conn.execute(
            "INSERT INTO ceremony_patterns_detected (id, iso_week, pattern_key, magnitude, payload, surfaced_in_retro) \
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![&id, &pattern.iso_week, &pattern.pattern_key, pattern.magnitude, payload],
        )
        .map_err(|e| CeremonyError::Storage(format!("insert pattern row: {e}")))?;
        Ok(id)
    }
}

// --- SQL helpers (private) ---

fn current_tablet_status(
    conn: &ConnHandle,
    kind: &str,
    period_key: &str,
) -> Result<Option<TabletStatus>, CeremonyError> {
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
    let status: Option<String> = conn
        .query_row(
            "SELECT status FROM ceremony_tablets WHERE kind = ?1 AND period_key = ?2",
            params![kind, period_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| CeremonyError::Storage(format!("query tablet status: {e}")))?;
    Ok(status.map(|s| match s.as_str() {
        "open" => TabletStatus::Open,
        "reviewed" => TabletStatus::Reviewed,
        "unreviewed" => TabletStatus::Unreviewed,
        _ => TabletStatus::Archived,
    }))
}

/// Delete a tablet row by id. Used by the dispatcher's error path
/// to roll back a failed pipeline so the next dispatch can retry.
fn delete_tablet(conn: &ConnHandle, tablet_id: &str) -> Result<(), CeremonyError> {
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
    conn.execute(
        "DELETE FROM ceremony_tablets WHERE id = ?1",
        params![tablet_id],
    )
    .map_err(|e| CeremonyError::Storage(format!("delete tablet: {e}")))?;
    Ok(())
}

fn insert_tablet(
    conn: &ConnHandle,
    tablet_id: &str,
    kind: &str,
    period_key: &str,
    now: chrono::DateTime<Utc>,
    recovered: bool,
) -> Result<(), CeremonyError> {
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
    conn.execute(
        "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, lenses_scanned, recovered) \
         VALUES (?1, ?2, ?3, ?4, 'open', '[]', ?5)",
        params![tablet_id, kind, period_key, now.to_rfc3339(), recovered as i64],
    )
    .map_err(|e| CeremonyError::Storage(format!("insert tablet: {e}")))?;
    Ok(())
}

fn next_ordinal(
    ordinal_by_section: &mut std::collections::HashMap<String, i32>,
    section_key: &str,
) -> i32 {
    let next = ordinal_by_section
        .entry(section_key.to_string())
        .or_insert(-1);
    *next += 1;
    *next
}

fn write_composed_item(
    conn: &ConnHandle,
    item: &ComposedItem,
    ordinal_by_section: &mut std::collections::HashMap<String, i32>,
) -> Result<(), CeremonyError> {
    if item.citation_id.trim().is_empty() {
        return Err(CeremonyError::missing_citation(format!(
            "composed item in section '{}' has empty citation_id",
            item.section_key
        )));
    }
    let _ = next_ordinal(ordinal_by_section, &item.section_key);
    let body = item.body.to_string();
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
    conn.execute(
        "INSERT INTO ceremony_items (id, tablet_id, section_key, ordinal, kind, body, citation_id, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            Uuid::new_v4().to_string(),
            &item.tablet_id,
            &item.section_key,
            item.ordinal,
            kind_str(&item.kind),
            body,
            &item.citation_id,
            Utc::now().to_rfc3339(),
        ],
    )
    .map_err(|e| CeremonyError::Storage(format!("insert composed item: {e}")))?;
    Ok(())
}

fn write_user_item(
    conn: &ConnHandle,
    item: &UserItem,
    ordinal_by_section: &mut std::collections::HashMap<String, i32>,
) -> Result<(), CeremonyError> {
    let _ = next_ordinal(ordinal_by_section, &item.section_key);
    let body = item.body.to_string();
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
    conn.execute(
        "INSERT INTO ceremony_items (id, tablet_id, section_key, ordinal, kind, body, citation_id, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
        params![
            Uuid::new_v4().to_string(),
            &item.tablet_id,
            &item.section_key,
            item.ordinal,
            kind_str(&item.kind),
            body,
            Utc::now().to_rfc3339(),
        ],
    )
    .map_err(|e| CeremonyError::Storage(format!("insert user item: {e}")))?;
    Ok(())
}

/// Manual transaction control. Production dispatch no longer
/// wraps the pipeline in a single transaction (see `dispatch_for`
/// for the reasoning), but the tests below still use these helpers
/// to exercise individual write paths in isolation.
#[cfg(test)]
fn begin(conn: &ConnHandle) -> Result<(), CeremonyError> {
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| CeremonyError::Storage(format!("BEGIN: {e}")))?;
    Ok(())
}

#[cfg(test)]
fn commit(conn: &ConnHandle) -> Result<(), CeremonyError> {
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
    conn.execute("COMMIT", [])
        .map_err(|e| CeremonyError::Storage(format!("COMMIT: {e}")))?;
    Ok(())
}

fn kind_str(k: &ItemKind) -> &'static str {
    match k {
        ItemKind::CalendarEvent => "calendar_event",
        ItemKind::Attention => "attention",
        ItemKind::Proposal => "proposal",
        ItemKind::Todo => "todo",
        ItemKind::Pattern => "pattern",
        ItemKind::Priority => "priority",
        ItemKind::Freeform => "freeform",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{Ceremony, CronSchedule, NewItem};
    use crate::types::GatheredFacts;
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;

    // --- Fixtures ---

    fn open_test_db() -> (TempDir, ConnHandle) {
        // Apply migrations via arawn-storage, then open a fresh
        // rusqlite connection to the same file. SQLite happily
        // accepts multiple connections to one file.
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let _db = arawn_storage::Database::open(&db_path).expect("migrations");
        drop(_db);
        let conn = Connection::open(&db_path).expect("open conn");
        (tmp, ConnHandle::new(conn))
    }

    // A plugin whose compose returns a configurable item set.
    struct ScriptedPlugin {
        kind: &'static str,
        items: std::sync::Mutex<Vec<NewItem>>,
    }
    impl ScriptedPlugin {
        fn new(kind: &'static str, items: Vec<NewItem>) -> Self {
            Self {
                kind,
                items: std::sync::Mutex::new(items),
            }
        }
    }
    #[async_trait]
    impl Ceremony for ScriptedPlugin {
        fn kind(&self) -> &'static str {
            self.kind
        }
        fn period_key(&self, _now: chrono::DateTime<Utc>) -> String {
            "2026-W20".into()
        }
        fn period_window(
            &self,
            _period_key: &str,
        ) -> Result<(DateTime<Utc>, DateTime<Utc>), CeremonyError> {
            let now = Utc::now();
            Ok((now, now + chrono::Duration::days(7)))
        }
        fn default_schedule(&self) -> CronSchedule {
            CronSchedule::local("0 16 * * FRI")
        }
        async fn gather(&self, _ctx: &dyn CeremonyCtx) -> Result<GatheredFacts, CeremonyError> {
            Ok(GatheredFacts::new(json!({})))
        }
        async fn compose(
            &self,
            _ctx: &dyn CeremonyCtx,
            _facts: GatheredFacts,
        ) -> Result<Vec<NewItem>, CeremonyError> {
            Ok(std::mem::take(&mut *self.items.lock().unwrap()))
        }
    }

    fn item_composed(tablet_id: &str, section: &str, citation: &str) -> NewItem {
        NewItem::composed(ComposedItem {
            tablet_id: tablet_id.into(),
            section_key: section.into(),
            ordinal: 0,
            kind: ItemKind::Pattern,
            body: json!({"text": "hi"}),
            citation_id: citation.into(),
        })
    }

    fn item_user(tablet_id: &str, section: &str) -> NewItem {
        NewItem::user(UserItem {
            tablet_id: tablet_id.into(),
            section_key: section.into(),
            ordinal: 0,
            kind: ItemKind::Freeform,
            body: json!({"text": "hi"}),
        })
    }

    fn count_rows(conn: &ConnHandle, table: &str) -> i64 {
        let c = conn.0.lock().unwrap();
        c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    // --- Tests ---

    #[tokio::test]
    async fn happy_path_writes_tablet_and_composed_item_with_citation() {
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        // Tablet id format = "{kind}-{period_key}" → "retro-2026-W20"
        let items = vec![item_composed("retro-2026-W20", "what_happened", "sig-1")];
        reg.register(Arc::new(ScriptedPlugin::new("retro", items)))
            .unwrap();
        let disp = EngineDispatcher::new(conn.clone(), reg);
        let outcome = disp.dispatch("retro").await.unwrap();
        match outcome {
            DispatchOutcome::Generated { tablet_id } => {
                assert_eq!(tablet_id, "retro-2026-W20");
            }
            other => panic!("expected Generated, got {other:?}"),
        }
        assert_eq!(count_rows(&conn, "ceremony_tablets"), 1);
        assert_eq!(count_rows(&conn, "ceremony_items"), 1);
    }

    #[tokio::test]
    async fn composed_item_missing_citation_rolls_back_whole_run() {
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        let items = vec![
            item_composed("retro-2026-W20", "what_happened", "sig-1"),
            item_composed("retro-2026-W20", "what_happened", ""), // missing
        ];
        reg.register(Arc::new(ScriptedPlugin::new("retro", items)))
            .unwrap();
        let disp = EngineDispatcher::new(conn.clone(), reg);
        let err = disp.dispatch("retro").await.unwrap_err();
        assert!(matches!(err, CeremonyError::MissingCitation(_)));
        // Rollback: no tablet, no items.
        assert_eq!(count_rows(&conn, "ceremony_tablets"), 0);
        assert_eq!(count_rows(&conn, "ceremony_items"), 0);
    }

    /// Read the latest `ceremony_run_history` row for a kind.
    fn latest_run(conn: &ConnHandle, kind: &str) -> (String, Option<String>) {
        let c = conn.0.lock().unwrap();
        c.query_row(
            "SELECT outcome, error FROM ceremony_run_history \
             WHERE kind = ?1 ORDER BY id DESC LIMIT 1",
            [kind],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn dispatch_records_ok_run_history() {
        // ARAWN-T-0477: a successful dispatch leaves an 'ok' history row.
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        let items = vec![item_composed("retro-2026-W20", "what_happened", "sig-1")];
        reg.register(Arc::new(ScriptedPlugin::new("retro", items)))
            .unwrap();
        let disp = EngineDispatcher::new(conn.clone(), reg);
        disp.dispatch("retro").await.unwrap();

        let (outcome, error) = latest_run(&conn, "retro");
        assert_eq!(outcome, "ok");
        assert!(error.is_none());
    }

    #[tokio::test]
    async fn failing_dispatch_records_error_run_history() {
        // ARAWN-T-0477: a failed dispatch is queryable after the fact — an
        // 'error' row with the failure text, even though the tablet itself
        // rolled back.
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        let items = vec![
            item_composed("retro-2026-W20", "what_happened", "sig-1"),
            item_composed("retro-2026-W20", "what_happened", ""), // missing citation → error
        ];
        reg.register(Arc::new(ScriptedPlugin::new("retro", items)))
            .unwrap();
        let disp = EngineDispatcher::new(conn.clone(), reg);
        let _ = disp.dispatch("retro").await.unwrap_err();

        // Tablet rolled back, but the failure is recorded.
        assert_eq!(count_rows(&conn, "ceremony_tablets"), 0);
        let (outcome, error) = latest_run(&conn, "retro");
        assert_eq!(outcome, "error");
        assert!(error.is_some(), "error text should be persisted");
    }

    #[tokio::test]
    async fn user_item_without_citation_is_accepted() {
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        let items = vec![item_user("retro-2026-W20", "diary")];
        reg.register(Arc::new(ScriptedPlugin::new("retro", items)))
            .unwrap();
        let disp = EngineDispatcher::new(conn.clone(), reg);
        let outcome = disp.dispatch("retro").await.unwrap();
        assert!(matches!(outcome, DispatchOutcome::Generated { .. }));
        assert_eq!(count_rows(&conn, "ceremony_items"), 1);
        // citation_id should be NULL.
        let c = conn.0.lock().unwrap();
        let citation: Option<String> = c
            .query_row(
                "SELECT citation_id FROM ceremony_items LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(citation.is_none(), "user item should have NULL citation_id");
    }

    #[tokio::test]
    async fn idempotency_skips_when_open_tablet_exists() {
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        reg.register(Arc::new(ScriptedPlugin::new("retro", vec![])))
            .unwrap();
        let disp = EngineDispatcher::new(conn.clone(), reg);
        let first = disp.dispatch("retro").await.unwrap();
        assert!(matches!(first, DispatchOutcome::Generated { .. }));
        let second = disp.dispatch("retro").await.unwrap();
        assert!(matches!(second, DispatchOutcome::Skipped { .. }));
        // Only one tablet row.
        assert_eq!(count_rows(&conn, "ceremony_tablets"), 1);
    }

    #[tokio::test]
    async fn unknown_kind_errors() {
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        let disp = EngineDispatcher::new(conn, reg);
        let err = disp.dispatch("nope").await.unwrap_err();
        assert!(matches!(err, CeremonyError::Other(_)));
    }

    #[tokio::test]
    async fn dispatch_for_today_marks_not_recovered() {
        // Live dispatch path: target == today → recovered should be 0.
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        reg.register(Arc::new(ScriptedPlugin::new("retro", vec![])))
            .unwrap();
        let disp = EngineDispatcher::new(conn.clone(), reg);
        let outcome = disp
            .dispatch_for("retro", Utc::now().date_naive())
            .await
            .unwrap();
        assert!(matches!(outcome, DispatchOutcome::Generated { .. }));
        let c = conn.0.lock().unwrap();
        let recovered: i64 = c
            .query_row(
                "SELECT recovered FROM ceremony_tablets LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            recovered, 0,
            "today's dispatch must not be flagged recovered"
        );
    }

    // A test stub whose period_key is date-sensitive — needed for
    // dispatch_for tests where historical dispatch must produce a
    // different period_key than the live one.
    struct DateAwarePlugin;
    #[async_trait]
    impl Ceremony for DateAwarePlugin {
        fn kind(&self) -> &'static str {
            "daily"
        }
        fn period_key(&self, now: chrono::DateTime<Utc>) -> String {
            now.format("%Y-%m-%d").to_string()
        }
        fn period_window(
            &self,
            _period_key: &str,
        ) -> Result<(chrono::DateTime<Utc>, chrono::DateTime<Utc>), CeremonyError> {
            let now = Utc::now();
            Ok((now, now + chrono::Duration::days(1)))
        }
        fn default_schedule(&self) -> CronSchedule {
            CronSchedule::local("0 7 * * *")
        }
        async fn gather(&self, _ctx: &dyn CeremonyCtx) -> Result<GatheredFacts, CeremonyError> {
            Ok(GatheredFacts::new(json!({})))
        }
        async fn compose(
            &self,
            _ctx: &dyn CeremonyCtx,
            _facts: GatheredFacts,
        ) -> Result<Vec<NewItem>, CeremonyError> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn dispatch_for_historical_marks_recovered() {
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        reg.register(Arc::new(DateAwarePlugin)).unwrap();
        let disp = EngineDispatcher::new(conn.clone(), reg);
        let target = (Utc::now() - chrono::Duration::days(90)).date_naive();
        let outcome = disp.dispatch_for("daily", target).await.unwrap();
        assert!(matches!(outcome, DispatchOutcome::Generated { .. }));
        let c = conn.0.lock().unwrap();
        let (period_key, recovered): (String, i64) = c
            .query_row(
                "SELECT period_key, recovered FROM ceremony_tablets LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(period_key, target.format("%Y-%m-%d").to_string());
        assert_eq!(recovered, 1, "historical dispatch must set recovered=1");
    }

    #[tokio::test]
    async fn dispatch_for_historical_idempotent() {
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        reg.register(Arc::new(DateAwarePlugin)).unwrap();
        let disp = EngineDispatcher::new(conn.clone(), reg);
        let target = (Utc::now() - chrono::Duration::days(30)).date_naive();
        let first = disp.dispatch_for("daily", target).await.unwrap();
        assert!(matches!(first, DispatchOutcome::Generated { .. }));
        let second = disp.dispatch_for("daily", target).await.unwrap();
        assert!(matches!(second, DispatchOutcome::Skipped { .. }));
        assert_eq!(count_rows(&conn, "ceremony_tablets"), 1);
    }

    #[tokio::test]
    async fn write_pattern_row_returns_id_and_writes() {
        let (_tmp, conn) = open_test_db();
        let ctx = EngineCtx::for_test(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
        // Wrap in BEGIN/COMMIT so the insert isn't auto-committed in
        // isolation (mimics how dispatch() actually runs).
        begin(&conn).unwrap();
        let id = ctx
            .write_pattern_row(DetectedPattern {
                iso_week: "2026-W20".into(),
                pattern_key: "priority_completion_ratio".into(),
                magnitude: 0.4,
                payload: json!({"source": "test"}),
            })
            .await
            .unwrap();
        commit(&conn).unwrap();
        assert!(!id.is_empty());
        assert_eq!(count_rows(&conn, "ceremony_patterns_detected"), 1);
    }
}
