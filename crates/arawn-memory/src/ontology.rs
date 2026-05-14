//! Per-workstream tag ontology — the closed list of tags an extractor
//! may emit into an entity's `tags_ontology` field. Colocated with the
//! workstream's `memory.db` so the ontology travels with the KB.
//!
//! Per ADR-0004 the ontology is required at workstream creation. Tags
//! are added via two paths:
//!
//! - Manual: `workstream_tag add <tag>` (audited via `added_via = 'manual'`).
//! - Promotion: the `tag-promoter` steward subroutine proposes; the
//!   user accepts via `workstream_apply <id>`; the accept-path inserts
//!   with `added_via = 'promotion'`.

use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::MemoryError;

/// One ontology row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OntologyEntry {
    pub tag: String,
    pub added_at: DateTime<Utc>,
    pub added_via: AddedVia,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddedVia {
    Manual,
    Promotion,
}

impl AddedVia {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Promotion => "promotion",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "manual" => Some(Self::Manual),
            "promotion" => Some(Self::Promotion),
            _ => None,
        }
    }
}

/// Read/write surface over the `workstream_tag_ontology` table.
///
/// Opens its own rusqlite connection to the workstream's `memory.db`.
/// Multiple connections to the same sqlite file are fine — graphqlite +
/// steward + ontology each maintain their own.
pub struct TagOntologyStore {
    conn: Arc<Mutex<Connection>>,
}

impl TagOntologyStore {
    /// Open (or create) the ontology table inside the workstream's
    /// `memory.db`. Path resolution mirrors `MemoryManager::for_workstream`:
    /// `<data_dir>/workstreams/<name>/memory.db`.
    pub fn open(data_dir: &Path, workstream_name: &str) -> Result<Self, MemoryError> {
        let ws_dir = data_dir.join("workstreams").join(workstream_name);
        Self::open_at(&ws_dir)
    }

    /// Open at an explicit workstream directory (the one that contains
    /// `memory.db`). Useful for callers that already have the
    /// workstream's root path on a `Workstream` record and don't want
    /// to re-derive `data_dir + name`.
    pub fn open_at(ws_dir: &Path) -> Result<Self, MemoryError> {
        std::fs::create_dir_all(ws_dir).map_err(|e| {
            MemoryError::Storage(format!("create workstream dir: {e}"))
        })?;
        let conn = Connection::open(ws_dir.join("memory.db"))
            .map_err(|e| MemoryError::Storage(format!("open ontology db: {e}")))?;
        ensure_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Add a tag. Idempotent — re-inserting an existing tag is a no-op
    /// and preserves the original `added_at` / `added_via`.
    pub fn add(&self, tag: &str, via: AddedVia) -> Result<(), MemoryError> {
        let tag = normalize_tag(tag);
        if tag.is_empty() {
            return Err(MemoryError::Storage("tag is empty".into()));
        }
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO workstream_tag_ontology \
             (tag, added_at, added_via) VALUES (?1, ?2, ?3)",
            params![tag, Utc::now().to_rfc3339(), via.as_str()],
        )
        .map_err(|e| MemoryError::Storage(format!("ontology insert: {e}")))?;
        Ok(())
    }

    /// Bulk-add — every tag in the list, all using the same `via`. Used
    /// at workstream creation to seed the initial ontology.
    pub fn add_many<I: IntoIterator<Item = String>>(
        &self,
        tags: I,
        via: AddedVia,
    ) -> Result<(), MemoryError> {
        for t in tags {
            self.add(&t, via)?;
        }
        Ok(())
    }

    /// Remove a tag. Returns true iff a row was deleted.
    pub fn remove(&self, tag: &str) -> Result<bool, MemoryError> {
        let tag = normalize_tag(tag);
        let conn = self.conn.lock().unwrap();
        let n = conn
            .execute(
                "DELETE FROM workstream_tag_ontology WHERE tag = ?1",
                params![tag],
            )
            .map_err(|e| MemoryError::Storage(format!("ontology delete: {e}")))?;
        Ok(n > 0)
    }

    pub fn contains(&self, tag: &str) -> Result<bool, MemoryError> {
        let tag = normalize_tag(tag);
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workstream_tag_ontology WHERE tag = ?1",
                params![tag],
                |r| r.get(0),
            )
            .map_err(|e| MemoryError::Storage(format!("ontology contains: {e}")))?;
        Ok(count > 0)
    }

    /// Return the full ontology, sorted alphabetically by tag.
    pub fn list(&self) -> Result<Vec<OntologyEntry>, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT tag, added_at, added_via FROM workstream_tag_ontology ORDER BY tag",
            )
            .map_err(|e| MemoryError::Storage(format!("ontology list prepare: {e}")))?;
        let rows = stmt
            .query_map([], |r| {
                Ok(parse_row(r))
            })
            .map_err(|e| MemoryError::Storage(format!("ontology list query: {e}")))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| MemoryError::Storage(e.to_string()))??);
        }
        Ok(out)
    }

    /// Convenience: tag strings only, in alpha order. Used by the
    /// extractor prompt and other consumers that only need the keys.
    pub fn tags(&self) -> Result<Vec<String>, MemoryError> {
        Ok(self.list()?.into_iter().map(|e| e.tag).collect())
    }

    pub fn count(&self) -> Result<usize, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workstream_tag_ontology",
                [],
                |r| r.get(0),
            )
            .map_err(|e| MemoryError::Storage(format!("ontology count: {e}")))?;
        Ok(n.max(0) as usize)
    }

    /// Fetch one entry by exact tag.
    pub fn get(&self, tag: &str) -> Result<Option<OntologyEntry>, MemoryError> {
        let tag = normalize_tag(tag);
        let conn = self.conn.lock().unwrap();
        let row: Option<Result<OntologyEntry, MemoryError>> = conn
            .query_row(
                "SELECT tag, added_at, added_via FROM workstream_tag_ontology WHERE tag = ?1",
                params![tag],
                |r| Ok(parse_row(r)),
            )
            .optional()
            .map_err(|e| MemoryError::Storage(format!("ontology get: {e}")))?;
        match row {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /// Filter `candidates` to the subset present in the ontology.
    /// Returns an empty vector when the ontology is empty (workstream
    /// would not have been creatable, but defensive). Tags are
    /// normalized before lookup so case/whitespace variants resolve.
    pub fn filter(&self, candidates: &[String]) -> Result<Vec<String>, MemoryError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let known: std::collections::HashSet<String> =
            self.tags()?.into_iter().collect();
        Ok(candidates
            .iter()
            .map(|t| normalize_tag(t))
            .filter(|t| known.contains(t))
            .collect())
    }
}

/// Canonical tag form — lowercase, trimmed. Two callers writing
/// `Falcon` and `falcon ` resolve to the same tag.
pub fn normalize_tag(tag: &str) -> String {
    tag.trim().to_lowercase()
}

fn ensure_schema(conn: &Connection) -> Result<(), MemoryError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS workstream_tag_ontology (
            tag TEXT PRIMARY KEY,
            added_at TEXT NOT NULL,
            added_via TEXT NOT NULL CHECK (added_via IN ('manual','promotion'))
        );
        CREATE INDEX IF NOT EXISTS idx_ontology_added_at
            ON workstream_tag_ontology(added_at);",
    )
    .map_err(|e| MemoryError::Storage(format!("ontology schema: {e}")))?;
    Ok(())
}

fn parse_row(r: &rusqlite::Row<'_>) -> Result<OntologyEntry, MemoryError> {
    let tag: String = r
        .get(0)
        .map_err(|e| MemoryError::Storage(format!("ontology col tag: {e}")))?;
    let added_at_str: String = r
        .get(1)
        .map_err(|e| MemoryError::Storage(format!("ontology col added_at: {e}")))?;
    let added_via_str: String = r
        .get(2)
        .map_err(|e| MemoryError::Storage(format!("ontology col added_via: {e}")))?;
    let added_at = DateTime::parse_from_rfc3339(&added_at_str)
        .map_err(|e| MemoryError::Storage(format!("parse added_at: {e}")))?
        .with_timezone(&Utc);
    let added_via = AddedVia::from_str(&added_via_str).ok_or_else(|| {
        MemoryError::Storage(format!("unknown added_via `{added_via_str}`"))
    })?;
    Ok(OntologyEntry {
        tag,
        added_at,
        added_via,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, TagOntologyStore) {
        let tmp = tempfile::tempdir().unwrap();
        let store = TagOntologyStore::open(tmp.path(), "ws-a").unwrap();
        (tmp, store)
    }

    #[test]
    fn add_and_list() {
        let (_tmp, store) = setup();
        store.add("falcon", AddedVia::Manual).unwrap();
        store.add("postgres", AddedVia::Manual).unwrap();
        let entries = store.list().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].tag, "falcon");
        assert_eq!(entries[1].tag, "postgres");
        assert_eq!(entries[0].added_via, AddedVia::Manual);
    }

    #[test]
    fn add_is_idempotent_and_preserves_initial_via() {
        let (_tmp, store) = setup();
        store.add("falcon", AddedVia::Manual).unwrap();
        store.add("falcon", AddedVia::Promotion).unwrap();
        let entries = store.list().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].added_via, AddedVia::Manual);
    }

    #[test]
    fn normalize_tag_collapses_case_and_whitespace() {
        assert_eq!(normalize_tag("  Falcon "), "falcon");
        assert_eq!(normalize_tag("POSTGRES"), "postgres");
    }

    #[test]
    fn contains_and_remove() {
        let (_tmp, store) = setup();
        store.add("falcon", AddedVia::Manual).unwrap();
        assert!(store.contains("falcon").unwrap());
        assert!(store.contains("FALCON").unwrap());
        assert!(store.remove("falcon").unwrap());
        assert!(!store.contains("falcon").unwrap());
        assert!(!store.remove("falcon").unwrap());
    }

    #[test]
    fn filter_returns_only_known_tags_normalized() {
        let (_tmp, store) = setup();
        store
            .add_many(
                ["falcon".into(), "postgres".into()],
                AddedVia::Manual,
            )
            .unwrap();
        let kept = store
            .filter(&[
                "Falcon".into(),
                "unknown".into(),
                "POSTGRES".into(),
                "  postgres  ".into(),
            ])
            .unwrap();
        // Dedup is up to the caller; filter preserves multiplicity.
        assert_eq!(
            kept,
            vec![
                "falcon".to_string(),
                "postgres".to_string(),
                "postgres".to_string(),
            ]
        );
    }

    #[test]
    fn rejects_empty_tag() {
        let (_tmp, store) = setup();
        assert!(store.add("", AddedVia::Manual).is_err());
        assert!(store.add("   ", AddedVia::Manual).is_err());
    }

    #[test]
    fn count_tracks_size() {
        let (_tmp, store) = setup();
        assert_eq!(store.count().unwrap(), 0);
        store.add("a", AddedVia::Manual).unwrap();
        store.add("b", AddedVia::Promotion).unwrap();
        assert_eq!(store.count().unwrap(), 2);
        store.remove("a").unwrap();
        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn schema_idempotent_on_reopen() {
        let tmp = tempfile::tempdir().unwrap();
        let s1 = TagOntologyStore::open(tmp.path(), "ws-x").unwrap();
        s1.add("foo", AddedVia::Manual).unwrap();
        drop(s1);
        let s2 = TagOntologyStore::open(tmp.path(), "ws-x").unwrap();
        assert!(s2.contains("foo").unwrap());
    }
}
