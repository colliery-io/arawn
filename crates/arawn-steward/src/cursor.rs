//! Per-subroutine cursor — the latest `updated_at` of an entity the
//! subroutine has already considered in this workstream. Used by
//! re-shelve so a pass only touches entities created/updated since
//! the last pass.

use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::StewardError;

pub struct CursorStore {
    conn: Arc<Mutex<Connection>>,
}

impl CursorStore {
    /// Open (or create) the cursor table inside `<data_dir>/workstreams/<name>/memory.db`.
    pub fn open(data_dir: &Path, workstream_name: &str) -> Result<Self, StewardError> {
        let ws_dir = data_dir.join("workstreams").join(workstream_name);
        std::fs::create_dir_all(&ws_dir)
            .map_err(|e| StewardError::Storage(format!("create ws dir: {e}")))?;
        let conn = Connection::open(ws_dir.join("memory.db"))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS steward_cursors (
                subroutine TEXT PRIMARY KEY,
                last_updated_at TEXT NOT NULL
            );",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn get(&self, subroutine: &str) -> Result<Option<DateTime<Utc>>, StewardError> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                "SELECT last_updated_at FROM steward_cursors WHERE subroutine = ?1",
                params![subroutine],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        match row {
            Some(s) => Ok(Some(
                DateTime::parse_from_rfc3339(&s)
                    .map_err(|e| StewardError::Parse(format!("cursor ts: {e}")))?
                    .with_timezone(&Utc),
            )),
            None => Ok(None),
        }
    }

    /// Advance the cursor monotonically — never moves backwards.
    pub fn advance(&self, subroutine: &str, ts: DateTime<Utc>) -> Result<(), StewardError> {
        let ts_str = ts.to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO steward_cursors (subroutine, last_updated_at) VALUES (?1, ?2) \
             ON CONFLICT(subroutine) DO UPDATE SET \
               last_updated_at = CASE \
                 WHEN excluded.last_updated_at > steward_cursors.last_updated_at \
                 THEN excluded.last_updated_at \
                 ELSE steward_cursors.last_updated_at \
               END",
            params![subroutine, ts_str],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_monotonic() {
        let tmp = tempfile::tempdir().unwrap();
        let s = CursorStore::open(tmp.path(), "ws-a").unwrap();
        assert!(s.get("reshelve").unwrap().is_none());

        let t1 = Utc::now() - chrono::Duration::seconds(60);
        let t2 = Utc::now();
        s.advance("reshelve", t2).unwrap();
        assert_eq!(s.get("reshelve").unwrap().unwrap(), t2);

        // Older write must not regress
        s.advance("reshelve", t1).unwrap();
        assert_eq!(s.get("reshelve").unwrap().unwrap(), t2);
    }
}
