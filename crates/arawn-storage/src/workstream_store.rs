//! CRUD over the workstream registry (`workstreams` table).
//!
//! Phase 3 (I-0040) turned this from a small bookkeeping table into the
//! primary scope abstraction. Every workstream gets its own KB under
//! `<data_dir>/workstreams/<name>/memory.db`, and sessions bind to a
//! workstream by name. The `id` (Uuid) column is retained for
//! session-FK compatibility but `name` is the addressing primitive
//! for users.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use arawn_core::{SCRATCH_NAME, Workstream, WorkstreamNameError, validate_name};

use crate::database::Database;
use crate::error::StorageError;

/// Workstream registry. Wraps the `workstreams` table.
pub struct WorkstreamStore<'a> {
    db: &'a Database,
}

impl<'a> WorkstreamStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Idempotently create the `scratch` workstream at the given root.
    /// Safe to call on every boot — does nothing if scratch already exists.
    pub fn ensure_scratch(&self, scratch_root: &Path) -> Result<Workstream, StorageError> {
        if let Some(existing) = self.find_by_name(SCRATCH_NAME)? {
            return Ok(existing);
        }
        let ws = Workstream::scratch(scratch_root);
        self.insert_row(&ws)?;
        Ok(ws)
    }

    /// Create a new workstream. Validates the name, refuses `scratch`,
    /// and errors on duplicate names.
    pub fn create(&self, ws: &Workstream) -> Result<(), StorageError> {
        if ws.name == SCRATCH_NAME {
            return Err(StorageError::InvalidOperation(
                "the name 'scratch' is reserved".into(),
            ));
        }
        validate_name(&ws.name).map_err(name_err)?;
        if self.find_by_name(&ws.name)?.is_some() {
            return Err(StorageError::InvalidOperation(format!(
                "workstream '{}' already exists",
                ws.name
            )));
        }
        self.insert_row(ws)
    }

    fn insert_row(&self, ws: &Workstream) -> Result<(), StorageError> {
        let bindings_json = serde_json::to_string(&ws.bindings)
            .map_err(|e| StorageError::InvalidOperation(format!("serialize bindings: {e}")))?;
        let display_name = if ws.display_name.is_empty() {
            &ws.name
        } else {
            &ws.display_name
        };
        self.db.conn().execute(
            "INSERT INTO workstreams \
                (id, name, root_dir, created_at, display_name, description, \
                 bindings, archived, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            (
                ws.id.to_string(),
                &ws.name,
                ws.root_dir.to_string_lossy().to_string(),
                ws.created_at.to_rfc3339(),
                display_name,
                &ws.description,
                bindings_json,
                ws.archived as i32,
                ws.updated_at.to_rfc3339(),
            ),
        )?;
        Ok(())
    }

    pub fn get(&self, id: Uuid) -> Result<Option<Workstream>, StorageError> {
        Ok(self
            .db
            .conn()
            .query_row(SELECT_COLS_WHERE_ID, [id.to_string()], |row| {
                row_to_workstream(row).map_err(rusqlite_map_err)
            })
            .optional()?)
    }

    pub fn find_by_name(&self, name: &str) -> Result<Option<Workstream>, StorageError> {
        Ok(self
            .db
            .conn()
            .query_row(SELECT_COLS_WHERE_NAME, [name], |row| {
                row_to_workstream(row).map_err(rusqlite_map_err)
            })
            .optional()?)
    }

    /// List active (non-archived) workstreams, newest update first.
    pub fn list(&self) -> Result<Vec<Workstream>, StorageError> {
        self.list_with_archived(false)
    }

    /// List all workstreams including soft-deleted (archived) ones.
    pub fn list_all(&self) -> Result<Vec<Workstream>, StorageError> {
        self.list_with_archived(true)
    }

    fn list_with_archived(&self, include_archived: bool) -> Result<Vec<Workstream>, StorageError> {
        let mut stmt = self.db.conn().prepare(if include_archived {
            "SELECT id, name, root_dir, created_at, display_name, description, \
             bindings, archived, updated_at \
             FROM workstreams ORDER BY updated_at DESC"
        } else {
            "SELECT id, name, root_dir, created_at, display_name, description, \
             bindings, archived, updated_at \
             FROM workstreams WHERE archived = 0 ORDER BY updated_at DESC"
        })?;
        let rows = stmt
            .query_map([], |row| row_to_workstream(row).map_err(rusqlite_map_err))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn update_description(
        &self,
        name: &str,
        description: &str,
    ) -> Result<(), StorageError> {
        let updated = self.db.conn().execute(
            "UPDATE workstreams SET description = ?1, updated_at = ?2 WHERE name = ?3",
            (description, Utc::now().to_rfc3339(), name),
        )?;
        if updated == 0 {
            return Err(StorageError::InvalidOperation(format!(
                "workstream '{name}' not found"
            )));
        }
        Ok(())
    }

    pub fn set_bindings(&self, name: &str, bindings: &[String]) -> Result<(), StorageError> {
        let json = serde_json::to_string(bindings)
            .map_err(|e| StorageError::InvalidOperation(format!("serialize bindings: {e}")))?;
        let updated = self.db.conn().execute(
            "UPDATE workstreams SET bindings = ?1, updated_at = ?2 WHERE name = ?3",
            (json, Utc::now().to_rfc3339(), name),
        )?;
        if updated == 0 {
            return Err(StorageError::InvalidOperation(format!(
                "workstream '{name}' not found"
            )));
        }
        Ok(())
    }

    pub fn add_binding(&self, name: &str, feed_id: &str) -> Result<(), StorageError> {
        let ws = self
            .find_by_name(name)?
            .ok_or_else(|| StorageError::InvalidOperation(format!("workstream '{name}' not found")))?;
        let mut bindings = ws.bindings;
        if !bindings.iter().any(|b| b == feed_id) {
            bindings.push(feed_id.to_string());
        }
        self.set_bindings(name, &bindings)
    }

    pub fn remove_binding(&self, name: &str, feed_id: &str) -> Result<(), StorageError> {
        let ws = self
            .find_by_name(name)?
            .ok_or_else(|| StorageError::InvalidOperation(format!("workstream '{name}' not found")))?;
        let bindings: Vec<String> = ws.bindings.into_iter().filter(|b| b != feed_id).collect();
        self.set_bindings(name, &bindings)
    }

    /// Soft-delete: sets `archived = 1`. Refuses `scratch`. The on-disk
    /// KB at `<root_dir>/memory.db` is left intact — operator can clean
    /// up manually or restore by setting archived = 0.
    pub fn soft_delete(&self, name: &str) -> Result<(), StorageError> {
        if name == SCRATCH_NAME {
            return Err(StorageError::InvalidOperation(
                "the 'scratch' workstream cannot be deleted".into(),
            ));
        }
        let updated = self.db.conn().execute(
            "UPDATE workstreams SET archived = 1, updated_at = ?1 WHERE name = ?2",
            (Utc::now().to_rfc3339(), name),
        )?;
        if updated == 0 {
            return Err(StorageError::InvalidOperation(format!(
                "workstream '{name}' not found"
            )));
        }
        Ok(())
    }

    /// Hard-delete by id. Retained for backward compatibility with the
    /// V1 surface; new code paths should prefer `soft_delete(name)`.
    pub fn delete(&self, id: Uuid) -> Result<bool, StorageError> {
        let affected = self
            .db
            .conn()
            .execute("DELETE FROM workstreams WHERE id = ?1", [id.to_string()])?;
        Ok(affected > 0)
    }
}

const SELECT_COLS_WHERE_ID: &str =
    "SELECT id, name, root_dir, created_at, display_name, description, \
     bindings, archived, updated_at \
     FROM workstreams WHERE id = ?1";

const SELECT_COLS_WHERE_NAME: &str =
    "SELECT id, name, root_dir, created_at, display_name, description, \
     bindings, archived, updated_at \
     FROM workstreams WHERE name = ?1";

fn row_to_workstream(row: &rusqlite::Row<'_>) -> Result<Workstream, StorageError> {
    let id_str: String = row.get(0)?;
    let name: String = row.get(1)?;
    let root_dir: String = row.get(2)?;
    let created_str: String = row.get(3)?;
    let display_name: String = row.get(4)?;
    let description: String = row.get(5)?;
    let bindings_json: String = row.get(6)?;
    let archived: i32 = row.get(7)?;
    let updated_str: String = row.get(8)?;

    let id = Uuid::parse_str(&id_str)
        .map_err(|e| StorageError::InvalidOperation(format!("invalid UUID: {e}")))?;
    let created_at = parse_dt(&created_str)?;
    let updated_at = if updated_str.is_empty() {
        created_at
    } else {
        parse_dt(&updated_str)?
    };
    let bindings: Vec<String> = serde_json::from_str(&bindings_json)
        .map_err(|e| StorageError::InvalidOperation(format!("invalid bindings JSON: {e}")))?;

    Ok(Workstream {
        id,
        name: name.clone(),
        display_name: if display_name.is_empty() { name } else { display_name },
        description,
        root_dir: PathBuf::from(root_dir),
        bindings,
        archived: archived != 0,
        created_at,
        updated_at,
    })
}

fn parse_dt(s: &str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| StorageError::InvalidOperation(format!("invalid timestamp: {e}")))
}

fn rusqlite_map_err(e: StorageError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(e.to_string())))
}

fn name_err(e: WorkstreamNameError) -> StorageError {
    StorageError::InvalidOperation(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn create_and_roundtrip() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        let mut ws = Workstream::new("pat", "/tmp/pat");
        ws.description = "skip-level for pat".into();
        ws.bindings = vec!["slack-pat-dm".into(), "calendar-pat-1on1".into()];
        store.create(&ws).unwrap();
        let loaded = store.find_by_name("pat").unwrap().unwrap();
        assert_eq!(loaded.name, "pat");
        assert_eq!(loaded.description, "skip-level for pat");
        assert_eq!(loaded.bindings, vec!["slack-pat-dm", "calendar-pat-1on1"]);
        assert!(!loaded.archived);
    }

    #[test]
    fn create_rejects_scratch() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        let ws = Workstream::new(SCRATCH_NAME, "/tmp/scratch");
        let err = store.create(&ws).unwrap_err();
        assert!(format!("{err}").contains("reserved"));
    }

    #[test]
    fn create_rejects_invalid_slug() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        // Leading capital → BadLeading
        let ws_upper = Workstream::new("BadName", "/tmp/bad");
        let err = store.create(&ws_upper).unwrap_err();
        assert!(format!("{err}").contains("lowercase letter or digit"));
        // Space in body → BadChar(' ')
        let ws_space = Workstream::new("bad name", "/tmp/bad");
        let err = store.create(&ws_space).unwrap_err();
        assert!(format!("{err}").contains("invalid character"));
    }

    #[test]
    fn create_rejects_duplicate() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        store.create(&Workstream::new("dupe", "/tmp/a")).unwrap();
        let err = store.create(&Workstream::new("dupe", "/tmp/b")).unwrap_err();
        assert!(format!("{err}").contains("already exists"));
    }

    #[test]
    fn ensure_scratch_idempotent() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        let s1 = store.ensure_scratch(Path::new("/tmp/scratch")).unwrap();
        let s2 = store.ensure_scratch(Path::new("/tmp/scratch")).unwrap();
        assert_eq!(s1.id, s2.id, "scratch should be the same row on re-ensure");
        assert_eq!(s1.name, SCRATCH_NAME);
    }

    #[test]
    fn update_description() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        store.create(&Workstream::new("proj", "/tmp/p")).unwrap();
        store.update_description("proj", "auth migration").unwrap();
        let loaded = store.find_by_name("proj").unwrap().unwrap();
        assert_eq!(loaded.description, "auth migration");
    }

    #[test]
    fn bindings_add_and_remove() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        store.create(&Workstream::new("proj", "/tmp/p")).unwrap();
        store.add_binding("proj", "slack-eng").unwrap();
        store.add_binding("proj", "slack-eng").unwrap(); // dedup
        store.add_binding("proj", "drive-design").unwrap();
        let loaded = store.find_by_name("proj").unwrap().unwrap();
        assert_eq!(loaded.bindings, vec!["slack-eng", "drive-design"]);
        store.remove_binding("proj", "slack-eng").unwrap();
        let loaded = store.find_by_name("proj").unwrap().unwrap();
        assert_eq!(loaded.bindings, vec!["drive-design"]);
    }

    #[test]
    fn soft_delete_marks_archived() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        store.create(&Workstream::new("temp", "/tmp/t")).unwrap();
        store.soft_delete("temp").unwrap();
        // `list` excludes archived
        let active = store.list().unwrap();
        assert!(active.iter().all(|w| w.name != "temp"));
        // `list_all` includes them
        let all = store.list_all().unwrap();
        assert!(all.iter().any(|w| w.name == "temp" && w.archived));
    }

    #[test]
    fn soft_delete_refuses_scratch() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        store.ensure_scratch(Path::new("/tmp/scratch")).unwrap();
        let err = store.soft_delete(SCRATCH_NAME).unwrap_err();
        assert!(format!("{err}").contains("scratch"));
    }

    #[test]
    fn list_orders_by_updated_at_desc() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        store.create(&Workstream::new("old", "/tmp/o")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.create(&Workstream::new("new", "/tmp/n")).unwrap();
        let listed = store.list().unwrap();
        assert_eq!(listed[0].name, "new");
        assert_eq!(listed[1].name, "old");
    }
}
