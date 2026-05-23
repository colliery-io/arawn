//! Generic todos store (I-0049 T-0309).
//!
//! Canonical source of truth for `done_at` across user todos, weekly
//! priorities, and rollover todos. The `kind` field is a free-form
//! text discriminant — `user`, `weekly_priority`, `rollover` today;
//! future kinds drop in without schema changes.
//!
//! `attrs` is kind-specific JSON. Callers serialize whatever they
//! need (tablet_id, confirmed_at, citation_id, ordinal, ...) and
//! deserialize on read. The store treats it as opaque.
//!
//! This is scaffold-only — no callers in v1. T-0310 adds the event
//! broadcast + WS-RPC surface, T-0311 backfills from ceremony state,
//! T-0312 swaps ceremony reads onto this store.

use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, params_from_iter, types::Value};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::database::Database;
use crate::error::StorageError;

/// Broadcast channel capacity for `TodoEvent`. Same order as
/// CeremonyEvent — burst of `Updated` on a list-confirm shouldn't
/// drop on a single slow subscriber.
pub const TODO_EVENT_CAPACITY: usize = 64;

/// All todo state-change events. Payloads stay tight (id/kind/done_at)
/// so the channel scales; subscribers can re-fetch the full row via
/// `TodoService::get` if they need more.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", content = "data")]
pub enum TodoEvent {
    Created {
        id: String,
        kind: String,
    },
    Completed {
        id: String,
        kind: String,
        done_at: DateTime<Utc>,
    },
    /// Emitted on `patch`, `undo`, and other in-place mutations.
    Updated {
        id: String,
        kind: String,
    },
    Archived {
        id: String,
        kind: String,
    },
}

pub type TodoEventSender = broadcast::Sender<TodoEvent>;
pub type TodoEventReceiver = broadcast::Receiver<TodoEvent>;

/// Build a fresh todo-event channel with the default capacity.
pub fn todo_event_channel() -> (TodoEventSender, TodoEventReceiver) {
    broadcast::channel(TODO_EVENT_CAPACITY)
}

fn emit(sender: &Option<TodoEventSender>, event: TodoEvent) {
    if let Some(tx) = sender {
        let _ = tx.send(event);
    }
}

/// One todo row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Todo {
    pub id: String,
    pub body: String,
    pub rationale: Option<String>,
    pub kind: String,
    pub workstream: Option<String>,
    pub created_at: DateTime<Utc>,
    pub due_at: Option<DateTime<Utc>>,
    pub done_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    pub attrs: serde_json::Value,
}

/// Input for `create`. `id` is generated when omitted so backfill
/// migrations can supply deterministic IDs while normal callers
/// don't have to think about it.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NewTodo {
    pub id: Option<String>,
    pub body: String,
    pub rationale: Option<String>,
    pub kind: String,
    pub workstream: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub attrs: Option<serde_json::Value>,
}

/// Patchable fields. Anything `None` is left unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoPatch {
    pub body: Option<String>,
    pub rationale: Option<String>,
    pub workstream: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub attrs: Option<serde_json::Value>,
}

/// Filter for `list`. All fields are optional and combine as AND.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListFilter {
    pub kind: Option<String>,
    pub workstream: Option<String>,
    /// When `Some(true)`: only `done_at IS NULL`. When `Some(false)`:
    /// only `done_at IS NOT NULL`. When `None`: no filter.
    pub open_only: Option<bool>,
    /// Inclusive lower bound on `due_at`.
    pub due_from: Option<DateTime<Utc>>,
    /// Inclusive upper bound on `due_at`.
    pub due_to: Option<DateTime<Utc>>,
    /// Exclude rows where `archived_at IS NOT NULL`. Defaults to true
    /// at the call site — most consumers don't want archived rows.
    pub include_archived: Option<bool>,
}

pub struct TodoService<'a> {
    db: &'a Database,
    events: Option<TodoEventSender>,
}

impl<'a> TodoService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db, events: None }
    }

    /// Attach a broadcast sender. Every mutating method emits a
    /// `TodoEvent` to subscribers; methods are no-op for events when
    /// no sender is wired (tests, read-only contexts).
    pub fn with_events(mut self, events: TodoEventSender) -> Self {
        self.events = Some(events);
        self
    }

    pub fn create(&self, req: NewTodo) -> Result<Todo, StorageError> {
        if req.body.trim().is_empty() {
            return Err(StorageError::InvalidOperation(
                "todo body must not be empty".into(),
            ));
        }
        if req.kind.trim().is_empty() {
            return Err(StorageError::InvalidOperation(
                "todo kind must not be empty".into(),
            ));
        }
        let id = req.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = Utc::now();
        let attrs = req.attrs.unwrap_or_else(|| serde_json::json!({}));
        let attrs_str = serde_json::to_string(&attrs)?;
        self.db.conn().execute(
            "INSERT INTO todos \
                 (id, body, rationale, kind, workstream, created_at, due_at, done_at, archived_at, attrs) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL, ?8)",
            (
                &id,
                &req.body,
                &req.rationale,
                &req.kind,
                &req.workstream,
                now.to_rfc3339(),
                req.due_at.map(|d| d.to_rfc3339()),
                attrs_str,
            ),
        )?;
        let row = self.get(&id)?.ok_or_else(|| {
            StorageError::InvalidOperation(format!("todo {id} vanished after insert"))
        })?;
        emit(
            &self.events,
            TodoEvent::Created {
                id: row.id.clone(),
                kind: row.kind.clone(),
            },
        );
        Ok(row)
    }

    pub fn get(&self, id: &str) -> Result<Option<Todo>, StorageError> {
        let row = self
            .db
            .conn()
            .query_row(
                "SELECT id, body, rationale, kind, workstream, created_at, due_at, done_at, archived_at, attrs \
                 FROM todos WHERE id = ?1",
                [id],
                row_to_todo,
            )
            .optional()?;
        Ok(row)
    }

    /// Idempotent — re-marking a done row preserves the original
    /// timestamp.
    pub fn mark_done(&self, id: &str) -> Result<Todo, StorageError> {
        let existing = self
            .get(id)?
            .ok_or_else(|| StorageError::NotFound(format!("todo {id}")))?;
        if existing.done_at.is_some() {
            return Ok(existing);
        }
        let now = Utc::now();
        self.db.conn().execute(
            "UPDATE todos SET done_at = ?1 WHERE id = ?2 AND done_at IS NULL",
            (now.to_rfc3339(), id),
        )?;
        let row = self
            .get(id)?
            .ok_or_else(|| StorageError::NotFound(format!("todo {id}")))?;
        if let Some(done_at) = row.done_at {
            emit(
                &self.events,
                TodoEvent::Completed {
                    id: row.id.clone(),
                    kind: row.kind.clone(),
                    done_at,
                },
            );
        }
        Ok(row)
    }

    /// Idempotent — undoing a not-done row is a no-op.
    pub fn undo(&self, id: &str) -> Result<Todo, StorageError> {
        let existing = self
            .get(id)?
            .ok_or_else(|| StorageError::NotFound(format!("todo {id}")))?;
        if existing.done_at.is_none() {
            return Ok(existing);
        }
        self.db
            .conn()
            .execute("UPDATE todos SET done_at = NULL WHERE id = ?1", [id])?;
        let row = self
            .get(id)?
            .ok_or_else(|| StorageError::NotFound(format!("todo {id}")))?;
        emit(
            &self.events,
            TodoEvent::Updated {
                id: row.id.clone(),
                kind: row.kind.clone(),
            },
        );
        Ok(row)
    }

    pub fn patch(&self, id: &str, patch: TodoPatch) -> Result<Todo, StorageError> {
        // Verify existence so callers get a NotFound instead of a
        // silent no-op when patching a missing id.
        self.get(id)?
            .ok_or_else(|| StorageError::NotFound(format!("todo {id}")))?;

        let mut sets: Vec<&'static str> = Vec::new();
        let mut params: Vec<Value> = Vec::new();
        if let Some(body) = patch.body {
            if body.trim().is_empty() {
                return Err(StorageError::InvalidOperation(
                    "todo body must not be empty".into(),
                ));
            }
            sets.push("body = ?");
            params.push(Value::Text(body));
        }
        if let Some(rationale) = patch.rationale {
            sets.push("rationale = ?");
            params.push(Value::Text(rationale));
        }
        if let Some(workstream) = patch.workstream {
            sets.push("workstream = ?");
            params.push(Value::Text(workstream));
        }
        if let Some(due_at) = patch.due_at {
            sets.push("due_at = ?");
            params.push(Value::Text(due_at.to_rfc3339()));
        }
        if let Some(attrs) = patch.attrs {
            sets.push("attrs = ?");
            params.push(Value::Text(serde_json::to_string(&attrs)?));
        }
        if sets.is_empty() {
            // Nothing to patch — just return the row as-is.
            return self
                .get(id)?
                .ok_or_else(|| StorageError::NotFound(format!("todo {id}")));
        }
        let sql = format!("UPDATE todos SET {} WHERE id = ?", sets.join(", "));
        params.push(Value::Text(id.to_string()));
        self.db.conn().execute(&sql, params_from_iter(params))?;
        let row = self
            .get(id)?
            .ok_or_else(|| StorageError::NotFound(format!("todo {id}")))?;
        emit(
            &self.events,
            TodoEvent::Updated {
                id: row.id.clone(),
                kind: row.kind.clone(),
            },
        );
        Ok(row)
    }

    /// Soft-delete. The row stays in the table so backrefs from
    /// ceremony tables don't dangle; subsequent `list`/`search` skip
    /// archived rows by default.
    pub fn archive(&self, id: &str) -> Result<(), StorageError> {
        let existing = self
            .get(id)?
            .ok_or_else(|| StorageError::NotFound(format!("todo {id}")))?;
        if existing.archived_at.is_some() {
            return Ok(());
        }
        let now = Utc::now();
        self.db.conn().execute(
            "UPDATE todos SET archived_at = ?1 WHERE id = ?2",
            (now.to_rfc3339(), id),
        )?;
        emit(
            &self.events,
            TodoEvent::Archived {
                id: existing.id.clone(),
                kind: existing.kind.clone(),
            },
        );
        Ok(())
    }

    pub fn list(&self, filter: ListFilter) -> Result<Vec<Todo>, StorageError> {
        let mut clauses: Vec<&'static str> = Vec::new();
        let mut params: Vec<Value> = Vec::new();
        if let Some(kind) = filter.kind {
            clauses.push("kind = ?");
            params.push(Value::Text(kind));
        }
        if let Some(workstream) = filter.workstream {
            clauses.push("workstream = ?");
            params.push(Value::Text(workstream));
        }
        match filter.open_only {
            Some(true) => clauses.push("done_at IS NULL"),
            Some(false) => clauses.push("done_at IS NOT NULL"),
            None => {}
        }
        if let Some(due_from) = filter.due_from {
            clauses.push("due_at >= ?");
            params.push(Value::Text(due_from.to_rfc3339()));
        }
        if let Some(due_to) = filter.due_to {
            clauses.push("due_at <= ?");
            params.push(Value::Text(due_to.to_rfc3339()));
        }
        if !filter.include_archived.unwrap_or(false) {
            clauses.push("archived_at IS NULL");
        }
        let where_sql = if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        };
        let sql = format!(
            "SELECT id, body, rationale, kind, workstream, created_at, due_at, done_at, archived_at, attrs \
             FROM todos {where_sql} \
             ORDER BY \
                 CASE WHEN due_at IS NULL THEN 1 ELSE 0 END, \
                 due_at ASC, \
                 created_at DESC"
        );
        let mut stmt = self.db.conn().prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params), row_to_todo)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
            .and_then(|rows| rows.into_iter().map(Ok).collect())
    }

    /// Plain `body LIKE` for v1. FTS5 deferred until usage shows a
    /// concrete need.
    pub fn search(&self, query: &str) -> Result<Vec<Todo>, StorageError> {
        let pattern = format!("%{}%", query.replace('%', r"\%").replace('_', r"\_"));
        let mut stmt = self.db.conn().prepare(
            "SELECT id, body, rationale, kind, workstream, created_at, due_at, done_at, archived_at, attrs \
             FROM todos \
             WHERE archived_at IS NULL AND body LIKE ?1 ESCAPE '\\' \
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([pattern], row_to_todo)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }
}

fn row_to_todo(row: &rusqlite::Row<'_>) -> rusqlite::Result<Todo> {
    let attrs_str: String = row.get(9)?;
    let attrs = serde_json::from_str(&attrs_str).unwrap_or_else(|_| serde_json::json!({}));
    Ok(Todo {
        id: row.get(0)?,
        body: row.get(1)?,
        rationale: row.get(2)?,
        kind: row.get(3)?,
        workstream: row.get(4)?,
        created_at: parse_dt_row(row, 5)?,
        due_at: parse_dt_row_opt(row, 6)?,
        done_at: parse_dt_row_opt(row, 7)?,
        archived_at: parse_dt_row_opt(row, 8)?,
        attrs,
    })
}

fn parse_dt_row(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<DateTime<Utc>> {
    let s: String = row.get(idx)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, Box::new(e))
        })
}

fn parse_dt_row_opt(
    row: &rusqlite::Row<'_>,
    idx: usize,
) -> rusqlite::Result<Option<DateTime<Utc>>> {
    let s: Option<String> = row.get(idx)?;
    match s {
        None => Ok(None),
        Some(s) => DateTime::parse_from_rfc3339(&s)
            .map(|d| Some(d.with_timezone(&Utc)))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    idx,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Database {
        Database::in_memory().unwrap()
    }

    fn new_user(body: &str) -> NewTodo {
        NewTodo {
            body: body.into(),
            kind: "user".into(),
            ..Default::default()
        }
    }

    #[test]
    fn migration_creates_todos_table_and_indexes() {
        let db = db();
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='todos'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        let idx_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name='todos' AND name IN ('todos_kind_idx','todos_done_idx','todos_workstream_idx')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(idx_count, 3);
    }

    #[test]
    fn create_round_trips_and_get_returns_some() {
        let db = db();
        let svc = TodoService::new(&db);
        let todo = svc.create(new_user("write the test")).unwrap();
        assert_eq!(todo.body, "write the test");
        assert_eq!(todo.kind, "user");
        assert!(todo.done_at.is_none());
        let fetched = svc.get(&todo.id).unwrap().unwrap();
        assert_eq!(fetched, todo);
    }

    #[test]
    fn create_rejects_empty_body_and_empty_kind() {
        let db = db();
        let svc = TodoService::new(&db);
        assert!(svc.create(new_user("")).is_err());
        let bad_kind = NewTodo {
            body: "x".into(),
            kind: "".into(),
            ..Default::default()
        };
        assert!(svc.create(bad_kind).is_err());
    }

    #[test]
    fn create_with_explicit_id_preserves_it() {
        let db = db();
        let svc = TodoService::new(&db);
        let req = NewTodo {
            id: Some("backfill-1".into()),
            body: "from migration".into(),
            kind: "weekly_priority".into(),
            attrs: Some(serde_json::json!({"tablet_id":"weekly-2026-W20","ordinal":0})),
            ..Default::default()
        };
        let todo = svc.create(req).unwrap();
        assert_eq!(todo.id, "backfill-1");
        assert_eq!(todo.attrs["tablet_id"], "weekly-2026-W20");
    }

    #[test]
    fn mark_done_is_idempotent() {
        let db = db();
        let svc = TodoService::new(&db);
        let t = svc.create(new_user("ship it")).unwrap();
        let first = svc.mark_done(&t.id).unwrap();
        assert!(first.done_at.is_some());
        let second = svc.mark_done(&t.id).unwrap();
        assert_eq!(first.done_at, second.done_at, "repeat done preserves ts");
    }

    #[test]
    fn undo_clears_done_and_is_idempotent_when_already_open() {
        let db = db();
        let svc = TodoService::new(&db);
        let t = svc.create(new_user("oops")).unwrap();
        let undone1 = svc.undo(&t.id).unwrap();
        assert!(undone1.done_at.is_none());
        let _ = svc.mark_done(&t.id).unwrap();
        let undone2 = svc.undo(&t.id).unwrap();
        assert!(undone2.done_at.is_none());
    }

    #[test]
    fn patch_updates_only_provided_fields() {
        let db = db();
        let svc = TodoService::new(&db);
        let t = svc.create(new_user("draft")).unwrap();
        let patched = svc
            .patch(
                &t.id,
                TodoPatch {
                    body: Some("revised".into()),
                    workstream: Some("auth-migration".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(patched.body, "revised");
        assert_eq!(patched.workstream.as_deref(), Some("auth-migration"));
        assert!(patched.rationale.is_none());
    }

    #[test]
    fn patch_unknown_id_returns_not_found() {
        let db = db();
        let svc = TodoService::new(&db);
        let err = svc.patch("missing", TodoPatch::default()).unwrap_err();
        assert!(matches!(err, StorageError::NotFound(_)));
    }

    #[test]
    fn archive_soft_deletes_and_hides_from_list() {
        let db = db();
        let svc = TodoService::new(&db);
        let a = svc.create(new_user("keep")).unwrap();
        let b = svc.create(new_user("drop")).unwrap();
        svc.archive(&b.id).unwrap();
        let listed = svc.list(ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, a.id);
        // Including archived still shows it.
        let listed_all = svc
            .list(ListFilter {
                include_archived: Some(true),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(listed_all.len(), 2);
    }

    #[test]
    fn list_filters_by_kind_workstream_and_open_only() {
        let db = db();
        let svc = TodoService::new(&db);
        let user = svc.create(new_user("alpha")).unwrap();
        let _wp = svc
            .create(NewTodo {
                body: "priority".into(),
                kind: "weekly_priority".into(),
                workstream: Some("ws-a".into()),
                ..Default::default()
            })
            .unwrap();
        let _ = svc.mark_done(&user.id).unwrap();

        let only_user = svc
            .list(ListFilter {
                kind: Some("user".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(only_user.len(), 1);
        assert_eq!(only_user[0].id, user.id);

        let only_ws_a = svc
            .list(ListFilter {
                workstream: Some("ws-a".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(only_ws_a.len(), 1);

        let open = svc
            .list(ListFilter {
                open_only: Some(true),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].kind, "weekly_priority");
    }

    #[test]
    fn list_due_window_filters_inclusive() {
        let db = db();
        let svc = TodoService::new(&db);
        let t1: DateTime<Utc> = "2026-05-10T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let t2: DateTime<Utc> = "2026-05-15T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let t3: DateTime<Utc> = "2026-05-20T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        for due in [t1, t2, t3] {
            svc.create(NewTodo {
                body: format!("due {due}"),
                kind: "user".into(),
                due_at: Some(due),
                ..Default::default()
            })
            .unwrap();
        }
        let in_window = svc
            .list(ListFilter {
                due_from: Some(t1),
                due_to: Some(t2),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(in_window.len(), 2);
    }

    #[test]
    fn search_matches_body_substring_and_skips_archived() {
        let db = db();
        let svc = TodoService::new(&db);
        let a = svc.create(new_user("ship the migration")).unwrap();
        let _b = svc.create(new_user("write the README")).unwrap();
        let c = svc.create(new_user("review migration plan")).unwrap();
        svc.archive(&c.id).unwrap();
        let hits = svc.search("migration").unwrap();
        assert_eq!(hits.len(), 1, "only un-archived match returned");
        assert_eq!(hits[0].id, a.id);
    }

    #[test]
    fn v8_backfills_ceremony_priorities_and_rolling_todos() {
        // Build V7 with ceremony state seeded, then apply V8 backfill.
        let mut db = Database::in_memory_at_version(7).unwrap();
        {
            let conn = db.conn();
            // Seed two tablets — a weekly with two priorities, a daily
            // with two rollover todos.
            conn.execute(
                "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                ["weekly-2026-W20", "weekly", "2026-W20", "2026-05-11T08:00:00Z", "reviewed", "[]"],
            ).unwrap();
            conn.execute(
                "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                ["daily-2026-05-15", "daily", "2026-05-15", "2026-05-15T07:00:00Z", "open", "[]"],
            ).unwrap();
            // Two confirmed priorities — one done, one open.
            for (idx, (id, body, done)) in [
                ("p-a", "ship I-0049", Some("2026-05-14T12:00:00Z")),
                ("p-b", "ship I-0050", None),
            ]
            .iter()
            .enumerate()
            {
                conn.execute(
                    "INSERT INTO ceremony_priorities (id, tablet_id, body, rationale, citation_id, confirmed_at, done_at, ordinal) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        id,
                        "weekly-2026-W20",
                        body,
                        "",
                        "cite-x",
                        "2026-05-11T09:00:00Z",
                        *done,
                        idx as i64,
                    ],
                )
                .unwrap();
            }
            // Two rollover todos — one done, one open.
            for (id, body, done) in [
                ("t-a", "review pr", Some("2026-05-15T11:00:00Z")),
                ("t-b", "write doc", None),
            ] {
                conn.execute(
                    "INSERT INTO ceremony_todos_rolling (todo_id, body, origin_tablet_id, created_at, done_at, last_seen_tablet_id) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        id,
                        body,
                        "daily-2026-05-15",
                        "2026-05-14T07:00:00Z",
                        done,
                        "daily-2026-05-15",
                    ],
                )
                .unwrap();
            }
        }
        // Run V8.
        db.run_pending_migrations().unwrap();

        let svc = TodoService::new(&db);
        let all = svc
            .list(ListFilter {
                include_archived: Some(true),
                ..Default::default()
            })
            .unwrap();
        // 2 priorities + 2 rollover = 4.
        assert_eq!(all.len(), 4);

        // Weekly priority round-trip.
        let p_a = svc.get("wp:p-a").unwrap().expect("wp:p-a backfilled");
        assert_eq!(p_a.kind, "weekly_priority");
        assert_eq!(p_a.body, "ship I-0049");
        assert!(p_a.done_at.is_some());
        assert!(p_a.rationale.is_none(), "empty rationale becomes NULL");
        assert_eq!(p_a.attrs["tablet_id"], "weekly-2026-W20");
        assert_eq!(p_a.attrs["ordinal"], 0);
        assert_eq!(p_a.attrs["confirmed_at"], "2026-05-11T09:00:00Z");
        assert_eq!(p_a.attrs["citation_id"], "cite-x");
        // created_at copied from the parent tablet's generated_at.
        assert_eq!(p_a.created_at.to_rfc3339(), "2026-05-11T08:00:00+00:00");

        let p_b = svc.get("wp:p-b").unwrap().expect("wp:p-b backfilled");
        assert!(p_b.done_at.is_none());

        // Rollover round-trip.
        let t_a = svc.get("rl:t-a").unwrap().expect("rl:t-a backfilled");
        assert_eq!(t_a.kind, "rollover");
        assert_eq!(t_a.body, "review pr");
        assert!(t_a.done_at.is_some());
        assert_eq!(t_a.attrs["origin_tablet_id"], "daily-2026-05-15");
        assert_eq!(t_a.attrs["last_seen_tablet_id"], "daily-2026-05-15");

        let t_b = svc.get("rl:t-b").unwrap().expect("rl:t-b backfilled");
        assert!(t_b.done_at.is_none());
    }

    #[test]
    fn v8_backfill_is_a_no_op_on_empty_ceremony_state() {
        // Fresh DB with no ceremony rows — V8 should run cleanly and
        // produce zero todos.
        let db = db();
        let svc = TodoService::new(&db);
        let all = svc
            .list(ListFilter {
                include_archived: Some(true),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(all.len(), 0);
    }

    #[test]
    fn events_emitted_on_create_done_undo_patch_archive() {
        let db = db();
        let (tx, mut rx) = todo_event_channel();
        let svc = TodoService::new(&db).with_events(tx);

        let t = svc.create(new_user("alpha")).unwrap();
        let _ = svc.mark_done(&t.id).unwrap();
        let _ = svc.undo(&t.id).unwrap();
        let _ = svc
            .patch(
                &t.id,
                TodoPatch {
                    body: Some("alpha v2".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        svc.archive(&t.id).unwrap();

        // Drain the channel and inspect the variants in order.
        let mut variants: Vec<&'static str> = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            variants.push(match ev {
                TodoEvent::Created { .. } => "Created",
                TodoEvent::Completed { .. } => "Completed",
                TodoEvent::Updated { .. } => "Updated",
                TodoEvent::Archived { .. } => "Archived",
            });
        }
        assert_eq!(
            variants,
            vec!["Created", "Completed", "Updated", "Updated", "Archived"]
        );
    }

    #[test]
    fn events_skipped_on_idempotent_paths() {
        let db = db();
        let (tx, mut rx) = todo_event_channel();
        let svc = TodoService::new(&db).with_events(tx);
        let t = svc.create(new_user("idempotent")).unwrap();
        // Drain the Created event.
        let _ = rx.try_recv().unwrap();
        // Second mark_done is a no-op — no event.
        svc.mark_done(&t.id).unwrap();
        let _ = rx.try_recv().unwrap(); // first real Completed
        svc.mark_done(&t.id).unwrap(); // idempotent
        assert!(rx.try_recv().is_err(), "no event on idempotent mark_done");
        // Undoing twice — the second is a no-op.
        svc.undo(&t.id).unwrap();
        let _ = rx.try_recv().unwrap();
        svc.undo(&t.id).unwrap();
        assert!(rx.try_recv().is_err(), "no event on idempotent undo");
    }

    #[test]
    fn search_escapes_like_metacharacters() {
        let db = db();
        let svc = TodoService::new(&db);
        let a = svc.create(new_user("100% done feel")).unwrap();
        let _b = svc.create(new_user("nothing")).unwrap();
        let hits = svc.search("100%").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, a.id);
    }
}
