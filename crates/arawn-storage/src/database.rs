use std::path::Path;

use rusqlite::Connection;
use tracing::info;

use crate::error::StorageError;

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations");
}

/// SQLite database with automatic schema migrations via refinery.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the given path and run pending migrations.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", "5000")?;
        let mut db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Create an in-memory database for testing. Runs migrations immediately.
    pub fn in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let mut db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Run all pending refinery migrations.
    fn run_migrations(&mut self) -> Result<(), StorageError> {
        embedded::migrations::runner()
            .run(&mut self.conn)
            .map_err(|e| StorageError::Migration(e.to_string()))?;
        info!("database migrations applied");
        Ok(())
    }

    /// Get a reference to the underlying connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn in_memory_db_has_tables() {
        let db = Database::in_memory().unwrap();

        // Verify workstreams table exists
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='workstreams'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Verify sessions table exists
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn migrations_are_idempotent() {
        let _db = Database::in_memory().unwrap();
        // Tables exist from first run — opening again shouldn't fail
        // We can't re-run on the same in-memory connection easily,
        // but we can verify that a file-based DB survives two opens.
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");

        let _db1 = Database::open(&db_path).unwrap();
        drop(_db1);
        let _db2 = Database::open(&db_path).unwrap(); // second open should not fail
    }

    #[test]
    fn file_based_db_creates_file() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("arawn.db");
        assert!(!db_path.exists());

        let _db = Database::open(&db_path).unwrap();
        assert!(db_path.exists());
    }
}
