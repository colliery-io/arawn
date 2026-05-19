//! `CeremonyService` — read + mutate surface for ceremony tablets.
//!
//! The methods here mirror the WS-RPC method names declared in
//! I-0043 (`ceremonies.get_today`, `ceremonies.list_items`,
//! `ceremonies.patch_item`, `ceremonies.add_item`,
//! `ceremonies.run`, `ceremonies.list_notifications`). The
//! binary's RPC dispatcher will wire these methods 1:1 into the
//! existing WS-RPC surface when ceremonies are integrated.
//!
//! Keeping the implementation in this crate (instead of jumping
//! straight to `arawn-service::ArawnService`) keeps T-0283 testable
//! without dragging in the full LocalService surface. The trait
//! integration lands when the binary wires the engine.
//!
//! Config CRUD (`ceremonies.list_config` / `config_update`) is
//! deferred — it needs a `ceremony_config` table that's not in V6.
//! Filed as a follow-up.

use std::sync::Arc;

use chrono::Utc;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::CeremonyError;
use crate::engine::ConnHandle;
use crate::events::{CeremonyEvent, CeremonyEventSender, emit as emit_event};
use crate::runner::{CeremonyDispatcher, DispatchOutcome};
use crate::types::{ItemKind, TabletStatus};

/// One tablet as the RPC clients see it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TabletDto {
    pub id: String,
    pub kind: String,
    pub period_key: String,
    pub generated_at: String,
    pub status: String,
    pub workstreams_scanned: serde_json::Value,
    pub priorities_confirmed_at: Option<String>,
    /// True when the tablet was composed for a historical date
    /// (back-fill on boot, manual dispatch_for), rather than by
    /// the live cron tick. UI can surface a "recovered" badge.
    #[serde(default)]
    pub recovered: bool,
}

/// One item row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemDto {
    pub id: String,
    pub tablet_id: String,
    pub section_key: String,
    pub ordinal: i32,
    pub kind: String,
    pub body: serde_json::Value,
    pub citation_id: Option<String>,
    pub done_at: Option<String>,
    pub created_at: String,
}

/// Notification surface: tablets the user has yet to interact with.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationDto {
    pub tablet_id: String,
    pub kind: String,
    pub period_key: String,
    pub status: String,
    pub generated_at: String,
}

/// Mutation payload for `patch_item`. Only the fields the client
/// actually wants to change get a value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemPatch {
    /// Toggle done — Some(true) sets `done_at` to now; Some(false)
    /// clears it; None leaves it untouched.
    #[serde(default)]
    pub done: Option<bool>,
    /// Replace the item body. Pass `None` to leave it untouched.
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

/// Payload for `add_item`. User-write path (no citation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddItemRequest {
    pub tablet_id: String,
    pub section_key: String,
    pub kind: ItemKind,
    pub body: serde_json::Value,
}

/// One priority row in `ceremony_priorities` or a yet-unconfirmed
/// candidate item in the `priorities` section of a weekly tablet.
/// `source` discriminates between them: `"confirmed"` for rows from
/// `ceremony_priorities`, `"candidate"` for items awaiting confirm.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriorityDto {
    pub id: String,
    pub tablet_id: String,
    pub body: serde_json::Value,
    pub rationale: String,
    pub citation_id: Option<String>,
    pub confirmed_at: Option<String>,
    pub done_at: Option<String>,
    pub ordinal: i32,
    pub source: String,
}

/// Payload for `add_priority`. User-write path — no `citation_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPriorityRequest {
    pub tablet_id: String,
    pub body: serde_json::Value,
    pub rationale: String,
}

/// The methods correspond 1:1 to the `ceremonies.*` WS-RPC method
/// names called out in I-0043. The binary's RPC dispatcher routes
/// JSON-RPC calls to these methods.
#[derive(Clone)]
pub struct CeremonyService {
    conn: ConnHandle,
    dispatcher: Arc<dyn CeremonyDispatcher>,
    events: Option<CeremonyEventSender>,
}

impl CeremonyService {
    pub fn new(conn: ConnHandle, dispatcher: Arc<dyn CeremonyDispatcher>) -> Self {
        Self {
            conn,
            dispatcher,
            events: None,
        }
    }

    /// Attach the event sender. The dispatcher should already have
    /// one; this clones the same channel so item-mutation events
    /// fire on the same broadcast as tablet generation.
    pub fn with_events(mut self, sender: CeremonyEventSender) -> Self {
        self.events = Some(sender);
        self
    }

    /// `ceremonies.set_retro_cadence` — persist a new retro cadence
    /// + anchor to `ceremony_config`. Applied to the live plugin on
    /// next restart. (Hot-applying would need interior mutability on
    /// the plugin Arc; deferred until that's actually painful.)
    ///
    /// `cadence_str` accepts `"weekly"` / `"biweekly"` / `"monthly"`
    /// (case-insensitive). For biweekly the anchor is set to "today's
    /// Monday" if `anchor` is `None`; for weekly/monthly the anchor
    /// is cleared.
    pub fn set_retro_cadence(
        &self,
        cadence_str: &str,
        anchor: Option<chrono::NaiveDate>,
    ) -> Result<crate::plugins::RetroCadence, CeremonyError> {
        let cadence = crate::plugins::RetroCadence::parse(cadence_str).ok_or_else(|| {
            CeremonyError::Other(format!(
                "unknown retro cadence '{cadence_str}' (want weekly/biweekly/monthly)"
            ))
        })?;
        let resolved_anchor = match cadence {
            crate::plugins::RetroCadence::Biweekly => Some(anchor.unwrap_or_else(|| {
                use chrono::Datelike;
                let today = Utc::now().date_naive();
                let offset = today.weekday().num_days_from_monday() as i64;
                today - chrono::Duration::days(offset)
            })),
            _ => None,
        };
        crate::plugins::RetroCeremony::save_cadence(&self.conn, cadence, resolved_anchor)?;
        Ok(cadence)
    }

    /// `ceremonies.get_today` — today's daily tablet, if any.
    /// Today is the UTC date — production may want to substitute
    /// a local-zone date once a user-zone config exists.
    pub fn get_today(&self) -> Result<Option<TabletDto>, CeremonyError> {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        self.get_by_period("daily", &today)
    }

    /// `ceremonies.get_by_period` — specific tablet.
    pub fn get_by_period(
        &self,
        kind: &str,
        period_key: &str,
    ) -> Result<Option<TabletDto>, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        conn.query_row(
            "SELECT id, kind, period_key, generated_at, status, workstreams_scanned, priorities_confirmed_at, recovered \
             FROM ceremony_tablets WHERE kind = ?1 AND period_key = ?2",
            params![kind, period_key],
            row_to_tablet,
        )
        .optional()
        .map_err(|e| CeremonyError::Storage(format!("get_by_period: {e}")))
    }

    /// `ceremonies.list_items` — items in a tablet, optionally
    /// filtered by section_key. Sorted by `(section_key, ordinal)`.
    pub fn list_items(
        &self,
        tablet_id: &str,
        section_key: Option<&str>,
    ) -> Result<Vec<ItemDto>, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, tablet_id, section_key, ordinal, kind, body, citation_id, done_at, created_at \
                 FROM ceremony_items WHERE tablet_id = ?1 \
                 AND (?2 IS NULL OR section_key = ?2) \
                 ORDER BY section_key, ordinal",
            )
            .map_err(|e| CeremonyError::Storage(format!("list_items prepare: {e}")))?;
        let rows = stmt
            .query_map(params![tablet_id, section_key], row_to_item)
            .map_err(|e| CeremonyError::Storage(format!("list_items query: {e}")))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| CeremonyError::Storage(format!("list_items row: {e}")))?);
        }
        Ok(out)
    }

    /// `ceremonies.patch_item` — toggle done, edit body. Returns
    /// the updated row.
    pub fn patch_item(&self, item_id: &str, patch: ItemPatch) -> Result<ItemDto, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        if let Some(done) = patch.done {
            let done_at = if done {
                Some(Utc::now().to_rfc3339())
            } else {
                None
            };
            conn.execute(
                "UPDATE ceremony_items SET done_at = ?1 WHERE id = ?2",
                params![done_at, item_id],
            )
            .map_err(|e| CeremonyError::Storage(format!("patch_item done: {e}")))?;
        }
        if let Some(body) = patch.body {
            conn.execute(
                "UPDATE ceremony_items SET body = ?1 WHERE id = ?2",
                params![body.to_string(), item_id],
            )
            .map_err(|e| CeremonyError::Storage(format!("patch_item body: {e}")))?;
        }
        let dto = conn
            .query_row(
                "SELECT id, tablet_id, section_key, ordinal, kind, body, citation_id, done_at, created_at \
                 FROM ceremony_items WHERE id = ?1",
                params![item_id],
                row_to_item,
            )
            .map_err(|e| CeremonyError::Storage(format!("patch_item reload: {e}")))?;
        drop(conn);
        if let Some(events) = &self.events {
            emit_event(
                events,
                CeremonyEvent::ItemUpdated {
                    item_id: dto.id.clone(),
                    tablet_id: dto.tablet_id.clone(),
                },
            );
        }
        Ok(dto)
    }

    /// `ceremonies.add_item` — user-write path. Inserts an item with
    /// NULL `citation_id`. The next ordinal in the section is picked
    /// automatically.
    pub fn add_item(&self, req: AddItemRequest) -> Result<ItemDto, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        let next_ordinal: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(ordinal) + 1, 0) FROM ceremony_items \
                 WHERE tablet_id = ?1 AND section_key = ?2",
                params![&req.tablet_id, &req.section_key],
                |row| row.get(0),
            )
            .map_err(|e| CeremonyError::Storage(format!("next_ordinal: {e}")))?;
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO ceremony_items (id, tablet_id, section_key, ordinal, kind, body, citation_id, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
            params![
                &id,
                &req.tablet_id,
                &req.section_key,
                next_ordinal,
                kind_str(&req.kind),
                req.body.to_string(),
                &created_at,
            ],
        )
        .map_err(|e| CeremonyError::Storage(format!("add_item insert: {e}")))?;
        let dto = ItemDto {
            id: id.clone(),
            tablet_id: req.tablet_id.clone(),
            section_key: req.section_key,
            ordinal: next_ordinal,
            kind: kind_str(&req.kind).to_string(),
            body: req.body,
            citation_id: None,
            done_at: None,
            created_at,
        };
        drop(conn);
        if let Some(events) = &self.events {
            emit_event(
                events,
                CeremonyEvent::ItemUpdated {
                    item_id: id,
                    tablet_id: req.tablet_id,
                },
            );
        }
        Ok(dto)
    }

    /// `ceremonies.run` — manual trigger; idempotent per period_key.
    pub async fn run(&self, kind: &str) -> Result<DispatchOutcome, CeremonyError> {
        self.dispatcher.dispatch(kind).await
    }

    /// User-write helper used by `daily_add_todo`: insert a row into
    /// `ceremony_todos_rolling` so future daily generations + the
    /// retro `rollover_heat` detector pick it up. The caller is
    /// responsible for the parallel `ceremony_items` insert via
    /// `add_item` (the rolling table is purely the persistence layer
    /// for cross-tablet todo continuity). Returns the new
    /// `todo_id`. No event is emitted — matches `add_item`'s
    /// item-DTO-return shape.
    pub fn add_rolling_todo(
        &self,
        body: &str,
        origin_tablet_id: &str,
    ) -> Result<String, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        let todo_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let attrs = serde_json::json!({
            "origin_tablet_id": origin_tablet_id,
            "last_seen_tablet_id": origin_tablet_id,
        })
        .to_string();
        // Post-cutover (V9): ceremony_todos_rolling is a read-only
        // view over `todos WHERE kind='rollover'`. Writes go to
        // `todos` directly with the rollover-shaped attrs.
        conn.execute(
            "INSERT INTO todos \
                 (id, body, rationale, kind, workstream, created_at, due_at, done_at, archived_at, attrs) \
             VALUES (?1, ?2, NULL, 'rollover', NULL, ?3, NULL, NULL, NULL, ?4)",
            params![&todo_id, body, &created_at, &attrs],
        )
        .map_err(|e| CeremonyError::Storage(format!("add_rolling_todo insert: {e}")))?;
        Ok(todo_id)
    }

    /// `ceremonies.confirm_priority` — promote a priority candidate
    /// item into a confirmed row in `ceremony_priorities`. Idempotent:
    /// re-calling on the same item returns the existing priority row.
    /// Emits `PriorityConfirmed`.
    pub fn confirm_priority(&self, item_id: &str) -> Result<PriorityDto, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        // Verify the item exists and is a priority candidate.
        let row: Option<(String, String, String, Option<String>, i32, String)> = conn
            .query_row(
                "SELECT tablet_id, section_key, kind, citation_id, ordinal, body \
                 FROM ceremony_items WHERE id = ?1",
                params![item_id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| CeremonyError::Storage(format!("confirm_priority lookup: {e}")))?;
        let (tablet_id, section_key, kind, citation_id_opt, ordinal, body_str) =
            row.ok_or_else(|| {
                CeremonyError::invalid_tablet_state(format!("no item with id '{item_id}'"))
            })?;
        // Kind is intentionally not constrained — the weekly plugin
        // emits priority candidates with kind="pattern" (matching the
        // LLM-claim shape). What identifies an item as a priority
        // candidate is its section_key.
        let _ = kind;
        if section_key != "priorities" {
            return Err(CeremonyError::invalid_tablet_state(format!(
                "item '{item_id}' is not a priority candidate \
                 (section_key='{section_key}', kind='{kind}')"
            )));
        }

        // Idempotency: if a priority row already cites this item via
        // its linked todo's attrs.citation_id, return it.
        let existing: Option<(String, Option<String>, Option<String>, Option<String>, i32, String)> = conn
            .query_row(
                "SELECT cp.id, t.rationale, cp.confirmed_at, t.done_at, cp.ordinal, t.body \
                 FROM ceremony_priorities cp \
                 JOIN todos t ON t.id = cp.todo_id \
                 WHERE json_extract(t.attrs, '$.citation_id') = ?1",
                params![item_id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| CeremonyError::Storage(format!("confirm_priority idempotency: {e}")))?;
        if let Some((id, rationale, confirmed_at, done_at, ord, body_s)) = existing {
            let body = serde_json::from_str(&body_s).unwrap_or(serde_json::Value::Null);
            return Ok(PriorityDto {
                id,
                tablet_id,
                body,
                rationale: rationale.unwrap_or_default(),
                citation_id: Some(item_id.to_string()),
                confirmed_at,
                done_at,
                ordinal: ord,
                source: "confirmed".to_string(),
            });
        }

        let new_id = Uuid::new_v4().to_string();
        let new_todo_id = Uuid::new_v4().to_string();
        let confirmed_at = Utc::now().to_rfc3339();
        let priority_citation = item_id.to_string();
        // I-0049 T-0315 — preserve the LLM-generated rationale.
        // Weekly compose emits body: {text, rationale}; pull the
        // rationale out so it lands on `todos.rationale` rather
        // than being discarded. NULL when absent / empty so the
        // canonical "no rationale" state is consistent.
        let rationale: Option<String> = serde_json::from_str::<serde_json::Value>(&body_str)
            .ok()
            .and_then(|v| {
                v.get("rationale")
                    .and_then(|r| r.as_str())
                    .map(|s| s.to_string())
            })
            .filter(|s| !s.trim().is_empty());
        // Post-cutover: body/rationale/done_at live on the todo. The
        // priority row is a thin link carrying ordinal + confirmed_at.
        let attrs = serde_json::json!({
            "tablet_id": &tablet_id,
            "ordinal": ordinal,
            "confirmed_at": &confirmed_at,
            "citation_id": &priority_citation,
        })
        .to_string();
        conn.execute(
            "INSERT INTO todos \
                 (id, body, rationale, kind, workstream, created_at, due_at, done_at, archived_at, attrs) \
             VALUES (?1, ?2, ?3, 'weekly_priority', NULL, ?4, NULL, NULL, NULL, ?5)",
            params![&new_todo_id, &body_str, &rationale, &confirmed_at, &attrs],
        )
        .map_err(|e| CeremonyError::Storage(format!("confirm_priority todo insert: {e}")))?;
        conn.execute(
            "INSERT INTO ceremony_priorities \
                 (id, tablet_id, todo_id, confirmed_at, ordinal) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![&new_id, &tablet_id, &new_todo_id, &confirmed_at, ordinal],
        )
        .map_err(|e| CeremonyError::Storage(format!("confirm_priority insert: {e}")))?;
        drop(conn);
        let _ = citation_id_opt;
        if let Some(events) = &self.events {
            emit_event(
                events,
                CeremonyEvent::PriorityConfirmed {
                    priority_id: new_id.clone(),
                    tablet_id: tablet_id.clone(),
                },
            );
        }
        let body = serde_json::from_str(&body_str).unwrap_or(serde_json::Value::Null);
        Ok(PriorityDto {
            id: new_id,
            tablet_id,
            body,
            rationale: rationale.unwrap_or_default(),
            citation_id: Some(priority_citation),
            confirmed_at: Some(confirmed_at),
            done_at: None,
            ordinal,
            source: "confirmed".to_string(),
        })
    }

    /// `ceremonies.reject_priority` — delete a priority candidate item.
    /// Also removes any `ceremony_priorities` row that cites it (so a
    /// confirm-then-reject leaves no orphan). No event emitted.
    pub fn reject_priority(&self, item_id: &str) -> Result<(), CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        // Post-cutover: citation_id sits on the linked todo's attrs.
        // Delete the priority row whose linked todo cites this item;
        // ON DELETE CASCADE on todo_id keeps the todo too if we
        // explicitly drop it. We delete the todo first so its
        // mark-done events don't fire orphaned later.
        conn.execute(
            "DELETE FROM todos \
             WHERE id IN ( \
                 SELECT cp.todo_id FROM ceremony_priorities cp \
                 JOIN todos t ON t.id = cp.todo_id \
                 WHERE json_extract(t.attrs, '$.citation_id') = ?1 \
             )",
            params![item_id],
        )
        .map_err(|e| CeremonyError::Storage(format!("reject_priority todo delete: {e}")))?;
        // FK ON DELETE CASCADE on todo_id removes the priority row;
        // belt-and-braces explicit delete in case the todo never
        // existed (e.g. legacy paths) so we still clean orphans.
        conn.execute(
            "DELETE FROM ceremony_priorities \
             WHERE id IN ( \
                 SELECT cp.id FROM ceremony_priorities cp \
                 LEFT JOIN todos t ON t.id = cp.todo_id \
                 WHERE t.id IS NULL \
             )",
            [],
        )
        .map_err(|e| CeremonyError::Storage(format!("reject_priority priority delete: {e}")))?;
        conn.execute("DELETE FROM ceremony_items WHERE id = ?1", params![item_id])
            .map_err(|e| CeremonyError::Storage(format!("reject_priority item delete: {e}")))?;
        Ok(())
    }

    /// `ceremonies.add_priority` — user-write path. Inserts directly
    /// into `ceremony_priorities` with `citation_id = NULL`. Ordinal
    /// is the next free slot for the tablet.
    pub fn add_priority(&self, req: AddPriorityRequest) -> Result<PriorityDto, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        let next_ordinal: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(ordinal) + 1, 0) FROM ceremony_priorities \
                 WHERE tablet_id = ?1",
                params![&req.tablet_id],
                |row| row.get(0),
            )
            .map_err(|e| CeremonyError::Storage(format!("add_priority next_ordinal: {e}")))?;
        let id = Uuid::new_v4().to_string();
        let new_todo_id = Uuid::new_v4().to_string();
        let confirmed_at = Utc::now().to_rfc3339();
        let body_str = req.body.to_string();
        // User-write path — citation_id stays NULL on the todo attrs.
        let attrs = serde_json::json!({
            "tablet_id": &req.tablet_id,
            "ordinal": next_ordinal,
            "confirmed_at": &confirmed_at,
            "citation_id": serde_json::Value::Null,
        })
        .to_string();
        conn.execute(
            "INSERT INTO todos \
                 (id, body, rationale, kind, workstream, created_at, due_at, done_at, archived_at, attrs) \
             VALUES (?1, ?2, ?3, 'weekly_priority', NULL, ?4, NULL, NULL, NULL, ?5)",
            params![&new_todo_id, &body_str, &req.rationale, &confirmed_at, &attrs],
        )
        .map_err(|e| CeremonyError::Storage(format!("add_priority todo insert: {e}")))?;
        conn.execute(
            "INSERT INTO ceremony_priorities \
                 (id, tablet_id, todo_id, confirmed_at, ordinal) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![&id, &req.tablet_id, &new_todo_id, &confirmed_at, next_ordinal],
        )
        .map_err(|e| CeremonyError::Storage(format!("add_priority insert: {e}")))?;
        Ok(PriorityDto {
            id,
            tablet_id: req.tablet_id,
            body: req.body,
            rationale: req.rationale,
            citation_id: None,
            confirmed_at: Some(confirmed_at),
            done_at: None,
            ordinal: next_ordinal,
            source: "confirmed".to_string(),
        })
    }

    /// `ceremonies.list_priorities` — union of confirmed priorities
    /// and yet-unconfirmed candidate items for a tablet. Confirmed
    /// rows come first (sorted by ordinal); candidates follow.
    pub fn list_priorities(&self, tablet_id: &str) -> Result<Vec<PriorityDto>, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        let mut confirmed: Vec<PriorityDto> = {
            let mut stmt = conn
                .prepare(
                    "SELECT cp.id, cp.tablet_id, t.body, COALESCE(t.rationale, ''), \
                            json_extract(t.attrs, '$.citation_id') AS citation_id, \
                            cp.confirmed_at, t.done_at, cp.ordinal \
                     FROM ceremony_priorities cp \
                     JOIN todos t ON t.id = cp.todo_id \
                     WHERE cp.tablet_id = ?1 ORDER BY cp.ordinal",
                )
                .map_err(|e| {
                    CeremonyError::Storage(format!("list_priorities confirmed prepare: {e}"))
                })?;
            let rows = stmt
                .query_map(params![tablet_id], |row| {
                    let body_str: String = row.get(2)?;
                    let body = serde_json::from_str(&body_str).unwrap_or(serde_json::Value::Null);
                    Ok(PriorityDto {
                        id: row.get(0)?,
                        tablet_id: row.get(1)?,
                        body,
                        rationale: row.get(3)?,
                        citation_id: row.get(4)?,
                        confirmed_at: row.get(5)?,
                        done_at: row.get(6)?,
                        ordinal: row.get(7)?,
                        source: "confirmed".to_string(),
                    })
                })
                .map_err(|e| {
                    CeremonyError::Storage(format!("list_priorities confirmed query: {e}"))
                })?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| {
                    CeremonyError::Storage(format!("list_priorities confirmed row: {e}"))
                })?);
            }
            out
        };

        let mut candidates: Vec<PriorityDto> = {
            let mut stmt = conn
                .prepare(
                    // Kind unconstrained — section_key='priorities' is
                    // the canonical "candidate priority" marker. The
                    // weekly plugin emits these with kind='pattern'
                    // (its LLM-claim default); filtering on kind here
                    // would hide every real candidate.
                    "SELECT id, tablet_id, body, citation_id, ordinal \
                     FROM ceremony_items \
                     WHERE tablet_id = ?1 AND section_key = 'priorities' \
                       AND id NOT IN ( \
                           SELECT json_extract(t.attrs, '$.citation_id') \
                           FROM ceremony_priorities cp \
                           JOIN todos t ON t.id = cp.todo_id \
                           WHERE cp.tablet_id = ?1 \
                             AND json_extract(t.attrs, '$.citation_id') IS NOT NULL \
                       ) \
                     ORDER BY ordinal",
                )
                .map_err(|e| {
                    CeremonyError::Storage(format!("list_priorities candidates prepare: {e}"))
                })?;
            let rows = stmt
                .query_map(params![tablet_id], |row| {
                    let body_str: String = row.get(2)?;
                    let body = serde_json::from_str(&body_str).unwrap_or(serde_json::Value::Null);
                    Ok(PriorityDto {
                        id: row.get(0)?,
                        tablet_id: row.get(1)?,
                        body,
                        rationale: String::new(),
                        citation_id: row.get(3)?,
                        confirmed_at: None,
                        done_at: None,
                        ordinal: row.get(4)?,
                        source: "candidate".to_string(),
                    })
                })
                .map_err(|e| {
                    CeremonyError::Storage(format!("list_priorities candidates query: {e}"))
                })?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(|e| {
                    CeremonyError::Storage(format!("list_priorities candidates row: {e}"))
                })?);
            }
            out
        };

        confirmed.append(&mut candidates);
        Ok(confirmed)
    }

    /// `ceremonies.upsert_diary` — writes (or replaces) the user's
    /// diary entry on a retro tablet. The body is stored verbatim
    /// — no markdown parsing, no transformations. `word_count` is
    /// a simple whitespace split. On success, flips the tablet's
    /// status from `open` to `reviewed` and fires
    /// `CeremonyEvent::DiaryUpdated`.
    ///
    /// Idempotent: a re-upsert replaces the previous body, updates
    /// `written_at` to now, recomputes `word_count`. The tablet's
    /// status remains `reviewed` (re-upserting on a `reviewed`
    /// tablet doesn't bump it back to `open`).
    /// `ceremonies.get_diary` — returns the diary body for a retro
    /// tablet, or `None` if no diary has been written. Pure read;
    /// no events emitted.
    pub fn get_diary(&self, tablet_id: &str) -> Result<Option<String>, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        conn.query_row(
            "SELECT body FROM ceremony_diary WHERE tablet_id = ?1",
            params![tablet_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| CeremonyError::Storage(format!("get_diary: {e}")))
    }

    pub fn upsert_diary(&self, tablet_id: &str, body: &str) -> Result<(), CeremonyError> {
        let word_count = body.split_whitespace().count() as i64;
        let written_at = Utc::now().to_rfc3339();
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        // Verify the tablet exists + is a retro. Other ceremonies
        // don't accept diary entries — they don't have a diary
        // section.
        let tablet_kind: Option<String> = conn
            .query_row(
                "SELECT kind FROM ceremony_tablets WHERE id = ?1",
                params![tablet_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| CeremonyError::Storage(format!("upsert_diary lookup: {e}")))?;
        let kind = tablet_kind.ok_or_else(|| {
            CeremonyError::invalid_tablet_state(format!("no tablet with id '{tablet_id}'"))
        })?;
        if kind != "retro" {
            return Err(CeremonyError::invalid_tablet_state(format!(
                "diary only valid on retro tablets; '{tablet_id}' is kind '{kind}'"
            )));
        }

        // Upsert via INSERT OR REPLACE since tablet_id is the PK.
        conn.execute(
            "INSERT OR REPLACE INTO ceremony_diary (tablet_id, body, written_at, word_count) \
             VALUES (?1, ?2, ?3, ?4)",
            params![tablet_id, body, &written_at, word_count],
        )
        .map_err(|e| CeremonyError::Storage(format!("upsert_diary write: {e}")))?;

        // Flip status to 'reviewed' if it was 'open'. Leave
        // 'reviewed' and 'archived' alone (re-upsert shouldn't
        // un-archive or "reset" a tablet).
        conn.execute(
            "UPDATE ceremony_tablets SET status = 'reviewed' \
             WHERE id = ?1 AND status = 'open'",
            params![tablet_id],
        )
        .map_err(|e| CeremonyError::Storage(format!("upsert_diary status: {e}")))?;

        drop(conn);
        if let Some(events) = &self.events {
            emit_event(
                events,
                CeremonyEvent::DiaryUpdated {
                    tablet_id: tablet_id.to_string(),
                },
            );
        }
        Ok(())
    }

    /// `ceremonies.list_notifications` — tablets the user has not
    /// interacted with yet. Today this is `status = "open"`; the
    /// retro adds an `unreviewed` state on the Sunday transition
    /// (T-0289) which we exclude.
    pub fn list_notifications(&self) -> Result<Vec<NotificationDto>, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, period_key, status, generated_at \
                 FROM ceremony_tablets WHERE status = 'open' ORDER BY generated_at DESC",
            )
            .map_err(|e| CeremonyError::Storage(format!("list_notifications prepare: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(NotificationDto {
                    tablet_id: row.get(0)?,
                    kind: row.get(1)?,
                    period_key: row.get(2)?,
                    status: row.get(3)?,
                    generated_at: row.get(4)?,
                })
            })
            .map_err(|e| CeremonyError::Storage(format!("list_notifications query: {e}")))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(
                r.map_err(|e| CeremonyError::Storage(format!("list_notifications row: {e}")))?,
            );
        }
        Ok(out)
    }
}

// --- Row mappers ---

fn row_to_tablet(row: &rusqlite::Row<'_>) -> rusqlite::Result<TabletDto> {
    let workstreams_str: String = row.get(5)?;
    let workstreams_scanned = serde_json::from_str(&workstreams_str)
        .unwrap_or_else(|_| serde_json::Value::Array(Vec::new()));
    let recovered_i: i64 = row.get(7)?;
    Ok(TabletDto {
        id: row.get(0)?,
        kind: row.get(1)?,
        period_key: row.get(2)?,
        generated_at: row.get(3)?,
        status: row.get(4)?,
        workstreams_scanned,
        priorities_confirmed_at: row.get(6)?,
        recovered: recovered_i != 0,
    })
}

fn row_to_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<ItemDto> {
    let body_str: String = row.get(5)?;
    let body = serde_json::from_str(&body_str).unwrap_or(serde_json::Value::Null);
    Ok(ItemDto {
        id: row.get(0)?,
        tablet_id: row.get(1)?,
        section_key: row.get(2)?,
        ordinal: row.get(3)?,
        kind: row.get(4)?,
        body,
        citation_id: row.get(6)?,
        done_at: row.get(7)?,
        created_at: row.get(8)?,
    })
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

/// Tiny use-once helper so callers that only need to render a
/// status enum back into the wire-string don't have to pull
/// `kind_str` into scope.
#[allow(dead_code)]
fn status_str(s: TabletStatus) -> &'static str {
    s.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PluginRegistry;
    use crate::engine::EngineDispatcher;
    use crate::plugin::{Ceremony, CeremonyCtx, ComposedItem, CronSchedule, NewItem, UserItem};
    use crate::types::GatheredFacts;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn open_test_db() -> (TempDir, ConnHandle) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let _db = arawn_storage::Database::open(&db_path).expect("migrations");
        drop(_db);
        let conn = rusqlite::Connection::open(&db_path).expect("open conn");
        (tmp, ConnHandle::new(conn))
    }

    struct ScriptedPlugin {
        kind: &'static str,
        items: std::sync::Mutex<Vec<NewItem>>,
        period: String,
    }
    #[async_trait]
    impl Ceremony for ScriptedPlugin {
        fn kind(&self) -> &'static str {
            self.kind
        }
        fn period_key(&self, _now: chrono::DateTime<Utc>) -> String {
            self.period.clone()
        }
        fn period_window(
            &self,
            _period_key: &str,
        ) -> Result<(chrono::DateTime<Utc>, chrono::DateTime<Utc>), CeremonyError> {
            let now = chrono::Utc::now();
            Ok((now, now + chrono::Duration::days(7)))
        }
        fn default_schedule(&self) -> CronSchedule {
            CronSchedule::local("0 0 * * *")
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

    fn build_service_with_items(
        kind: &'static str,
        period: &str,
        tablet_id_prefix: &str,
    ) -> (TempDir, CeremonyService, String) {
        let (tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        // Predictable tablet id format = "{kind}-{period_key}"
        let tablet_id = format!("{kind}-{period}");
        let items = vec![
            NewItem::composed(ComposedItem {
                tablet_id: tablet_id.clone(),
                section_key: "what_happened".into(),
                ordinal: 0,
                kind: ItemKind::Pattern,
                body: json!({"text": "looks good"}),
                citation_id: "sig-1".into(),
            }),
            NewItem::user(UserItem {
                tablet_id: tablet_id.clone(),
                section_key: "diary".into(),
                ordinal: 0,
                kind: ItemKind::Freeform,
                body: json!({"text": ""}),
            }),
        ];
        reg.register(Arc::new(ScriptedPlugin {
            kind,
            items: std::sync::Mutex::new(items),
            period: period.into(),
        }))
        .unwrap();
        let dispatcher = Arc::new(EngineDispatcher::new(conn.clone(), reg));
        let service = CeremonyService::new(conn, dispatcher.clone());
        let _ = tablet_id_prefix; // unused but kept for naming intent
        (tmp, service, tablet_id)
    }

    #[tokio::test]
    async fn run_generates_and_get_by_period_reads_back() {
        let (_tmp, service, tablet_id) = build_service_with_items("retro", "2026-W20", "retro");
        let outcome = service.run("retro").await.unwrap();
        assert!(matches!(outcome, DispatchOutcome::Generated { .. }));
        let dto = service.get_by_period("retro", "2026-W20").unwrap().unwrap();
        assert_eq!(dto.id, tablet_id);
        assert_eq!(dto.status, "open");
    }

    #[tokio::test]
    async fn list_items_filters_by_section() {
        let (_tmp, service, tablet_id) = build_service_with_items("retro", "2026-W20", "retro");
        service.run("retro").await.unwrap();
        let all = service.list_items(&tablet_id, None).unwrap();
        assert_eq!(all.len(), 2);
        let only_diary = service.list_items(&tablet_id, Some("diary")).unwrap();
        assert_eq!(only_diary.len(), 1);
        assert_eq!(only_diary[0].section_key, "diary");
        assert!(only_diary[0].citation_id.is_none());
    }

    #[tokio::test]
    async fn patch_item_toggles_done() {
        let (_tmp, service, tablet_id) = build_service_with_items("retro", "2026-W20", "retro");
        service.run("retro").await.unwrap();
        let items = service
            .list_items(&tablet_id, Some("what_happened"))
            .unwrap();
        let id = &items[0].id;
        // Mark done.
        let patched = service
            .patch_item(
                id,
                ItemPatch {
                    done: Some(true),
                    body: None,
                },
            )
            .unwrap();
        assert!(patched.done_at.is_some());
        // Mark undone.
        let patched = service
            .patch_item(
                id,
                ItemPatch {
                    done: Some(false),
                    body: None,
                },
            )
            .unwrap();
        assert!(patched.done_at.is_none());
    }

    #[tokio::test]
    async fn add_item_inserts_user_row_with_null_citation_and_next_ordinal() {
        let (_tmp, service, tablet_id) = build_service_with_items("retro", "2026-W20", "retro");
        service.run("retro").await.unwrap();
        // The retro tablet has one "diary" item at ordinal 0. Adding
        // another should land at ordinal 1.
        let added = service
            .add_item(AddItemRequest {
                tablet_id: tablet_id.clone(),
                section_key: "diary".into(),
                kind: ItemKind::Freeform,
                body: json!({"text": "second"}),
            })
            .unwrap();
        assert_eq!(added.ordinal, 1);
        assert!(added.citation_id.is_none());
        let diary_items = service.list_items(&tablet_id, Some("diary")).unwrap();
        assert_eq!(diary_items.len(), 2);
    }

    #[tokio::test]
    async fn list_notifications_surfaces_open_tablets() {
        let (_tmp, service, _tablet_id) = build_service_with_items("retro", "2026-W20", "retro");
        service.run("retro").await.unwrap();
        let notes = service.list_notifications().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].kind, "retro");
        assert_eq!(notes[0].status, "open");
    }

    #[tokio::test]
    async fn get_today_returns_none_when_no_daily_tablet() {
        let (_tmp, conn) = open_test_db();
        let dispatcher = Arc::new(EngineDispatcher::new(conn.clone(), PluginRegistry::new()));
        let service = CeremonyService::new(conn, dispatcher);
        let today = service.get_today().unwrap();
        assert!(today.is_none());
    }

    #[tokio::test]
    async fn dispatch_emits_tablet_generated_event() {
        use crate::CeremonyEvent;
        let (tx, mut rx) = crate::event_channel();
        let (_tmp, conn) = open_test_db();
        let reg = PluginRegistry::new();
        let items = vec![NewItem::user(UserItem {
            tablet_id: "retro-2026-W20".into(),
            section_key: "diary".into(),
            ordinal: 0,
            kind: ItemKind::Freeform,
            body: json!({}),
        })];
        reg.register(Arc::new(ScriptedPlugin {
            kind: "retro",
            items: std::sync::Mutex::new(items),
            period: "2026-W20".into(),
        }))
        .unwrap();
        let dispatcher = Arc::new(EngineDispatcher::new(conn.clone(), reg).with_events(tx.clone()));
        let service = CeremonyService::new(conn, dispatcher.clone()).with_events(tx);
        service.run("retro").await.unwrap();
        let event = rx.recv().await.unwrap();
        match event {
            CeremonyEvent::TabletGenerated {
                tablet_id,
                kind,
                period_key,
            } => {
                assert_eq!(tablet_id, "retro-2026-W20");
                assert_eq!(kind, "retro");
                assert_eq!(period_key, "2026-W20");
            }
            other => panic!("expected TabletGenerated, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn upsert_diary_writes_row_and_flips_status() {
        let (_tmp, service, tablet_id) = build_service_with_items("retro", "2026-W20", "retro");
        service.run("retro").await.unwrap();
        service
            .upsert_diary(&tablet_id, "Felt productive.")
            .unwrap();
        // Diary row + tablet status = reviewed.
        let dto = service.get_by_period("retro", "2026-W20").unwrap().unwrap();
        assert_eq!(dto.status, "reviewed");
        // Read the diary row directly.
        let (conn, _, _) = service_internals(&service);
        let c = conn.0.lock().unwrap();
        let body: String = c
            .query_row(
                "SELECT body FROM ceremony_diary WHERE tablet_id = ?1",
                rusqlite::params![&tablet_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(body, "Felt productive.");
        let word_count: i64 = c
            .query_row(
                "SELECT word_count FROM ceremony_diary WHERE tablet_id = ?1",
                rusqlite::params![&tablet_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(word_count, 2);
    }

    #[tokio::test]
    async fn upsert_diary_is_idempotent_and_replaces_body() {
        let (_tmp, service, tablet_id) = build_service_with_items("retro", "2026-W20", "retro");
        service.run("retro").await.unwrap();
        service.upsert_diary(&tablet_id, "first version").unwrap();
        service.upsert_diary(&tablet_id, "rewritten").unwrap();
        // Read the body in its own scope so the lock drops before
        // the next service call (std::sync::Mutex doesn't re-enter).
        let body: String = {
            let (conn, _, _) = service_internals(&service);
            let c = conn.0.lock().unwrap();
            c.query_row(
                "SELECT body FROM ceremony_diary WHERE tablet_id = ?1",
                rusqlite::params![&tablet_id],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(body, "rewritten");
        // Status stays reviewed; doesn't ping-pong back to open.
        let dto = service.get_by_period("retro", "2026-W20").unwrap().unwrap();
        assert_eq!(dto.status, "reviewed");
    }

    #[tokio::test]
    async fn upsert_diary_rejects_non_retro_tablet() {
        let (_tmp, conn) = open_test_db();
        // Hand-roll a daily tablet so we can prove the kind check.
        let c = conn.0.lock().unwrap();
        c.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
             VALUES ('daily-1', 'daily', '2026-05-15', '2026-05-15T07:00:00Z', 'open', '[]')",
            [],
        )
        .unwrap();
        drop(c);
        let dispatcher = Arc::new(EngineDispatcher::new(conn.clone(), PluginRegistry::new()));
        let service = CeremonyService::new(conn, dispatcher);
        let err = service
            .upsert_diary("daily-1", "shouldn't work")
            .unwrap_err();
        assert!(matches!(err, CeremonyError::InvalidTabletState(_)));
    }

    #[tokio::test]
    async fn upsert_diary_rejects_unknown_tablet() {
        let (_tmp, conn) = open_test_db();
        let dispatcher = Arc::new(EngineDispatcher::new(conn.clone(), PluginRegistry::new()));
        let service = CeremonyService::new(conn, dispatcher);
        let err = service.upsert_diary("nope", "x").unwrap_err();
        assert!(matches!(err, CeremonyError::InvalidTabletState(_)));
    }

    #[tokio::test]
    async fn upsert_diary_emits_diary_updated_event() {
        use crate::CeremonyEvent;
        let (tx, mut rx) = crate::event_channel();
        let (_tmp, service, tablet_id) = build_service_with_items("retro", "2026-W20", "retro");
        let service = service.with_events(tx);
        service.run("retro").await.unwrap();
        service.upsert_diary(&tablet_id, "thoughts").unwrap();
        // Drain until we see DiaryUpdated.
        loop {
            let event = rx.recv().await.unwrap();
            if let CeremonyEvent::DiaryUpdated { tablet_id: tid } = event {
                assert_eq!(tid, tablet_id);
                break;
            }
        }
    }

    /// Tiny accessor for the in-test connection so the diary tests
    /// can read raw rows without going through more service
    /// methods. The service exposes the conn it was constructed
    /// with; we reach in for tests only.
    fn service_internals(service: &CeremonyService) -> (ConnHandle, (), ()) {
        // service.conn is private — clone it via a friend
        // accessor. Since this lives in the same module the
        // private field is reachable.
        (service.conn.clone(), (), ())
    }

    /// Build a weekly tablet with N priority candidate items via raw
    /// SQL. Priorities live on weekly tablets only — the
    /// `build_service_with_items` retro helper doesn't fit. Returns
    /// `(service, tablet_id, item_ids)`.
    fn build_weekly_with_priority_candidates(
        n: usize,
    ) -> (TempDir, CeremonyService, String, Vec<String>) {
        let (tmp, conn) = open_test_db();
        let tablet_id = "weekly-2026-W20".to_string();
        let mut item_ids = Vec::new();
        {
            let c = conn.0.lock().unwrap();
            c.execute(
                "INSERT INTO ceremony_tablets \
                 (id, kind, period_key, generated_at, status, workstreams_scanned) \
                 VALUES (?1, 'weekly', '2026-W20', '2026-05-11T07:00:00Z', 'open', '[]')",
                rusqlite::params![&tablet_id],
            )
            .unwrap();
            for i in 0..n {
                let id = format!("item-prio-{i}");
                c.execute(
                    "INSERT INTO ceremony_items \
                     (id, tablet_id, section_key, ordinal, kind, body, citation_id, created_at) \
                     VALUES (?1, ?2, 'priorities', ?3, 'priority', ?4, ?5, '2026-05-11T07:00:00Z')",
                    rusqlite::params![
                        &id,
                        &tablet_id,
                        i as i32,
                        json!({"text": format!("priority {i}")}).to_string(),
                        format!("sig-{i}"),
                    ],
                )
                .unwrap();
                item_ids.push(id);
            }
        }
        let reg = PluginRegistry::new();
        let dispatcher = Arc::new(EngineDispatcher::new(conn.clone(), reg));
        let service = CeremonyService::new(conn, dispatcher);
        (tmp, service, tablet_id, item_ids)
    }

    #[tokio::test]
    async fn confirm_priority_happy_path_inserts_row_and_copies_citation() {
        let (_tmp, service, tablet_id, item_ids) = build_weekly_with_priority_candidates(1);
        let dto = service.confirm_priority(&item_ids[0]).unwrap();
        assert_eq!(dto.tablet_id, tablet_id);
        assert_eq!(dto.source, "confirmed");
        assert!(dto.confirmed_at.is_some());
        assert_eq!(dto.citation_id.as_deref(), Some(item_ids[0].as_str()));
        // Confirm row in DB.
        let (conn, _, _) = service_internals(&service);
        let c = conn.0.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM ceremony_priorities cp JOIN todos t ON t.id = cp.todo_id WHERE json_extract(t.attrs, '$.citation_id') = ?1",
                rusqlite::params![&item_ids[0]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn confirm_priority_preserves_rationale_from_body() {
        // I-0049 T-0315 — body: {text, rationale} flows into
        // todos.rationale rather than being discarded.
        let (tmp, conn) = open_test_db();
        let tablet_id = "weekly-2026-W20".to_string();
        let item_id = "item-with-rationale".to_string();
        {
            let c = conn.0.lock().unwrap();
            c.execute(
                "INSERT INTO ceremony_tablets \
                 (id, kind, period_key, generated_at, status, workstreams_scanned) \
                 VALUES (?1, 'weekly', '2026-W20', '2026-05-11T07:00:00Z', 'open', '[]')",
                rusqlite::params![&tablet_id],
            )
            .unwrap();
            c.execute(
                "INSERT INTO ceremony_items \
                 (id, tablet_id, section_key, ordinal, kind, body, citation_id, created_at) \
                 VALUES (?1, ?2, 'priorities', 0, 'priority', ?3, 'sig-0', '2026-05-11T07:00:00Z')",
                rusqlite::params![
                    &item_id,
                    &tablet_id,
                    json!({"text":"ship auth migration","rationale":"unblocks compliance audit"})
                        .to_string(),
                ],
            )
            .unwrap();
        }
        let dispatcher = Arc::new(EngineDispatcher::new(conn.clone(), PluginRegistry::new()));
        let service = CeremonyService::new(conn.clone(), dispatcher);
        let dto = service.confirm_priority(&item_id).unwrap();
        assert_eq!(dto.rationale, "unblocks compliance audit");
        // Round-trip through storage too.
        let c = conn.0.lock().unwrap();
        let stored: Option<String> = c
            .query_row(
                "SELECT t.rationale FROM ceremony_priorities cp \
                 JOIN todos t ON t.id = cp.todo_id \
                 WHERE cp.id = ?1",
                rusqlite::params![&dto.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored.as_deref(), Some("unblocks compliance audit"));
        drop(tmp);
    }

    #[tokio::test]
    async fn confirm_priority_leaves_rationale_null_when_body_has_none() {
        // body has only `text` — confirm should leave todos.rationale
        // NULL rather than synthesising one.
        let (_tmp, service, _tablet_id, item_ids) = build_weekly_with_priority_candidates(1);
        let dto = service.confirm_priority(&item_ids[0]).unwrap();
        assert!(dto.rationale.is_empty());
        let (conn, _, _) = service_internals(&service);
        let c = conn.0.lock().unwrap();
        let stored: Option<String> = c
            .query_row(
                "SELECT t.rationale FROM ceremony_priorities cp \
                 JOIN todos t ON t.id = cp.todo_id \
                 WHERE cp.id = ?1",
                rusqlite::params![&dto.id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(stored.is_none(), "rationale should be NULL, got {stored:?}");
    }

    #[tokio::test]
    async fn confirm_priority_is_idempotent() {
        let (_tmp, service, _tablet_id, item_ids) = build_weekly_with_priority_candidates(1);
        let a = service.confirm_priority(&item_ids[0]).unwrap();
        let b = service.confirm_priority(&item_ids[0]).unwrap();
        assert_eq!(a.id, b.id);
        let (conn, _, _) = service_internals(&service);
        let c = conn.0.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM ceremony_priorities cp JOIN todos t ON t.id = cp.todo_id WHERE json_extract(t.attrs, '$.citation_id') = ?1",
                rusqlite::params![&item_ids[0]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn confirm_priority_rejects_non_priority_item() {
        let (_tmp, conn) = open_test_db();
        // Hand-roll a weekly tablet with a non-priority item.
        {
            let c = conn.0.lock().unwrap();
            c.execute(
                "INSERT INTO ceremony_tablets \
                 (id, kind, period_key, generated_at, status, workstreams_scanned) \
                 VALUES ('weekly-1', 'weekly', '2026-W20', '2026-05-11T07:00:00Z', 'open', '[]')",
                [],
            )
            .unwrap();
            c.execute(
                "INSERT INTO ceremony_items \
                 (id, tablet_id, section_key, ordinal, kind, body, citation_id, created_at) \
                 VALUES ('item-x', 'weekly-1', 'attention', 0, 'attention', '{}', 'src-1', '2026-05-11T07:00:00Z')",
                [],
            )
            .unwrap();
        }
        let dispatcher = Arc::new(EngineDispatcher::new(conn.clone(), PluginRegistry::new()));
        let service = CeremonyService::new(conn, dispatcher);
        let err = service.confirm_priority("item-x").unwrap_err();
        assert!(matches!(err, CeremonyError::InvalidTabletState(_)));
        // Also unknown item.
        let err = service.confirm_priority("nope").unwrap_err();
        assert!(matches!(err, CeremonyError::InvalidTabletState(_)));
    }

    #[tokio::test]
    async fn reject_priority_deletes_item_and_priority_row() {
        let (_tmp, service, _tablet_id, item_ids) = build_weekly_with_priority_candidates(1);
        service.confirm_priority(&item_ids[0]).unwrap();
        service.reject_priority(&item_ids[0]).unwrap();
        let (conn, _, _) = service_internals(&service);
        let c = conn.0.lock().unwrap();
        let item_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM ceremony_items WHERE id = ?1",
                rusqlite::params![&item_ids[0]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(item_count, 0);
        let prio_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM ceremony_priorities cp JOIN todos t ON t.id = cp.todo_id WHERE json_extract(t.attrs, '$.citation_id') = ?1",
                rusqlite::params![&item_ids[0]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(prio_count, 0);
    }

    #[tokio::test]
    async fn add_priority_inserts_with_null_citation() {
        let (_tmp, service, tablet_id, _) = build_weekly_with_priority_candidates(0);
        let dto = service
            .add_priority(AddPriorityRequest {
                tablet_id: tablet_id.clone(),
                body: json!({"text": "ship the thing"}),
                rationale: "because launches".into(),
            })
            .unwrap();
        assert!(dto.citation_id.is_none());
        assert!(dto.confirmed_at.is_some());
        assert_eq!(dto.rationale, "because launches");
        assert_eq!(dto.source, "confirmed");
        assert_eq!(dto.ordinal, 0);
        // A second add bumps ordinal.
        let dto2 = service
            .add_priority(AddPriorityRequest {
                tablet_id: tablet_id.clone(),
                body: json!({"text": "second"}),
                rationale: "".into(),
            })
            .unwrap();
        assert_eq!(dto2.ordinal, 1);
    }

    #[tokio::test]
    async fn list_priorities_unions_confirmed_and_candidates_deduped() {
        let (_tmp, service, tablet_id, item_ids) = build_weekly_with_priority_candidates(3);
        // Confirm the first one. Remaining two stay as candidates.
        service.confirm_priority(&item_ids[0]).unwrap();
        // Also add a no-citation user priority.
        service
            .add_priority(AddPriorityRequest {
                tablet_id: tablet_id.clone(),
                body: json!({"text": "user-added"}),
                rationale: "r".into(),
            })
            .unwrap();
        let list = service.list_priorities(&tablet_id).unwrap();
        // 2 confirmed + 2 candidates.
        let confirmed: Vec<_> = list.iter().filter(|p| p.source == "confirmed").collect();
        let candidates: Vec<_> = list.iter().filter(|p| p.source == "candidate").collect();
        assert_eq!(confirmed.len(), 2);
        assert_eq!(candidates.len(), 2);
        // Confirmed come first, sorted by ordinal.
        assert_eq!(list[0].source, "confirmed");
        assert_eq!(list[1].source, "confirmed");
        assert_eq!(list[2].source, "candidate");
        assert_eq!(list[3].source, "candidate");
        // Candidates are the two not-yet-confirmed item ids.
        let cand_ids: std::collections::HashSet<_> =
            candidates.iter().map(|p| p.id.clone()).collect();
        assert!(cand_ids.contains(&item_ids[1]));
        assert!(cand_ids.contains(&item_ids[2]));
        assert!(!cand_ids.contains(&item_ids[0]));
    }

    #[tokio::test]
    async fn confirm_priority_emits_priority_confirmed_event() {
        use crate::CeremonyEvent;
        let (tx, mut rx) = crate::event_channel();
        let (_tmp, service, tablet_id, item_ids) = build_weekly_with_priority_candidates(1);
        let service = service.with_events(tx);
        let dto = service.confirm_priority(&item_ids[0]).unwrap();
        let event = rx.recv().await.unwrap();
        match event {
            CeremonyEvent::PriorityConfirmed {
                priority_id,
                tablet_id: tid,
            } => {
                assert_eq!(priority_id, dto.id);
                assert_eq!(tid, tablet_id);
            }
            other => panic!("expected PriorityConfirmed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn patch_item_emits_item_updated_event() {
        use crate::CeremonyEvent;
        let (tx, mut rx) = crate::event_channel();
        let (_tmp, service, tablet_id) = build_service_with_items("retro", "2026-W20", "retro");
        // Replace service with one that has events wired.
        let service = service.with_events(tx);
        service.run("retro").await.unwrap();
        let items = service
            .list_items(&tablet_id, Some("what_happened"))
            .unwrap();
        let id = &items[0].id;
        service
            .patch_item(
                id,
                ItemPatch {
                    done: Some(true),
                    body: None,
                },
            )
            .unwrap();
        // Drain until we see ItemUpdated for this id; service.run
        // does not currently fire an event (dispatcher has no
        // sender in this helper), so the first event off the wire
        // should be the patch.
        let event = rx.recv().await.unwrap();
        assert!(matches!(
            event,
            CeremonyEvent::ItemUpdated { ref item_id, .. } if item_id == id
        ));
    }
}
