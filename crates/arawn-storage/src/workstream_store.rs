use std::path::PathBuf;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use arawn_core::Workstream;

use crate::database::Database;
use crate::error::StorageError;

/// CRUD operations for workstream metadata in SQLite.
pub struct WorkstreamStore<'a> {
    db: &'a Database,
}

impl<'a> WorkstreamStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, ws: &Workstream) -> Result<(), StorageError> {
        self.db.conn().execute(
            "INSERT INTO workstreams (id, name, root_dir, created_at) VALUES (?1, ?2, ?3, ?4)",
            (
                ws.id.to_string(),
                &ws.name,
                ws.root_dir.to_string_lossy().to_string(),
                ws.created_at.to_rfc3339(),
            ),
        )?;
        Ok(())
    }

    pub fn get(&self, id: Uuid) -> Result<Option<Workstream>, StorageError> {
        let mut stmt = self
            .db
            .conn()
            .prepare("SELECT id, name, root_dir, created_at FROM workstreams WHERE id = ?1")?;

        let result = stmt.query_row([id.to_string()], |row| {
            Ok(WorkstreamRow {
                id: row.get(0)?,
                name: row.get(1)?,
                root_dir: row.get(2)?,
                created_at: row.get(3)?,
            })
        });

        match result {
            Ok(row) => Ok(Some(row.into_workstream()?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn find_by_name(&self, name: &str) -> Result<Option<Workstream>, StorageError> {
        let mut stmt = self
            .db
            .conn()
            .prepare("SELECT id, name, root_dir, created_at FROM workstreams WHERE name = ?1")?;

        let result = stmt.query_row([name], |row| {
            Ok(WorkstreamRow {
                id: row.get(0)?,
                name: row.get(1)?,
                root_dir: row.get(2)?,
                created_at: row.get(3)?,
            })
        });

        match result {
            Ok(row) => Ok(Some(row.into_workstream()?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list(&self) -> Result<Vec<Workstream>, StorageError> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, name, root_dir, created_at FROM workstreams ORDER BY created_at",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(WorkstreamRow {
                id: row.get(0)?,
                name: row.get(1)?,
                root_dir: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;

        let mut workstreams = Vec::new();
        for row in rows {
            workstreams.push(row?.into_workstream()?);
        }
        Ok(workstreams)
    }

    pub fn delete(&self, id: Uuid) -> Result<bool, StorageError> {
        let affected = self
            .db
            .conn()
            .execute("DELETE FROM workstreams WHERE id = ?1", [id.to_string()])?;
        Ok(affected > 0)
    }
}

struct WorkstreamRow {
    id: String,
    name: String,
    root_dir: String,
    created_at: String,
}

impl WorkstreamRow {
    fn into_workstream(self) -> Result<Workstream, StorageError> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| StorageError::InvalidOperation(format!("invalid UUID: {e}")))?;
        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map_err(|e| StorageError::InvalidOperation(format!("invalid timestamp: {e}")))?
            .with_timezone(&Utc);

        Ok(Workstream {
            id,
            name: self.name,
            root_dir: PathBuf::from(self.root_dir),
            created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn create_and_get_roundtrip() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        let ws = Workstream::new("Test WS", "/tmp/test-ws");

        store.create(&ws).unwrap();
        let loaded = store.get(ws.id).unwrap().unwrap();

        assert_eq!(loaded.id, ws.id);
        assert_eq!(loaded.name, "Test WS");
        assert_eq!(loaded.root_dir, PathBuf::from("/tmp/test-ws"));
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        assert!(store.get(Uuid::new_v4()).unwrap().is_none());
    }

    #[test]
    fn find_by_name() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        let ws = Workstream::new("findme", "/tmp/findme");
        store.create(&ws).unwrap();

        let found = store.find_by_name("findme").unwrap().unwrap();
        assert_eq!(found.id, ws.id);

        assert!(store.find_by_name("nope").unwrap().is_none());
    }

    #[test]
    fn list_workstreams() {
        let db = setup();
        let store = WorkstreamStore::new(&db);

        store.create(&Workstream::new("ws1", "/tmp/ws1")).unwrap();
        store.create(&Workstream::new("ws2", "/tmp/ws2")).unwrap();
        store.create(&Workstream::new("ws3", "/tmp/ws3")).unwrap();

        let all = store.list().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn delete_workstream() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        let ws = Workstream::new("deleteme", "/tmp/del");
        store.create(&ws).unwrap();

        assert!(store.delete(ws.id).unwrap());
        assert!(store.get(ws.id).unwrap().is_none());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let db = setup();
        let store = WorkstreamStore::new(&db);
        assert!(!store.delete(Uuid::new_v4()).unwrap());
    }
}
