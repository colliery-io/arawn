//! Per-row extraction log (ARAWN-T-0484).
//!
//! Backs the operator tooling for the extraction pipeline: record what the
//! extractor decided for each (lens, projection row), explain it after the
//! fact, and let a row be dismissed so it's skipped on future passes. One
//! row per `(lens_name, projection_id)`; the latest extraction upserts.

use chrono::Utc;
use rusqlite::OptionalExtension;

use crate::database::Database;
use crate::error::StorageError;

/// Terminal outcome of extracting one projection row for one lens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionOutcome {
    /// Entities/relations were written.
    Ok,
    /// In scope, but the extractor produced nothing.
    Empty,
    /// Classified out of scope for the lens.
    Skipped,
}

impl ExtractionOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            ExtractionOutcome::Ok => "ok",
            ExtractionOutcome::Empty => "empty",
            ExtractionOutcome::Skipped => "skipped",
        }
    }
}

/// A recorded extraction decision for one row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionRecord {
    pub lens_name: String,
    pub projection_id: String,
    pub run_id: String,
    /// `ok` | `empty` | `skipped`.
    pub outcome: String,
    pub reason: Option<String>,
    pub dismissed: bool,
    pub updated_at: String,
}

pub struct ExtractionLogStore<'a> {
    db: &'a Database,
}

impl<'a> ExtractionLogStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Upsert the extraction outcome for a row. Preserves an existing
    /// `dismissed` flag (a dismissed row that's re-evaluated stays dismissed
    /// until explicitly un-dismissed) — though callers should skip dismissed
    /// rows before extracting, this is belt-and-suspenders.
    pub fn record(
        &self,
        lens_name: &str,
        projection_id: &str,
        run_id: &str,
        outcome: ExtractionOutcome,
        reason: Option<&str>,
    ) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();
        self.db.conn().execute(
            "INSERT INTO extraction_log \
                 (lens_name, projection_id, run_id, outcome, reason, dismissed, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6) \
             ON CONFLICT(lens_name, projection_id) DO UPDATE SET \
                 run_id = excluded.run_id, outcome = excluded.outcome, \
                 reason = excluded.reason, updated_at = excluded.updated_at",
            rusqlite::params![
                lens_name,
                projection_id,
                run_id,
                outcome.as_str(),
                reason,
                now
            ],
        )?;
        Ok(())
    }

    /// Read the extraction record for a row, if any.
    pub fn get(
        &self,
        lens_name: &str,
        projection_id: &str,
    ) -> Result<Option<ExtractionRecord>, StorageError> {
        self.db
            .conn()
            .query_row(
                "SELECT lens_name, projection_id, run_id, outcome, reason, dismissed, updated_at \
                 FROM extraction_log WHERE lens_name = ?1 AND projection_id = ?2",
                [lens_name, projection_id],
                |row| {
                    Ok(ExtractionRecord {
                        lens_name: row.get(0)?,
                        projection_id: row.get(1)?,
                        run_id: row.get(2)?,
                        outcome: row.get(3)?,
                        reason: row.get(4)?,
                        dismissed: row.get::<_, i64>(5)? != 0,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Most recently updated extraction records, newest first. Backs the GUI
    /// extraction-provenance surface (ARAWN-T-0499).
    pub fn list_recent(&self, limit: usize) -> Result<Vec<ExtractionRecord>, StorageError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT lens_name, projection_id, run_id, outcome, reason, dismissed, updated_at \
             FROM extraction_log ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |row| {
            Ok(ExtractionRecord {
                lens_name: row.get(0)?,
                projection_id: row.get(1)?,
                run_id: row.get(2)?,
                outcome: row.get(3)?,
                reason: row.get(4)?,
                dismissed: row.get::<_, i64>(5)? != 0,
                updated_at: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Whether a row has been dismissed (so the extractor should skip it).
    pub fn is_dismissed(&self, lens_name: &str, projection_id: &str) -> Result<bool, StorageError> {
        let flag: Option<i64> = self
            .db
            .conn()
            .query_row(
                "SELECT dismissed FROM extraction_log WHERE lens_name = ?1 AND projection_id = ?2",
                [lens_name, projection_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(flag.unwrap_or(0) != 0)
    }

    /// Mark a row dismissed (or un-dismissed). Inserts a placeholder record
    /// if the row was never extracted, so a pre-emptive dismiss still sticks.
    pub fn set_dismissed(
        &self,
        lens_name: &str,
        projection_id: &str,
        dismissed: bool,
    ) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();
        self.db.conn().execute(
            "INSERT INTO extraction_log \
                 (lens_name, projection_id, run_id, outcome, reason, dismissed, updated_at) \
             VALUES (?1, ?2, '', 'skipped', NULL, ?3, ?4) \
             ON CONFLICT(lens_name, projection_id) DO UPDATE SET \
                 dismissed = excluded.dismissed, updated_at = excluded.updated_at",
            rusqlite::params![lens_name, projection_id, dismissed as i64, now],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn record_and_get_roundtrip() {
        let db = db();
        let s = ExtractionLogStore::new(&db);
        s.record(
            "pat",
            "m1",
            "run-1",
            ExtractionOutcome::Skipped,
            Some("off topic"),
        )
        .unwrap();
        let r = s.get("pat", "m1").unwrap().unwrap();
        assert_eq!(r.outcome, "skipped");
        assert_eq!(r.reason.as_deref(), Some("off topic"));
        assert!(!r.dismissed);
        assert_eq!(r.run_id, "run-1");
    }

    #[test]
    fn list_recent_returns_newest_first_capped() {
        let db = db();
        let s = ExtractionLogStore::new(&db);
        // Distinct (lens, projection) rows (the PK) with increasing timestamps.
        for (i, proj) in ["m1", "m2", "m3"].iter().enumerate() {
            s.record(
                "pat",
                proj,
                &format!("run-{i}"),
                ExtractionOutcome::Ok,
                None,
            )
            .unwrap();
            // Nudge updated_at ordering deterministically.
            db.conn()
                .execute(
                    "UPDATE extraction_log SET updated_at = ?1 WHERE projection_id = ?2",
                    rusqlite::params![format!("2026-06-1{}T00:00:00Z", i + 1), proj],
                )
                .unwrap();
        }
        let rows = s.list_recent(2).unwrap();
        assert_eq!(rows.len(), 2, "limit respected");
        assert_eq!(rows[0].projection_id, "m3", "newest first");
        assert_eq!(rows[1].projection_id, "m2");
        // Full read returns all three, still newest-first.
        let all = s.list_recent(10).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].projection_id, "m3");
        assert_eq!(all[2].projection_id, "m1");
    }

    #[test]
    fn record_distinguishes_empty_from_skipped() {
        let db = db();
        let s = ExtractionLogStore::new(&db);
        s.record("pat", "empty1", "r", ExtractionOutcome::Empty, None)
            .unwrap();
        s.record(
            "pat",
            "skip1",
            "r",
            ExtractionOutcome::Skipped,
            Some("nope"),
        )
        .unwrap();
        assert_eq!(s.get("pat", "empty1").unwrap().unwrap().outcome, "empty");
        assert_eq!(s.get("pat", "skip1").unwrap().unwrap().outcome, "skipped");
    }

    #[test]
    fn dismiss_sticks_and_survives_re_record() {
        let db = db();
        let s = ExtractionLogStore::new(&db);
        s.record("pat", "m1", "r1", ExtractionOutcome::Ok, None)
            .unwrap();
        s.set_dismissed("pat", "m1", true).unwrap();
        assert!(s.is_dismissed("pat", "m1").unwrap());
        // A later re-extraction record must not clear the dismiss flag.
        s.record("pat", "m1", "r2", ExtractionOutcome::Ok, None)
            .unwrap();
        assert!(
            s.is_dismissed("pat", "m1").unwrap(),
            "re-record must preserve dismissed"
        );
    }

    #[test]
    fn pre_emptive_dismiss_on_unseen_row() {
        let db = db();
        let s = ExtractionLogStore::new(&db);
        assert!(!s.is_dismissed("pat", "never-seen").unwrap());
        s.set_dismissed("pat", "never-seen", true).unwrap();
        assert!(s.is_dismissed("pat", "never-seen").unwrap());
    }
}
