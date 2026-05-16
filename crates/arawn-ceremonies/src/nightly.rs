//! Nightly maintenance tasks fired by cloacina cron in the binary.
//!
//! Today the only inhabitant is the Sunday-night retro sweep that
//! transitions un-reviewed retro tablets to `unreviewed` status so
//! the next weekly prep can detect "diary skipped 3 weeks running"
//! patterns. The cron expression (`59 23 * * SUN` in the user's
//! local zone) lives in the binary's wiring; the function itself
//! is timezone-agnostic — it sweeps every retro tablet that is
//! still `open` with no diary row from a prior ISO week.

use chrono::Utc;
use rusqlite::params;

use crate::CeremonyError;
use crate::engine::ConnHandle;
use crate::plugins::retro::RetroCeremony;

/// Transition `open` retro tablets that have no diary row to
/// `unreviewed`. Only touches tablets whose `period_key` is from a
/// strictly prior ISO week so the current week's retro has time to
/// be reviewed before the sweep marks it.
///
/// Returns the number of tablets transitioned. Idempotent — running
/// twice is a no-op on the second call.
pub fn sweep_unreviewed_retros(conn: &ConnHandle) -> Result<usize, CeremonyError> {
    let now_week = RetroCeremony::iso_week(Utc::now());
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
    let updated = conn
        .execute(
            "UPDATE ceremony_tablets SET status = 'unreviewed' \
             WHERE kind = 'retro' \
                   AND status = 'open' \
                   AND period_key < ?1 \
                   AND id NOT IN (SELECT tablet_id FROM ceremony_diary)",
            params![&now_week],
        )
        .map_err(|e| CeremonyError::Storage(format!("sweep_unreviewed_retros: {e}")))?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use tempfile::TempDir;

    fn open_test_db() -> (TempDir, ConnHandle) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let _db = arawn_storage::Database::open(&db_path).expect("migrations");
        drop(_db);
        let conn = rusqlite::Connection::open(&db_path).expect("open conn");
        (tmp, ConnHandle::new(conn))
    }

    fn insert_retro(conn: &ConnHandle, id: &str, period_key: &str, status: &str) {
        let c = conn.0.lock().unwrap();
        c.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
             VALUES (?1, 'retro', ?2, ?3, ?4, '[]')",
            params![id, period_key, "2026-05-15T16:00:00Z", status],
        )
        .unwrap();
    }

    fn insert_diary(conn: &ConnHandle, tablet_id: &str, body: &str) {
        let c = conn.0.lock().unwrap();
        c.execute(
            "INSERT INTO ceremony_diary (tablet_id, body, written_at, word_count) \
             VALUES (?1, ?2, ?3, ?4)",
            params![tablet_id, body, "2026-05-15T17:00:00Z", 3],
        )
        .unwrap();
    }

    fn status(conn: &ConnHandle, id: &str) -> String {
        let c = conn.0.lock().unwrap();
        c.query_row(
            "SELECT status FROM ceremony_tablets WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn open_retro_from_prior_week_without_diary_transitions() {
        let (_tmp, conn) = open_test_db();
        // Pick an obviously-prior ISO week. The sweep uses
        // `Utc::now()` for the cutoff; "2020-W01" is safely in the
        // past regardless of when this test runs.
        insert_retro(&conn, "retro-2020-W01", "2020-W01", "open");
        let n = sweep_unreviewed_retros(&conn).unwrap();
        assert_eq!(n, 1);
        assert_eq!(status(&conn, "retro-2020-W01"), "unreviewed");
    }

    #[tokio::test]
    async fn open_retro_with_diary_is_left_alone() {
        let (_tmp, conn) = open_test_db();
        insert_retro(&conn, "retro-2020-W02", "2020-W02", "open");
        insert_diary(&conn, "retro-2020-W02", "thoughts");
        let n = sweep_unreviewed_retros(&conn).unwrap();
        assert_eq!(n, 0);
        assert_eq!(status(&conn, "retro-2020-W02"), "open");
    }

    #[tokio::test]
    async fn reviewed_retro_is_left_alone() {
        let (_tmp, conn) = open_test_db();
        insert_retro(&conn, "retro-2020-W03", "2020-W03", "reviewed");
        let n = sweep_unreviewed_retros(&conn).unwrap();
        assert_eq!(n, 0);
        assert_eq!(status(&conn, "retro-2020-W03"), "reviewed");
    }

    #[tokio::test]
    async fn current_week_open_retro_is_skipped() {
        let (_tmp, conn) = open_test_db();
        // The sweep filters by period_key < current_iso_week, so
        // current-week tablets never get touched.
        let current = RetroCeremony::iso_week(Utc::now());
        insert_retro(&conn, "retro-current", &current, "open");
        let n = sweep_unreviewed_retros(&conn).unwrap();
        assert_eq!(n, 0);
        assert_eq!(status(&conn, "retro-current"), "open");
    }

    #[tokio::test]
    async fn sweep_is_idempotent() {
        let (_tmp, conn) = open_test_db();
        insert_retro(&conn, "retro-2020-W04", "2020-W04", "open");
        assert_eq!(sweep_unreviewed_retros(&conn).unwrap(), 1);
        assert_eq!(sweep_unreviewed_retros(&conn).unwrap(), 0);
        assert_eq!(status(&conn, "retro-2020-W04"), "unreviewed");
    }

    #[tokio::test]
    async fn sweep_only_touches_retros() {
        let (_tmp, conn) = open_test_db();
        // A daily tablet with status='open' shouldn't be touched
        // (sweep is retro-only).
        let c = conn.0.lock().unwrap();
        c.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
             VALUES ('daily-2020-01-01', 'daily', '2020-01-01', '2020-01-01T07:00:00Z', 'open', '[]')",
            [],
        )
        .unwrap();
        drop(c);
        sweep_unreviewed_retros(&conn).unwrap();
        assert_eq!(status(&conn, "daily-2020-01-01"), "open");
    }
}
