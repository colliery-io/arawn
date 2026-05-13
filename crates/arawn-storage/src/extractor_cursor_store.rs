//! Per-workstream extractor cursors (I-0040 phase 4).
//!
//! One row per (workstream_name, feed_type). The extractor reads
//! `last_source_ts` to skip already-processed projection rows on the
//! next run and `advance` it monotonically as it makes progress.

use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;

use crate::database::Database;
use crate::error::StorageError;

pub struct ExtractorCursorStore<'a> {
    db: &'a Database,
}

#[derive(Debug, Clone)]
pub struct ExtractorCursor {
    pub workstream_name: String,
    pub feed_type: String,
    pub last_source_ts: Option<DateTime<Utc>>,
    pub last_processed_at: DateTime<Utc>,
}

impl<'a> ExtractorCursorStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Read the current cursor for (workstream, feed_type). Returns
    /// `None` when no row exists — the extractor treats this as
    /// "start from the beginning."
    pub fn get(
        &self,
        workstream_name: &str,
        feed_type: &str,
    ) -> Result<Option<ExtractorCursor>, StorageError> {
        let result = self
            .db
            .conn()
            .query_row(
                "SELECT workstream_name, feed_type, last_source_ts, last_processed_at \
                 FROM extractor_cursors \
                 WHERE workstream_name = ?1 AND feed_type = ?2",
                [workstream_name, feed_type],
                |row| {
                    let ws: String = row.get(0)?;
                    let ft: String = row.get(1)?;
                    let last_str: String = row.get(2)?;
                    let proc_str: String = row.get(3)?;
                    Ok((ws, ft, last_str, proc_str))
                },
            )
            .optional()?;
        match result {
            None => Ok(None),
            Some((ws, ft, last_str, proc_str)) => {
                let last_source_ts = if last_str.is_empty() {
                    None
                } else {
                    Some(parse_dt(&last_str)?)
                };
                Ok(Some(ExtractorCursor {
                    workstream_name: ws,
                    feed_type: ft,
                    last_source_ts,
                    last_processed_at: parse_dt(&proc_str)?,
                }))
            }
        }
    }

    /// Advance the cursor for (workstream, feed_type) to `new_source_ts`.
    /// Monotonic — refuses to move backwards. Upserts on first run.
    pub fn advance(
        &self,
        workstream_name: &str,
        feed_type: &str,
        new_source_ts: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let new_str = new_source_ts.to_rfc3339();
        let now = Utc::now().to_rfc3339();
        // Insert or update — only advance if the new ts is strictly
        // greater than the persisted one.
        self.db.conn().execute(
            "INSERT INTO extractor_cursors \
                 (workstream_name, feed_type, last_source_ts, last_processed_at) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(workstream_name, feed_type) DO UPDATE SET \
                 last_source_ts = CASE \
                     WHEN excluded.last_source_ts > extractor_cursors.last_source_ts \
                     THEN excluded.last_source_ts \
                     ELSE extractor_cursors.last_source_ts \
                 END, \
                 last_processed_at = ?4",
            (workstream_name, feed_type, &new_str, &now),
        )?;
        Ok(())
    }

    /// List every cursor row for a workstream — used by
    /// `/workstream show` and ops tooling.
    pub fn list_for_workstream(
        &self,
        workstream_name: &str,
    ) -> Result<Vec<ExtractorCursor>, StorageError> {
        let mut stmt = self.db.conn().prepare(
            "SELECT workstream_name, feed_type, last_source_ts, last_processed_at \
             FROM extractor_cursors \
             WHERE workstream_name = ?1 \
             ORDER BY feed_type",
        )?;
        let rows = stmt.query_map([workstream_name], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (ws, ft, last_str, proc_str) = r?;
            let last_source_ts = if last_str.is_empty() {
                None
            } else {
                Some(parse_dt(&last_str)?)
            };
            out.push(ExtractorCursor {
                workstream_name: ws,
                feed_type: ft,
                last_source_ts,
                last_processed_at: parse_dt(&proc_str)?,
            });
        }
        Ok(out)
    }
}

fn parse_dt(s: &str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| StorageError::InvalidOperation(format!("invalid timestamp: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn get_returns_none_for_unknown() {
        let db = db();
        let store = ExtractorCursorStore::new(&db);
        assert!(store.get("pat", "gmail_messages").unwrap().is_none());
    }

    #[test]
    fn advance_inserts_then_updates() {
        let db = db();
        let store = ExtractorCursorStore::new(&db);
        let t1: DateTime<Utc> = "2026-05-01T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let t2: DateTime<Utc> = "2026-05-02T10:00:00Z".parse::<DateTime<Utc>>().unwrap();

        store.advance("pat", "gmail_messages", t1).unwrap();
        let c = store.get("pat", "gmail_messages").unwrap().unwrap();
        assert_eq!(c.last_source_ts, Some(t1));

        store.advance("pat", "gmail_messages", t2).unwrap();
        let c = store.get("pat", "gmail_messages").unwrap().unwrap();
        assert_eq!(c.last_source_ts, Some(t2));
    }

    #[test]
    fn advance_refuses_to_go_backwards() {
        let db = db();
        let store = ExtractorCursorStore::new(&db);
        let t1: DateTime<Utc> = "2026-05-02T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let t0: DateTime<Utc> = "2026-05-01T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        store.advance("pat", "gmail_messages", t1).unwrap();
        // Going backwards is silently ignored — cursor stays at t1.
        store.advance("pat", "gmail_messages", t0).unwrap();
        let c = store.get("pat", "gmail_messages").unwrap().unwrap();
        assert_eq!(c.last_source_ts, Some(t1));
    }

    #[test]
    fn list_for_workstream_returns_all_feed_types() {
        let db = db();
        let store = ExtractorCursorStore::new(&db);
        let t: DateTime<Utc> = "2026-05-01T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        store.advance("pat", "gmail_messages", t).unwrap();
        store.advance("pat", "slack_messages", t).unwrap();
        store.advance("auth-migration", "jira_issues", t).unwrap();

        let pats = store.list_for_workstream("pat").unwrap();
        assert_eq!(pats.len(), 2);
        assert!(pats.iter().any(|c| c.feed_type == "gmail_messages"));
        assert!(pats.iter().any(|c| c.feed_type == "slack_messages"));

        let auth = store.list_for_workstream("auth-migration").unwrap();
        assert_eq!(auth.len(), 1);
    }
}
