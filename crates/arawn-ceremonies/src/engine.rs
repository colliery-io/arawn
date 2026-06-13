//! Concrete [`CeremonyDispatcher`] implementation: the
//! gather → pattern_detect → compose → write pipeline.
//!
//! This is the load-bearing piece of the ceremony engine. The
//! contract:
//!
//! 1. Look up the plugin by kind.
//! 2. Compute the period key via `plugin.period_key(now)`.
//! 3. Short-circuit if a tablet already exists for `(kind,
//!    period_key)` with `status != "open"` (or, for an `open` tablet,
//!    unless `force` is set — then delete + regenerate, T-0479).
//! 4. Construct [`EngineCtx`] and call `plugin.gather()` (read-only).
//! 5. If the plugin returns a [`PatternDetector`], run it now and
//!    pre-assign each [`DetectedPattern`] an id. The ids are injected
//!    into the gather facts so they're valid `citation_id`s for the
//!    compose phase — but the rows are NOT written yet.
//! 6. Acquire an `arawn_llm::gate::acquire_local` permit and call
//!    `plugin.compose()`. **No DB lock is held across this call.**
//! 7. Open ONE SQLite transaction and, inside it, write the tablet
//!    row, the deferred pattern rows, and every composed/user item.
//!    `Composed` items require a non-empty `citation_id` (refused
//!    with [`CeremonyError::MissingCitation`]); `User` items don't.
//!    Commit on success; any error rolls back the ENTIRE dispatch —
//!    no half-written tablet, no orphaned pattern rows (T-0479).
//!
//! The transaction deliberately spans only the post-compose write
//! phase (step 7), never gather/compose, so it can't hold the
//! `arawn.db` write lock across the multi-second LLM call — the
//! lock-starvation regression an earlier per-item auto-commit was
//! working around.
//!
//! The two-write-path citation contract from I-0043 §Design
//! Decisions #4 is enforced via the [`NewItem`] enum variants from
//! T-0279 plus a runtime check on `citation_id.is_empty()` inside
//! step 7.

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
        self.dispatch_with(kind, Utc::now().date_naive(), false)
            .await
    }

    async fn dispatch_for(
        &self,
        kind: &str,
        target: chrono::NaiveDate,
    ) -> Result<DispatchOutcome, CeremonyError> {
        self.dispatch_with(kind, target, false).await
    }

    async fn dispatch_with(
        &self,
        kind: &str,
        target: chrono::NaiveDate,
        force: bool,
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
            // 3: idempotency — skip if a tablet already exists for the period.
            if let Some(status) = current_tablet_status(&self.conn, kind, &period_key)? {
                if status != TabletStatus::Open {
                    // Reviewed / archived — never overwrite, even with force.
                    return Ok(DispatchOutcome::Skipped {
                        reason: format!(
                            "tablet for ({kind}, {period_key}) already exists with status '{}'",
                            status.as_str()
                        ),
                    });
                }
                // status == Open (generated but never reviewed).
                if force {
                    // ARAWN-T-0479: regenerate — delete the open tablet (its
                    // items/sections cascade via ON DELETE CASCADE) and fall
                    // through to a fresh run. A bad LLM output can thus be
                    // redone without manual row surgery.
                    delete_tablet(&self.conn, &format!("{kind}-{period_key}"))?;
                } else {
                    return Ok(DispatchOutcome::Skipped {
                        reason: format!(
                            "tablet for ({kind}, {period_key}) already open — refusing to \
                             overwrite (force a regenerate to replace it)"
                        ),
                    });
                }
            }

            // 4: run the pipeline. Every write it makes is wrapped in one
            // transaction (see `run_pipeline`), so a failure leaves no
            // partial tablet and no orphaned pattern rows — no post-hoc
            // cleanup needed.
            let (tablet_id, item_count) = self
                .run_pipeline(plugin.as_ref(), &period_key, now, recovered)
                .await?;
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
            Ok(DispatchOutcome::Generated {
                tablet_id,
                item_count,
            })
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

    /// Run the gather → detect → compose → write pipeline. Returns the
    /// tablet id and the number of items written.
    ///
    /// **Atomicity (ARAWN-T-0479):** every persisted row of a dispatch —
    /// the tablet, its detected-pattern rows, and all its items — is written
    /// inside ONE SQLite transaction in step 10, AFTER the slow LLM compose
    /// returns. A failure anywhere in that phase rolls the whole thing back,
    /// so there's no half-written tablet and no orphaned pattern rows.
    /// Crucially the transaction does NOT span gather/compose, so it never
    /// holds the `arawn.db` write lock across the (multi-second) LLM call —
    /// the lock-starvation regression the per-item auto-commit was guarding
    /// against (see the comment in `dispatch_with`).
    async fn run_pipeline(
        &self,
        plugin: &dyn Ceremony,
        period_key: &str,
        now: chrono::DateTime<Utc>,
        recovered: bool,
    ) -> Result<(String, usize), CeremonyError> {
        let tablet_id = format!("{}-{period_key}", plugin.kind());

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

        // 7: gather (deterministic, read-only).
        let mut facts: GatheredFacts = plugin.gather(&ctx).await?;

        // 8: pattern detector (optional). Patterns are pre-assigned ids and
        // injected into `facts.payload.patterns_detected` so the compose LLM
        // can cite them — but the ROWS are deferred to the write transaction
        // below, so a compose failure can't orphan them. (Re-injection was
        // added in I-0049 T-0316 when priority_completion_ratio fired in the
        // DB but never reached the retro's patterns section.)
        let mut pending_patterns: Vec<(String, DetectedPattern)> = Vec::new();
        if let Some(detector) = plugin.patterns() {
            let patterns = detector.detect(&ctx).await?;
            let mut written: Vec<serde_json::Value> = Vec::with_capacity(patterns.len());
            for pattern in patterns {
                let id = Uuid::new_v4().to_string();
                written.push(serde_json::json!({
                    "id": id,
                    "iso_week": pattern.iso_week,
                    "pattern_key": pattern.pattern_key,
                    "magnitude": pattern.magnitude,
                    "payload": pattern.payload,
                }));
                pending_patterns.push((id, pattern));
            }
            if let Some(payload_obj) = facts.payload.as_object_mut() {
                payload_obj.insert(
                    "patterns_detected".to_string(),
                    serde_json::Value::Array(written),
                );
            }
        }

        // 9: compose, gated through the process-wide LLM resource gate.
        // No DB lock is held here.
        let new_items = {
            let _permit = arawn_llm::gate::acquire_local()
                .await
                .map_err(|e| CeremonyError::Llm(format!("llm gate refused acquire: {e:?}")))?;
            plugin.compose(&ctx, facts).await?
        };

        // 10: write phase — tablet + pattern rows + items, all in ONE
        // transaction. Any error rolls back the entire dispatch.
        let item_count = {
            let mut guard = self
                .conn
                .0
                .lock()
                .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
            let tx = guard
                .transaction()
                .map_err(|e| CeremonyError::Storage(format!("begin dispatch tx: {e}")))?;

            insert_tablet(&tx, &tablet_id, plugin.kind(), period_key, now, recovered)?;
            for (id, pattern) in &pending_patterns {
                write_pattern_row_tx(&tx, id, pattern)?;
            }
            let mut ordinal_by_section: std::collections::HashMap<String, i32> =
                std::collections::HashMap::new();
            let mut count = 0usize;
            for item in &new_items {
                match item {
                    NewItem::Composed(c) => write_composed_item(&tx, c, &mut ordinal_by_section)?,
                    NewItem::User(u) => write_user_item(&tx, u, &mut ordinal_by_section)?,
                }
                count += 1;
            }
            tx.commit()
                .map_err(|e| CeremonyError::Storage(format!("commit dispatch tx: {e}")))?;
            count
        };

        // 11: emit pattern events only after the rows are durably committed.
        if let Some(events) = &self.events {
            for (id, pattern) in &pending_patterns {
                emit_event(
                    events,
                    CeremonyEvent::PatternDetected {
                        pattern_id: id.clone(),
                        iso_week: pattern.iso_week.clone(),
                        pattern_key: pattern.pattern_key.clone(),
                    },
                );
            }
        }

        Ok((tablet_id, item_count))
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

/// Delete a tablet and all of its child rows. Used by the `force`
/// regenerate path (ARAWN-T-0479) to clear an existing open tablet before a
/// fresh dispatch.
///
/// The ceremony schema declares `ON DELETE CASCADE` on every child FK, but
/// the ceremony connection doesn't enable `PRAGMA foreign_keys`, so the
/// cascade never fires — we delete the children by hand, in one transaction,
/// to avoid orphaning `ceremony_items` / `_sections` / `_priorities` /
/// `_diary` rows. (`ceremony_todos_rolling` is a view since V9, not a table.)
fn delete_tablet(conn: &ConnHandle, tablet_id: &str) -> Result<(), CeremonyError> {
    let mut guard = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
    let tx = guard
        .transaction()
        .map_err(|e| CeremonyError::Storage(format!("begin delete-tablet tx: {e}")))?;
    for sql in [
        "DELETE FROM ceremony_items WHERE tablet_id = ?1",
        "DELETE FROM ceremony_priorities WHERE tablet_id = ?1",
        "DELETE FROM ceremony_diary WHERE tablet_id = ?1",
        "DELETE FROM ceremony_sections WHERE tablet_id = ?1",
        "DELETE FROM ceremony_tablets WHERE id = ?1",
    ] {
        tx.execute(sql, params![tablet_id])
            .map_err(|e| CeremonyError::Storage(format!("delete tablet child rows: {e}")))?;
    }
    tx.commit()
        .map_err(|e| CeremonyError::Storage(format!("commit delete-tablet tx: {e}")))?;
    Ok(())
}

// ARAWN-T-0477/T-0479: the tablet/pattern/item writes run inside a single
// transaction (see `run_pipeline`), so these helpers take a borrowed
// `&Connection` (a `&Transaction` derefs to one) rather than locking the
// shared handle themselves. The lock is acquired once, around the whole
// write phase, AFTER the slow LLM compose call — never across it.
fn insert_tablet(
    conn: &Connection,
    tablet_id: &str,
    kind: &str,
    period_key: &str,
    now: chrono::DateTime<Utc>,
    recovered: bool,
) -> Result<(), CeremonyError> {
    conn.execute(
        "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, lenses_scanned, recovered) \
         VALUES (?1, ?2, ?3, ?4, 'open', '[]', ?5)",
        params![tablet_id, kind, period_key, now.to_rfc3339(), recovered as i64],
    )
    .map_err(|e| CeremonyError::Storage(format!("insert tablet: {e}")))?;
    Ok(())
}

/// Write a detected-pattern row inside the dispatch transaction. The id is
/// pre-generated in `run_pipeline` (so the compose LLM can cite it before
/// the row is committed).
fn write_pattern_row_tx(
    conn: &Connection,
    id: &str,
    pattern: &DetectedPattern,
) -> Result<(), CeremonyError> {
    conn.execute(
        "INSERT INTO ceremony_patterns_detected (id, iso_week, pattern_key, magnitude, payload, surfaced_in_retro) \
         VALUES (?1, ?2, ?3, ?4, ?5, 0)",
        params![id, &pattern.iso_week, &pattern.pattern_key, pattern.magnitude, pattern.payload.to_string()],
    )
    .map_err(|e| CeremonyError::Storage(format!("insert pattern row: {e}")))?;
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
    conn: &Connection,
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
    conn: &Connection,
    item: &UserItem,
    ordinal_by_section: &mut std::collections::HashMap<String, i32>,
) -> Result<(), CeremonyError> {
    let _ = next_ordinal(ordinal_by_section, &item.section_key);
    let body = item.body.to_string();
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
            DispatchOutcome::Generated { tablet_id, .. } => {
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

    /// Detects exactly one pattern — used to prove pattern rows roll back
    /// with the rest of a failed dispatch (ARAWN-T-0479).
    struct OnePattern;
    #[async_trait]
    impl crate::plugin::PatternDetector for OnePattern {
        async fn detect(
            &self,
            _ctx: &dyn CeremonyCtx,
        ) -> Result<Vec<DetectedPattern>, CeremonyError> {
            Ok(vec![DetectedPattern {
                iso_week: "2026-W20".into(),
                pattern_key: "test_pattern".into(),
                magnitude: 1.0,
                payload: json!({}),
            }])
        }
    }

    /// A plugin with a pattern detector + a configurable item set.
    struct PatternPlugin {
        items: std::sync::Mutex<Vec<NewItem>>,
        detector: OnePattern,
    }
    #[async_trait]
    impl Ceremony for PatternPlugin {
        fn kind(&self) -> &'static str {
            "retro"
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
        fn patterns(&self) -> Option<&dyn crate::plugin::PatternDetector> {
            Some(&self.detector)
        }
    }

    #[tokio::test]
    async fn failed_dispatch_rolls_back_pattern_rows() {
        // ARAWN-T-0479: a mid-write failure rolls back the WHOLE dispatch,
        // including detected-pattern rows that older code wrote before
        // compose and then orphaned.
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        let items = vec![
            item_composed("retro-2026-W20", "what_happened", "sig-1"),
            item_composed("retro-2026-W20", "what_happened", ""), // missing citation → fail
        ];
        reg.register(Arc::new(PatternPlugin {
            items: std::sync::Mutex::new(items),
            detector: OnePattern,
        }))
        .unwrap();
        let disp = EngineDispatcher::new(conn.clone(), reg);

        let err = disp.dispatch("retro").await.unwrap_err();
        assert!(matches!(err, CeremonyError::MissingCitation(_)));
        assert_eq!(count_rows(&conn, "ceremony_tablets"), 0);
        assert_eq!(count_rows(&conn, "ceremony_items"), 0);
        assert_eq!(
            count_rows(&conn, "ceremony_patterns_detected"),
            0,
            "the pattern row must roll back with the failed dispatch"
        );
    }

    /// Composes one user item every run (idempotent — no `mem::take`), so a
    /// re-dispatch produces the same shape. Backs the force-regenerate test.
    struct RepeatPlugin;
    #[async_trait]
    impl Ceremony for RepeatPlugin {
        fn kind(&self) -> &'static str {
            "retro"
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
            Ok(vec![item_user("retro-2026-W20", "diary")])
        }
    }

    #[tokio::test]
    async fn force_regenerates_open_tablet() {
        // ARAWN-T-0479: an open (never-reviewed) tablet is skipped on a
        // normal re-dispatch but regenerated under `force` — the old rows
        // are deleted, not accumulated.
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        reg.register(Arc::new(RepeatPlugin)).unwrap();
        let disp = EngineDispatcher::new(conn.clone(), reg);
        let today = Utc::now().date_naive();

        let out = disp.dispatch_with("retro", today, false).await.unwrap();
        assert!(matches!(
            out,
            DispatchOutcome::Generated { item_count: 1, .. }
        ));
        assert_eq!(count_rows(&conn, "ceremony_tablets"), 1);
        assert_eq!(count_rows(&conn, "ceremony_items"), 1);

        // No force → the open tablet is left alone.
        let out = disp.dispatch_with("retro", today, false).await.unwrap();
        assert!(matches!(out, DispatchOutcome::Skipped { .. }));
        assert_eq!(count_rows(&conn, "ceremony_items"), 1);

        // Force → regenerate. Still exactly one tablet + one item (the prior
        // open tablet's rows were deleted first, not duplicated).
        let out = disp.dispatch_with("retro", today, true).await.unwrap();
        assert!(matches!(out, DispatchOutcome::Generated { .. }));
        assert_eq!(count_rows(&conn, "ceremony_tablets"), 1);
        assert_eq!(count_rows(&conn, "ceremony_items"), 1);
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
