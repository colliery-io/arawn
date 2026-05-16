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
use uuid::Uuid;

use crate::database::Database;
use crate::error::StorageError;

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
}

impl<'a> TodoService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
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
        self.get(&id)?.ok_or_else(|| {
            StorageError::InvalidOperation(format!("todo {id} vanished after insert"))
        })
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
        self.get(id)?
            .ok_or_else(|| StorageError::NotFound(format!("todo {id}")))
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
        self.get(id)?
            .ok_or_else(|| StorageError::NotFound(format!("todo {id}")))
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
        self.get(id)?
            .ok_or_else(|| StorageError::NotFound(format!("todo {id}")))
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
