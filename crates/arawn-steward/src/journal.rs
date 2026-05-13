//! Append-only `steward_journal` colocated with each workstream KB.
//!
//! Per ARAWN-A-0003: every steward action — mutations and proposals —
//! gets exactly one row, written *before* the mutation runs (write-
//! ahead). `outputs_json` carries the diff payload sufficient for
//! `Journal::revert(action_id)` to reconstruct the inverse.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::StewardError;

/// One row about to be (or already) written to the journal. Strings
/// are pre-serialized JSON so the caller controls exactly what lands.
#[derive(Debug, Clone)]
pub struct JournalRecord {
    pub subroutine: String,
    pub action: String,
    pub inputs_json: String,
    pub outputs_json: String,
    pub model: String,
    pub prompt_hash: String,
    /// `false` for proposal-only subroutines (map / door-watch). The
    /// runner enforces that proposal-only actions stay applied=false.
    pub applied: bool,
}

/// A journal row as read back from sqlite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalRow {
    pub id: i64,
    pub ts: DateTime<Utc>,
    pub subroutine: String,
    pub action: String,
    pub inputs_json: String,
    pub outputs_json: String,
    pub model: String,
    pub prompt_hash: String,
    pub applied: bool,
    pub reverted_at: Option<DateTime<Utc>>,
}

/// Outcome of a `Journal::revert` call. The actual inverse is applied
/// by per-subroutine code; the journal only records the metadata flip
/// and returns a copy of the row for callers that need its payload.
#[derive(Debug, Clone)]
pub struct RevertResult {
    pub row: JournalRow,
    /// True if this call actually changed `reverted_at` from null; false
    /// if the row was already reverted (idempotent).
    pub newly_reverted: bool,
}

/// Outcome of `Journal::mark_applied`. Mirrors `RevertResult` so the
/// proposal-accept path has a symmetric idempotency surface.
#[derive(Debug, Clone)]
pub struct AppliedResult {
    pub row: JournalRow,
    pub newly_applied: bool,
}

/// Workstream-scoped journal. Opens its own rusqlite connection to the
/// workstream's `memory.db`. Multiple connections to the same file are
/// fine — graphqlite + steward live in the same db but use disjoint
/// tables.
pub struct Journal {
    conn: Arc<Mutex<Connection>>,
    workstream: String,
    path: PathBuf,
}

impl Journal {
    /// Open (or create) the journal for `workstream_name` rooted at
    /// `data_dir`. The workstream's KB lives at
    /// `<data_dir>/workstreams/<name>/memory.db`. Creates parent dirs
    /// if they don't exist so first-touch lazy-init mirrors what
    /// `MemoryManager::for_workstream` already does.
    pub fn open(data_dir: &Path, workstream_name: &str) -> Result<Self, StewardError> {
        let ws_dir = data_dir.join("workstreams").join(workstream_name);
        std::fs::create_dir_all(&ws_dir).map_err(|e| {
            StewardError::Storage(format!("create workstream dir {ws_dir:?}: {e}"))
        })?;
        let path = ws_dir.join("memory.db");
        let conn = Connection::open(&path)?;
        ensure_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            workstream: workstream_name.to_string(),
            path,
        })
    }

    pub fn workstream(&self) -> &str {
        &self.workstream
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write a journal row *before* the mutation. Returns the row id
    /// the caller threads through to its mutation; if the mutation
    /// fails the caller is responsible for `mark_failed` (a follow-up
    /// API) or rolling back the entire pass via sqlite transaction.
    ///
    /// Today this is a single INSERT — wrapping the write-ahead +
    /// mutation in one sqlite tx is the subroutine's job because the
    /// mutation reaches into graphqlite via a separate connection.
    pub fn write_ahead(&self, record: &JournalRecord) -> Result<i64, StewardError> {
        let ts = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO steward_journal \
             (ts, subroutine, action, inputs_json, outputs_json, \
              model, prompt_hash, applied) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                ts,
                record.subroutine,
                record.action,
                record.inputs_json,
                record.outputs_json,
                record.model,
                record.prompt_hash,
                if record.applied { 1 } else { 0 },
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Fetch one row by id.
    pub fn get(&self, id: i64) -> Result<Option<JournalRow>, StewardError> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                "SELECT id, ts, subroutine, action, inputs_json, outputs_json, \
                        model, prompt_hash, applied, reverted_at \
                 FROM steward_journal WHERE id = ?1",
                params![id],
                |r| Ok(row_to_record(r)),
            )
            .optional()?;
        row.transpose()
    }

    /// Last `limit` rows, newest first. Used by `/workstream journal`.
    pub fn recent(&self, limit: usize) -> Result<Vec<JournalRow>, StewardError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, ts, subroutine, action, inputs_json, outputs_json, \
                    model, prompt_hash, applied, reverted_at \
             FROM steward_journal ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |r| Ok(row_to_record(r)))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row??);
        }
        Ok(out)
    }

    /// Rows where `applied = 0` (proposals from map / door-watch) and
    /// `reverted_at` is null (still pending).
    pub fn pending_proposals(&self, limit: usize) -> Result<Vec<JournalRow>, StewardError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, ts, subroutine, action, inputs_json, outputs_json, \
                    model, prompt_hash, applied, reverted_at \
             FROM steward_journal \
             WHERE applied = 0 AND reverted_at IS NULL \
             ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |r| Ok(row_to_record(r)))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row??);
        }
        Ok(out)
    }

    /// Flip a row from `applied = false` to `applied = true`. Used by
    /// the proposal-accept path (`workstream_apply`). Idempotent: a row
    /// already applied returns `newly_applied = false`. Returns the
    /// (re-read) row so the caller has the post-flip state.
    pub fn mark_applied(&self, id: i64) -> Result<AppliedResult, StewardError> {
        let Some(row) = self.get(id)? else {
            return Err(StewardError::NotFound(format!("journal row {id}")));
        };
        if row.applied {
            return Ok(AppliedResult {
                row,
                newly_applied: false,
            });
        }
        if row.reverted_at.is_some() {
            return Err(StewardError::Journal(format!(
                "row {id} is reverted; cannot apply"
            )));
        }
        {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "UPDATE steward_journal SET applied = 1 WHERE id = ?1 \
                 AND applied = 0 AND reverted_at IS NULL",
                params![id],
            )?;
        }
        let row = self
            .get(id)?
            .ok_or_else(|| StewardError::Journal(format!("row {id} vanished post-apply")))?;
        Ok(AppliedResult {
            row,
            newly_applied: true,
        })
    }

    /// Mark a row reverted. Idempotent: a second call returns the row
    /// without re-flipping. Returns the row so callers can use the
    /// payload to apply the inverse (the actual KB mutation is per-
    /// subroutine and lives outside this module).
    pub fn revert(&self, id: i64) -> Result<RevertResult, StewardError> {
        let Some(row) = self.get(id)? else {
            return Err(StewardError::NotFound(format!("journal row {id}")));
        };
        if row.reverted_at.is_some() {
            return Ok(RevertResult {
                row,
                newly_reverted: false,
            });
        }
        let ts = Utc::now().to_rfc3339();
        {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "UPDATE steward_journal SET reverted_at = ?1 WHERE id = ?2 \
                 AND reverted_at IS NULL",
                params![ts, id],
            )?;
        }
        let row = self
            .get(id)?
            .ok_or_else(|| StewardError::Journal(format!("row {id} vanished post-revert")))?;
        Ok(RevertResult {
            row,
            newly_reverted: true,
        })
    }

    /// Build a deterministic prompt-hash id from arbitrary input bytes.
    /// Used by subroutines that want to mark journal rows with a hash
    /// of their prompt — uuid v5 over a fixed namespace is enough.
    pub fn prompt_hash(input: impl AsRef<[u8]>) -> String {
        let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, input.as_ref());
        id.simple().to_string()
    }
}

fn ensure_schema(conn: &Connection) -> Result<(), StewardError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS steward_journal (
            id INTEGER PRIMARY KEY,
            ts TEXT NOT NULL,
            subroutine TEXT NOT NULL,
            action TEXT NOT NULL,
            inputs_json TEXT NOT NULL,
            outputs_json TEXT NOT NULL,
            model TEXT NOT NULL,
            prompt_hash TEXT NOT NULL,
            applied INTEGER NOT NULL DEFAULT 1,
            reverted_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_journal_ts ON steward_journal(ts);
        CREATE INDEX IF NOT EXISTS idx_journal_pending
            ON steward_journal(applied, reverted_at);",
    )?;
    Ok(())
}

fn row_to_record(r: &rusqlite::Row<'_>) -> Result<JournalRow, StewardError> {
    let ts_str: String = r.get(1)?;
    let ts = DateTime::parse_from_rfc3339(&ts_str)
        .map_err(|e| StewardError::Parse(format!("ts: {e}")))?
        .with_timezone(&Utc);
    let reverted_at = r
        .get::<_, Option<String>>(9)?
        .map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|d| d.with_timezone(&Utc))
                .map_err(|e| StewardError::Parse(format!("reverted_at: {e}")))
        })
        .transpose()?;
    let applied: i64 = r.get(8)?;
    Ok(JournalRow {
        id: r.get(0)?,
        ts,
        subroutine: r.get(2)?,
        action: r.get(3)?,
        inputs_json: r.get(4)?,
        outputs_json: r.get(5)?,
        model: r.get(6)?,
        prompt_hash: r.get(7)?,
        applied: applied != 0,
        reverted_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> JournalRecord {
        JournalRecord {
            subroutine: "identity".into(),
            action: "noop".into(),
            inputs_json: "[]".into(),
            outputs_json: "{}".into(),
            model: "test-model".into(),
            prompt_hash: Journal::prompt_hash("hello"),
            applied: true,
        }
    }

    #[test]
    fn write_then_read() {
        let tmp = tempfile::tempdir().unwrap();
        let j = Journal::open(tmp.path(), "ws-a").unwrap();
        let id = j.write_ahead(&sample()).unwrap();
        assert!(id > 0);
        let row = j.get(id).unwrap().unwrap();
        assert_eq!(row.subroutine, "identity");
        assert!(row.applied);
        assert!(row.reverted_at.is_none());
    }

    #[test]
    fn revert_flips_metadata_idempotently() {
        let tmp = tempfile::tempdir().unwrap();
        let j = Journal::open(tmp.path(), "ws-a").unwrap();
        let id = j.write_ahead(&sample()).unwrap();
        let r1 = j.revert(id).unwrap();
        assert!(r1.newly_reverted);
        assert!(r1.row.reverted_at.is_some());
        let r2 = j.revert(id).unwrap();
        assert!(!r2.newly_reverted, "second revert should be a no-op");
        assert_eq!(r1.row.reverted_at, r2.row.reverted_at);
    }

    #[test]
    fn recent_returns_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        let j = Journal::open(tmp.path(), "ws-a").unwrap();
        let id1 = j.write_ahead(&sample()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id2 = j.write_ahead(&sample()).unwrap();
        let rows = j.recent(10).unwrap();
        assert_eq!(rows[0].id, id2);
        assert_eq!(rows[1].id, id1);
    }

    #[test]
    fn pending_proposals_filters_applied_and_reverted() {
        let tmp = tempfile::tempdir().unwrap();
        let j = Journal::open(tmp.path(), "ws-a").unwrap();
        let mut proposal = sample();
        proposal.subroutine = "map".into();
        proposal.applied = false;
        let prop_id = j.write_ahead(&proposal).unwrap();
        let _ = j.write_ahead(&sample()).unwrap();
        // Make a second proposal then revert it — should drop out of pending.
        let mut p2 = proposal.clone();
        p2.action = "propose_other".into();
        let p2_id = j.write_ahead(&p2).unwrap();
        j.revert(p2_id).unwrap();
        let pending = j.pending_proposals(10).unwrap();
        let ids: Vec<i64> = pending.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![prop_id]);
    }

    #[test]
    fn prompt_hash_is_deterministic() {
        let a = Journal::prompt_hash("x");
        let b = Journal::prompt_hash("x");
        let c = Journal::prompt_hash("y");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn schema_idempotent_on_reopen() {
        let tmp = tempfile::tempdir().unwrap();
        let _ = Journal::open(tmp.path(), "ws-a").unwrap();
        // Second open should not error or duplicate the table.
        let j = Journal::open(tmp.path(), "ws-a").unwrap();
        let id = j.write_ahead(&sample()).unwrap();
        assert!(id > 0);
    }
}
