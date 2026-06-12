//! Persisted background-job failure history (ARAWN-T-0477, I-0068 P2-1b).
//!
//! Two append-only tables on the shared `arawn.db` (migration V13):
//! `ceremony_run_history` (one row per ceremony dispatch) and
//! `steward_error_log` (one row per failed steward subroutine pass). The
//! writers live in `arawn-ceremonies` (raw `ConnHandle`) and `arawn-steward`
//! (via `Store::database`); the readers back the `/status` health surface
//! (`LocalService::status`). All four entry points are free functions over a
//! borrowed `rusqlite::Connection` so every caller — whichever connection
//! handle it holds onto — can reuse the same SQL.
//!
//! Both tables are pruned to a bounded row count on every insert so they
//! can't grow without limit. History writes are best-effort: a caller that
//! fails to record must log and carry on, never abort the run it's recording.

use rusqlite::Connection;

use crate::error::StorageError;

/// Max rows retained per history table. Older rows are pruned on insert.
pub const HISTORY_CAP: usize = 500;

/// One recorded ceremony dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CeremonyRunRecord {
    pub kind: String,
    pub period_key: String,
    /// `ok` | `skipped` | `error`.
    pub outcome: String,
    /// Failure text when `outcome == "error"`.
    pub error: Option<String>,
    /// RFC3339 timestamp.
    pub ran_at: String,
}

/// One recorded steward subroutine failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StewardErrorRecord {
    pub lens_name: String,
    pub subroutine: String,
    pub error: String,
    /// RFC3339 timestamp.
    pub failed_at: String,
}

/// Append a ceremony dispatch outcome, then prune to [`HISTORY_CAP`].
pub fn record_ceremony_run(
    conn: &Connection,
    kind: &str,
    period_key: &str,
    outcome: &str,
    error: Option<&str>,
    ran_at: &str,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO ceremony_run_history (kind, period_key, outcome, error, ran_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![kind, period_key, outcome, error, ran_at],
    )?;
    prune(conn, "ceremony_run_history")?;
    Ok(())
}

/// The most recent dispatch for each ceremony kind, newest first. Backs the
/// "latest run outcome per ceremony" line in `/status`.
pub fn latest_ceremony_runs(conn: &Connection) -> Result<Vec<CeremonyRunRecord>, StorageError> {
    // One row per kind: the max-id (most recent) row for that kind.
    let mut stmt = conn.prepare(
        "SELECT h.kind, h.period_key, h.outcome, h.error, h.ran_at \
         FROM ceremony_run_history h \
         JOIN (SELECT kind, MAX(id) AS mid FROM ceremony_run_history GROUP BY kind) m \
           ON h.id = m.mid \
         ORDER BY h.ran_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(CeremonyRunRecord {
            kind: row.get(0)?,
            period_key: row.get(1)?,
            outcome: row.get(2)?,
            error: row.get(3)?,
            ran_at: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Append a steward subroutine failure, then prune to [`HISTORY_CAP`].
pub fn record_steward_error(
    conn: &Connection,
    lens_name: &str,
    subroutine: &str,
    error: &str,
    failed_at: &str,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO steward_error_log (lens_name, subroutine, error, failed_at) \
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![lens_name, subroutine, error, failed_at],
    )?;
    prune(conn, "steward_error_log")?;
    Ok(())
}

/// The most recent steward errors, newest first, capped at `limit`.
pub fn recent_steward_errors(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<StewardErrorRecord>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT lens_name, subroutine, error, failed_at \
         FROM steward_error_log \
         ORDER BY id DESC \
         LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit as i64], |row| {
        Ok(StewardErrorRecord {
            lens_name: row.get(0)?,
            subroutine: row.get(1)?,
            error: row.get(2)?,
            failed_at: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Keep only the most recent [`HISTORY_CAP`] rows of `table`. `table` is a
/// hard-coded identifier (never user input), so it's interpolated rather
/// than bound.
fn prune(conn: &Connection, table: &str) -> Result<(), StorageError> {
    conn.execute(
        &format!(
            "DELETE FROM {table} WHERE id NOT IN \
             (SELECT id FROM {table} ORDER BY id DESC LIMIT {HISTORY_CAP})"
        ),
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    fn db() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn ceremony_run_history_records_and_reads_latest_per_kind() {
        let db = db();
        let c = db.conn();
        record_ceremony_run(c, "daily", "2026-06-10", "ok", None, "2026-06-10T07:00:00Z").unwrap();
        record_ceremony_run(
            c,
            "daily",
            "2026-06-11",
            "error",
            Some("compose failed"),
            "2026-06-11T07:00:00Z",
        )
        .unwrap();
        record_ceremony_run(c, "weekly", "2026-W23", "ok", None, "2026-06-08T07:00:00Z").unwrap();

        let latest = latest_ceremony_runs(c).unwrap();
        assert_eq!(latest.len(), 2, "one row per kind");
        let daily = latest.iter().find(|r| r.kind == "daily").unwrap();
        // The newest daily row wins — the error, not the earlier ok.
        assert_eq!(daily.outcome, "error");
        assert_eq!(daily.error.as_deref(), Some("compose failed"));
        assert_eq!(daily.period_key, "2026-06-11");
    }

    #[test]
    fn steward_error_log_records_and_reads_recent() {
        let db = db();
        let c = db.conn();
        record_steward_error(c, "pat", "identity", "boom", "2026-06-11T02:00:00Z").unwrap();
        record_steward_error(c, "auth", "cadence", "kaboom", "2026-06-11T02:05:00Z").unwrap();

        let recent = recent_steward_errors(c, 10).unwrap();
        assert_eq!(recent.len(), 2);
        // Newest first.
        assert_eq!(recent[0].subroutine, "cadence");
        assert_eq!(recent[0].error, "kaboom");
        assert_eq!(recent[1].lens_name, "pat");
    }

    #[test]
    fn history_is_pruned_to_cap() {
        let db = db();
        let c = db.conn();
        for i in 0..(HISTORY_CAP + 25) {
            record_steward_error(
                c,
                "pat",
                "identity",
                &format!("e{i}"),
                "2026-06-11T02:00:00Z",
            )
            .unwrap();
        }
        let count: i64 = c
            .query_row("SELECT COUNT(*) FROM steward_error_log", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count as usize, HISTORY_CAP, "older rows pruned to the cap");
    }
}
