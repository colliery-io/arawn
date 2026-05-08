//! SQLite-backed CRUD for feed records.
//!
//! The `feeds` table is the source of truth for *what we're configured
//! to fetch*. cloacina cron schedules are derived from this table at
//! server boot + on `/watch`. `meta.json` per feed dir on disk is the
//! source of truth for *what we've fetched* (cursor + last_run).

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::error::FeedError;
use crate::types::TemplateParams;

/// One row from the `feeds` table.
#[derive(Debug, Clone)]
pub struct FeedRecord {
    pub id: String,
    pub template: String,
    pub params: TemplateParams,
    pub cadence: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// CRUD over the `feeds` table. Borrows a `&Connection` so it can be
/// used inside arawn-storage's `Database` or any other rusqlite handle.
pub struct FeedStore<'a> {
    conn: &'a Connection,
}

impl<'a> FeedStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn insert(&self, rec: &FeedRecord) -> Result<(), FeedError> {
        let params_json = serde_json::to_string(&rec.params)
            .map_err(|e| FeedError::Storage(format!("serialize params: {e}")))?;
        self.conn
            .execute(
                "INSERT INTO feeds (id, template, params, cadence, enabled, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    rec.id,
                    rec.template,
                    params_json,
                    rec.cadence,
                    rec.enabled as i32,
                    rec.created_at.to_rfc3339(),
                    rec.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| FeedError::Storage(format!("insert feed: {e}")))?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<FeedRecord>, FeedError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, template, params, cadence, enabled, created_at, updated_at
                 FROM feeds WHERE id = ?1",
            )
            .map_err(|e| FeedError::Storage(format!("prepare get: {e}")))?;
        stmt.query_row(params![id], row_to_record)
            .optional()
            .map_err(|e| FeedError::Storage(format!("get feed: {e}")))?
            .transpose()
    }

    pub fn list_enabled(&self) -> Result<Vec<FeedRecord>, FeedError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, template, params, cadence, enabled, created_at, updated_at
                 FROM feeds WHERE enabled = 1 ORDER BY id",
            )
            .map_err(|e| FeedError::Storage(format!("prepare list: {e}")))?;
        let rows = stmt
            .query_map([], row_to_record)
            .map_err(|e| FeedError::Storage(format!("list feeds: {e}")))?;
        rows.collect::<Result<Result<Vec<_>, _>, _>>()
            .map_err(|e| FeedError::Storage(format!("collect rows: {e}")))?
    }

    pub fn list_all(&self) -> Result<Vec<FeedRecord>, FeedError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, template, params, cadence, enabled, created_at, updated_at
                 FROM feeds ORDER BY id",
            )
            .map_err(|e| FeedError::Storage(format!("prepare list: {e}")))?;
        let rows = stmt
            .query_map([], row_to_record)
            .map_err(|e| FeedError::Storage(format!("list feeds: {e}")))?;
        rows.collect::<Result<Result<Vec<_>, _>, _>>()
            .map_err(|e| FeedError::Storage(format!("collect rows: {e}")))?
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), FeedError> {
        let now = Utc::now().to_rfc3339();
        let n = self
            .conn
            .execute(
                "UPDATE feeds SET enabled = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, enabled as i32, now],
            )
            .map_err(|e| FeedError::Storage(format!("update enabled: {e}")))?;
        if n == 0 {
            return Err(FeedError::Storage(format!("no feed with id '{id}'")));
        }
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), FeedError> {
        self.conn
            .execute("DELETE FROM feeds WHERE id = ?1", params![id])
            .map_err(|e| FeedError::Storage(format!("delete feed: {e}")))?;
        Ok(())
    }
}

fn row_to_record(row: &rusqlite::Row) -> rusqlite::Result<Result<FeedRecord, FeedError>> {
    let id: String = row.get(0)?;
    let template: String = row.get(1)?;
    let params_json: String = row.get(2)?;
    let cadence: String = row.get(3)?;
    let enabled_i: i64 = row.get(4)?;
    let created_at: String = row.get(5)?;
    let updated_at: String = row.get(6)?;

    Ok((|| {
        let params: TemplateParams = serde_json::from_str(&params_json)
            .map_err(|e| FeedError::Storage(format!("parse params for '{id}': {e}")))?;
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| FeedError::Storage(format!("parse created_at: {e}")))?
            .with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| FeedError::Storage(format!("parse updated_at: {e}")))?
            .with_timezone(&Utc);
        Ok(FeedRecord {
            id,
            template,
            params,
            cadence,
            enabled: enabled_i != 0,
            created_at,
            updated_at,
        })
    })())
}

/// Convenience builder for tests / `/watch` registration.
pub fn new_record(
    id: impl Into<String>,
    template: impl Into<String>,
    params: TemplateParams,
    cadence: impl Into<String>,
) -> FeedRecord {
    let now = Utc::now();
    FeedRecord {
        id: id.into(),
        template: template.into(),
        params,
        cadence: cadence.into(),
        enabled: true,
        created_at: now,
        updated_at: now,
    }
}

/// Re-export so callers can `use arawn_feeds::Value` if they want.
#[allow(unused_imports)]
pub use serde_json::Value as JsonValue;
#[allow(unused)]
fn _value_marker(_: Value) {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn open_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        // Inline the V2 migration here so the test doesn't need
        // arawn-storage's refinery setup.
        conn.execute_batch(
            "CREATE TABLE feeds (
                id TEXT PRIMARY KEY,
                template TEXT NOT NULL,
                params TEXT NOT NULL,
                cadence TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn insert_get_round_trip() {
        let conn = open_test_db();
        let store = FeedStore::new(&conn);
        let rec = new_record(
            "slack-design",
            "slack/channel-archive",
            TemplateParams::new(json!({ "channel": "#design" })),
            "*/15 * * * *",
        );
        store.insert(&rec).unwrap();
        let got = store.get("slack-design").unwrap().unwrap();
        assert_eq!(got.template, "slack/channel-archive");
        assert_eq!(got.cadence, "*/15 * * * *");
        assert_eq!(got.params.get_str("channel"), Some("#design"));
        assert!(got.enabled);
    }

    #[test]
    fn list_enabled_omits_disabled() {
        let conn = open_test_db();
        let store = FeedStore::new(&conn);
        store
            .insert(&new_record(
                "a",
                "stub/echo",
                TemplateParams::default(),
                "*/15 * * * *",
            ))
            .unwrap();
        let mut b = new_record(
            "b",
            "stub/echo",
            TemplateParams::default(),
            "*/15 * * * *",
        );
        b.enabled = false;
        store.insert(&b).unwrap();

        let listed: Vec<_> = store
            .list_enabled()
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(listed, vec!["a".to_string()]);
    }

    #[test]
    fn set_enabled_round_trips() {
        let conn = open_test_db();
        let store = FeedStore::new(&conn);
        let rec = new_record("x", "stub/echo", TemplateParams::default(), "*/15 * * * *");
        store.insert(&rec).unwrap();
        store.set_enabled("x", false).unwrap();
        let got = store.get("x").unwrap().unwrap();
        assert!(!got.enabled);
        store.set_enabled("x", true).unwrap();
        let got = store.get("x").unwrap().unwrap();
        assert!(got.enabled);
    }

    #[test]
    fn set_enabled_errors_for_unknown_id() {
        let conn = open_test_db();
        let store = FeedStore::new(&conn);
        let err = store.set_enabled("nope", false).unwrap_err();
        assert!(matches!(err, FeedError::Storage(_)));
    }

    #[test]
    fn delete_removes_row() {
        let conn = open_test_db();
        let store = FeedStore::new(&conn);
        let rec = new_record("z", "stub/echo", TemplateParams::default(), "*/15 * * * *");
        store.insert(&rec).unwrap();
        store.delete("z").unwrap();
        assert!(store.get("z").unwrap().is_none());
    }
}
