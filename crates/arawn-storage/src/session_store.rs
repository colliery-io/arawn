use chrono::{DateTime, Utc};
use uuid::Uuid;

use arawn_core::{Session, SessionStats};

use crate::database::Database;
use crate::error::StorageError;

/// CRUD operations for session metadata in SQLite.
pub struct SessionStore<'a> {
    db: &'a Database,
}

impl<'a> SessionStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, session: &Session) -> Result<(), StorageError> {
        self.db.conn().execute(
            "INSERT INTO sessions (id, workstream_id, created_at) VALUES (?1, ?2, ?3)",
            (
                session.id.to_string(),
                session.workstream_id().map(|id| id.to_string()),
                session.created_at.to_rfc3339(),
            ),
        )?;
        Ok(())
    }

    pub fn get(&self, id: Uuid) -> Result<Option<SessionMeta>, StorageError> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, workstream_id, created_at, input_tokens, output_tokens, turns, tool_calls FROM sessions WHERE id = ?1",
        )?;

        let result = stmt.query_row([id.to_string()], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                workstream_id: row.get(1)?,
                created_at: row.get(2)?,
                input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                turns: row.get(5)?,
                tool_calls: row.get(6)?,
            })
        });

        match result {
            Ok(row) => Ok(Some(row.into_meta()?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_for_workstream(&self, ws_id: Uuid) -> Result<Vec<SessionMeta>, StorageError> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, workstream_id, created_at, input_tokens, output_tokens, turns, tool_calls FROM sessions WHERE workstream_id = ?1 ORDER BY created_at",
        )?;

        let rows = stmt.query_map([ws_id.to_string()], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                workstream_id: row.get(1)?,
                created_at: row.get(2)?,
                input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                turns: row.get(5)?,
                tool_calls: row.get(6)?,
            })
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?.into_meta()?);
        }
        Ok(sessions)
    }

    pub fn list_scratch(&self) -> Result<Vec<SessionMeta>, StorageError> {
        let mut stmt = self.db.conn().prepare(
            "SELECT id, workstream_id, created_at, input_tokens, output_tokens, turns, tool_calls FROM sessions WHERE workstream_id IS NULL ORDER BY created_at",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                workstream_id: row.get(1)?,
                created_at: row.get(2)?,
                input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                turns: row.get(5)?,
                tool_calls: row.get(6)?,
            })
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?.into_meta()?);
        }
        Ok(sessions)
    }

    /// Delete a session record from SQLite by ID.
    pub fn delete(&self, session_id: Uuid) -> Result<bool, StorageError> {
        let rows = self.db.conn().execute(
            "DELETE FROM sessions WHERE id = ?1",
            [session_id.to_string()],
        )?;
        Ok(rows > 0)
    }

    /// Update session token/turn stats in SQLite.
    pub fn update_stats(&self, session_id: Uuid, stats: &SessionStats) -> Result<(), StorageError> {
        self.db.conn().execute(
            "UPDATE sessions SET input_tokens = ?1, output_tokens = ?2, turns = ?3, tool_calls = ?4 WHERE id = ?5",
            (
                stats.input_tokens as i64,
                stats.output_tokens as i64,
                stats.turns as i64,
                stats.tool_calls as i64,
                session_id.to_string(),
            ),
        )?;
        Ok(())
    }

    pub fn update_workstream_id(
        &self,
        session_id: Uuid,
        new_ws_id: Uuid,
    ) -> Result<bool, StorageError> {
        let affected = self.db.conn().execute(
            "UPDATE sessions SET workstream_id = ?1 WHERE id = ?2 AND workstream_id IS NULL",
            (new_ws_id.to_string(), session_id.to_string()),
        )?;
        Ok(affected > 0)
    }
}

/// Session metadata as stored in SQLite (no messages — those are in JSONL).
#[derive(Debug, Clone)]
pub struct SessionMeta {
    pub id: Uuid,
    pub workstream_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub stats: SessionStats,
}

impl SessionMeta {
    /// Convert to an arawn_core::Session (without messages — load those separately).
    /// Note: This creates a new Session with a new UUID. To reconstruct with the
    /// stored ID, use Session::with_id (to be added) or set up the session manually.
    pub fn into_session(self) -> Session {
        match self.workstream_id {
            Some(ws_id) => Session::new(ws_id),
            None => Session::scratch(),
        }
    }
}

struct SessionRow {
    id: String,
    workstream_id: Option<String>,
    created_at: String,
    input_tokens: i64,
    output_tokens: i64,
    turns: i64,
    tool_calls: i64,
}

impl SessionRow {
    fn into_meta(self) -> Result<SessionMeta, StorageError> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| StorageError::InvalidOperation(format!("invalid UUID: {e}")))?;
        let workstream_id = self
            .workstream_id
            .map(|s| {
                Uuid::parse_str(&s)
                    .map_err(|e| StorageError::InvalidOperation(format!("invalid UUID: {e}")))
            })
            .transpose()?;
        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map_err(|e| StorageError::InvalidOperation(format!("invalid timestamp: {e}")))?
            .with_timezone(&Utc);

        Ok(SessionMeta {
            id,
            workstream_id,
            created_at,
            stats: SessionStats {
                input_tokens: self.input_tokens as u64,
                output_tokens: self.output_tokens as u64,
                turns: self.turns as u32,
                tool_calls: self.tool_calls as u32,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workstream_store::WorkstreamStore;

    fn setup() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn create_and_get_session() {
        let db = setup();
        let store = SessionStore::new(&db);
        // Need a workstream first (foreign key)
        let ws = arawn_core::Workstream::new("test", "/tmp/test");
        WorkstreamStore::new(&db).create(&ws).unwrap();

        let session = Session::new(ws.id);
        store.create(&session).unwrap();

        let meta = store.get(session.id).unwrap().unwrap();
        assert_eq!(meta.id, session.id);
        assert_eq!(meta.workstream_id, Some(ws.id));
    }

    #[test]
    fn create_scratch_session() {
        let db = setup();
        let store = SessionStore::new(&db);

        let session = Session::scratch();
        store.create(&session).unwrap();

        let meta = store.get(session.id).unwrap().unwrap();
        assert_eq!(meta.id, session.id);
        assert!(meta.workstream_id.is_none());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let db = setup();
        let store = SessionStore::new(&db);
        assert!(store.get(Uuid::new_v4()).unwrap().is_none());
    }

    #[test]
    fn list_for_workstream() {
        let db = setup();
        let ss = SessionStore::new(&db);
        let ws_store = WorkstreamStore::new(&db);

        let ws1 = arawn_core::Workstream::new("ws1", "/tmp/ws1");
        let ws2 = arawn_core::Workstream::new("ws2", "/tmp/ws2");
        ws_store.create(&ws1).unwrap();
        ws_store.create(&ws2).unwrap();

        let s1 = Session::new(ws1.id);
        let s2 = Session::new(ws1.id);
        let s3 = Session::new(ws2.id);
        ss.create(&s1).unwrap();
        ss.create(&s2).unwrap();
        ss.create(&s3).unwrap();

        let ws1_sessions = ss.list_for_workstream(ws1.id).unwrap();
        assert_eq!(ws1_sessions.len(), 2);

        let ws2_sessions = ss.list_for_workstream(ws2.id).unwrap();
        assert_eq!(ws2_sessions.len(), 1);
    }

    #[test]
    fn list_scratch_sessions() {
        let db = setup();
        let ss = SessionStore::new(&db);
        let ws_store = WorkstreamStore::new(&db);

        let ws = arawn_core::Workstream::new("ws", "/tmp/ws");
        ws_store.create(&ws).unwrap();

        let bound = Session::new(ws.id);
        let scratch1 = Session::scratch();
        let scratch2 = Session::scratch();
        ss.create(&bound).unwrap();
        ss.create(&scratch1).unwrap();
        ss.create(&scratch2).unwrap();

        let scratches = ss.list_scratch().unwrap();
        assert_eq!(scratches.len(), 2);
        assert!(scratches.iter().all(|s| s.workstream_id.is_none()));
    }

    #[test]
    fn update_workstream_id_promotes_scratch() {
        let db = setup();
        let ss = SessionStore::new(&db);
        let ws_store = WorkstreamStore::new(&db);

        let ws = arawn_core::Workstream::new("target", "/tmp/target");
        ws_store.create(&ws).unwrap();

        let session = Session::scratch();
        ss.create(&session).unwrap();

        assert!(ss.update_workstream_id(session.id, ws.id).unwrap());

        let meta = ss.get(session.id).unwrap().unwrap();
        assert_eq!(meta.workstream_id, Some(ws.id));
    }

    #[test]
    fn update_workstream_id_on_bound_session_returns_false() {
        let db = setup();
        let ss = SessionStore::new(&db);
        let ws_store = WorkstreamStore::new(&db);

        let ws = arawn_core::Workstream::new("ws", "/tmp/ws");
        ws_store.create(&ws).unwrap();

        let session = Session::new(ws.id);
        ss.create(&session).unwrap();

        // Already bound — should not update
        let new_ws = arawn_core::Workstream::new("new", "/tmp/new");
        ws_store.create(&new_ws).unwrap();
        assert!(!ss.update_workstream_id(session.id, new_ws.id).unwrap());
    }
}
