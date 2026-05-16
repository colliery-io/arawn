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
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use crate::CeremonyError;
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
}

impl EngineDispatcher {
    pub fn new(conn: ConnHandle, registry: PluginRegistry) -> Self {
        Self { conn, registry }
    }
}

#[async_trait]
impl CeremonyDispatcher for EngineDispatcher {
    async fn dispatch(&self, kind: &str) -> Result<DispatchOutcome, CeremonyError> {
        // 1–2: plugin + period
        let plugin = self
            .registry
            .get(kind)
            .ok_or_else(|| CeremonyError::Other(format!("no plugin registered for kind '{kind}'")))?;
        let now = Utc::now();
        let period_key = plugin.period_key(now);

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
                reason: format!("tablet for ({kind}, {period_key}) already open — refusing to overwrite"),
            });
        }

        // 4: BEGIN. Everything below either commits at the end or
        // rolls back on the way out.
        begin(&self.conn)?;

        // Wrap the rest in a closure so we can ROLLBACK on any err.
        let result = self.run_pipeline(plugin.as_ref(), &period_key, now).await;
        match result {
            Ok(tablet_id) => {
                commit(&self.conn)?;
                Ok(DispatchOutcome::Generated { tablet_id })
            }
            Err(e) => {
                let _ = rollback(&self.conn);
                Err(e)
            }
        }
    }
}

impl EngineDispatcher {
    async fn run_pipeline(
        &self,
        plugin: &dyn Ceremony,
        period_key: &str,
        now: chrono::DateTime<Utc>,
    ) -> Result<String, CeremonyError> {
        // 5: insert the tablet.
        let tablet_id = format!("{}-{period_key}", plugin.kind());
        insert_tablet(&self.conn, &tablet_id, plugin.kind(), period_key, now)?;

        // 6: construct ctx.
        let ctx = EngineCtx::new(self.conn.clone(), tablet_id.clone(), period_key.to_string());

        // 7: gather (deterministic).
        let facts: GatheredFacts = plugin.gather(&ctx).await?;

        // 8: pattern detector (optional).
        if let Some(detector) = plugin.patterns() {
            let patterns = detector.detect(&ctx).await?;
            for pattern in patterns {
                let _id = ctx.write_pattern_row(pattern).await?;
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
                NewItem::Composed(c) => write_composed_item(&self.conn, &c, &mut ordinal_by_section)?,
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
}

impl EngineCtx {
    pub fn new(conn: ConnHandle, tablet_id: String, period_key: String) -> Self {
        Self {
            conn,
            tablet_id,
            period_key,
        }
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

    async fn write_pattern_row(
        &self,
        pattern: DetectedPattern,
    ) -> Result<String, CeremonyError> {
        let id = Uuid::new_v4().to_string();
        let payload = pattern.payload.to_string();
        let conn = self.conn.0.lock().map_err(|_| {
            CeremonyError::Storage("connection mutex poisoned".to_string())
        })?;
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

fn insert_tablet(
    conn: &ConnHandle,
    tablet_id: &str,
    kind: &str,
    period_key: &str,
    now: chrono::DateTime<Utc>,
) -> Result<(), CeremonyError> {
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
    conn.execute(
        "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
         VALUES (?1, ?2, ?3, ?4, 'open', '[]')",
        params![tablet_id, kind, period_key, now.to_rfc3339()],
    )
    .map_err(|e| CeremonyError::Storage(format!("insert tablet: {e}")))?;
    Ok(())
}

fn next_ordinal(
    ordinal_by_section: &mut std::collections::HashMap<String, i32>,
    section_key: &str,
) -> i32 {
    let next = ordinal_by_section.entry(section_key.to_string()).or_insert(-1);
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

fn begin(conn: &ConnHandle) -> Result<(), CeremonyError> {
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| CeremonyError::Storage(format!("BEGIN: {e}")))?;
    Ok(())
}

fn commit(conn: &ConnHandle) -> Result<(), CeremonyError> {
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
    conn.execute("COMMIT", [])
        .map_err(|e| CeremonyError::Storage(format!("COMMIT: {e}")))?;
    Ok(())
}

fn rollback(conn: &ConnHandle) -> Result<(), CeremonyError> {
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".to_string()))?;
    conn.execute("ROLLBACK", [])
        .map_err(|e| CeremonyError::Storage(format!("ROLLBACK: {e}")))?;
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
        c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
            .unwrap()
    }

    // --- Tests ---

    #[tokio::test]
    async fn happy_path_writes_tablet_and_composed_item_with_citation() {
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        // Tablet id format = "{kind}-{period_key}" → "retro-2026-W20"
        let items = vec![item_composed("retro-2026-W20", "what_happened", "sig-1")];
        reg.register(Arc::new(ScriptedPlugin::new("retro", items))).unwrap();
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
        reg.register(Arc::new(ScriptedPlugin::new("retro", items))).unwrap();
        let disp = EngineDispatcher::new(conn.clone(), reg);
        let err = disp.dispatch("retro").await.unwrap_err();
        assert!(matches!(err, CeremonyError::MissingCitation(_)));
        // Rollback: no tablet, no items.
        assert_eq!(count_rows(&conn, "ceremony_tablets"), 0);
        assert_eq!(count_rows(&conn, "ceremony_items"), 0);
    }

    #[tokio::test]
    async fn user_item_without_citation_is_accepted() {
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        let items = vec![item_user("retro-2026-W20", "diary")];
        reg.register(Arc::new(ScriptedPlugin::new("retro", items))).unwrap();
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
    async fn write_pattern_row_returns_id_and_writes() {
        let (_tmp, conn) = open_test_db();
        let ctx = EngineCtx::new(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
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
