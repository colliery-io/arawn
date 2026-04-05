use std::path::{Path, PathBuf};

use tracing::{info, warn};
use uuid::Uuid;

use arawn_core::{Message, Session, Workstream};

use crate::database::Database;
use crate::error::StorageError;
use crate::jsonl::{JsonlMessageStore, workstream_dir_name};
use crate::layout::DataLayout;
use crate::session_store::{SessionMeta, SessionStore};
use crate::workstream_store::WorkstreamStore;

/// Unified persistence interface composing SQLite metadata + JSONL messages.
pub struct Store {
    db: Database,
    messages: JsonlMessageStore,
    data_dir: PathBuf,
}

impl Store {
    /// Open or create a store at the given data directory.
    /// Creates directories, opens/creates SQLite DB, runs migrations.
    pub fn open(data_dir: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let data_dir = data_dir.into();

        // Ensure filesystem layout
        DataLayout::v1().ensure(&data_dir)?;

        // Open database
        let db_path = data_dir.join("arawn.db");
        let db = Database::open(&db_path)?;

        let messages = JsonlMessageStore::new(&data_dir);

        info!(path = ?data_dir, "store opened");

        Ok(Self {
            db,
            messages,
            data_dir,
        })
    }

    /// Data directory path.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Get the JSONL message store (for direct access in service layer).
    pub fn message_store(&self) -> &JsonlMessageStore {
        &self.messages
    }

    // --- Workstream operations ---

    pub fn create_workstream(&self, ws: &Workstream) -> Result<(), StorageError> {
        let store = WorkstreamStore::new(&self.db);
        store.create(ws)?;

        // Create workstream directory under workstreams/<name>/
        let ws_dir = self.data_dir.join("workstreams").join(&ws.name);
        std::fs::create_dir_all(&ws_dir)?;

        Ok(())
    }

    pub fn get_workstream(&self, id: Uuid) -> Result<Option<Workstream>, StorageError> {
        WorkstreamStore::new(&self.db).get(id)
    }

    pub fn find_workstream_by_name(&self, name: &str) -> Result<Option<Workstream>, StorageError> {
        WorkstreamStore::new(&self.db).find_by_name(name)
    }

    pub fn list_workstreams(&self) -> Result<Vec<Workstream>, StorageError> {
        WorkstreamStore::new(&self.db).list()
    }

    // --- Session operations ---

    pub fn create_session(&self, session: &Session) -> Result<(), StorageError> {
        SessionStore::new(&self.db).create(session)
    }

    pub fn get_session_meta(&self, id: Uuid) -> Result<Option<SessionMeta>, StorageError> {
        SessionStore::new(&self.db).get(id)
    }

    pub fn list_sessions_for_workstream(
        &self,
        ws_id: Uuid,
    ) -> Result<Vec<SessionMeta>, StorageError> {
        SessionStore::new(&self.db).list_for_workstream(ws_id)
    }

    pub fn list_scratch_sessions(&self) -> Result<Vec<SessionMeta>, StorageError> {
        SessionStore::new(&self.db).list_scratch()
    }

    /// Remove SQLite session records whose JSONL files no longer exist on disk.
    /// Call on startup to clean up after manual filesystem deletions.
    pub fn reconcile_sessions(&self) -> Result<usize, StorageError> {
        let mut removed = 0;
        let session_store = SessionStore::new(&self.db);

        // Check scratch sessions
        let scratch_sessions = session_store.list_scratch()?;
        for meta in &scratch_sessions {
            let jsonl = self.messages.path_for(meta.id, "scratch");
            if !jsonl.exists() {
                session_store.delete(meta.id)?;
                removed += 1;
            }
        }

        // Check workstream-bound sessions
        let workstreams = WorkstreamStore::new(&self.db).list()?;
        for ws in &workstreams {
            let ws_dir = workstream_dir_name(&ws.name, ws.id);
            let sessions = session_store.list_for_workstream(ws.id)?;
            for meta in &sessions {
                let jsonl = self.messages.path_for(meta.id, &ws_dir);
                if !jsonl.exists() {
                    session_store.delete(meta.id)?;
                    removed += 1;
                }
            }
        }

        if removed > 0 {
            info!(removed, "reconciled stale sessions");
        }
        Ok(removed)
    }

    /// Resolve the directory name for a workstream by UUID.
    /// Uses name if available, falls back to UUID string.
    fn resolve_ws_dir(&self, ws_id: Option<Uuid>) -> Result<String, StorageError> {
        match ws_id {
            Some(id) => {
                let ws = WorkstreamStore::new(&self.db).get(id)?.ok_or_else(|| {
                    StorageError::InvalidOperation(format!("workstream {id} not found"))
                })?;
                Ok(workstream_dir_name(&ws.name, ws.id))
            }
            None => Ok("scratch".to_string()),
        }
    }

    /// Load a full session (metadata + messages) by ID.
    pub async fn load_session(&self, id: Uuid) -> Result<Option<Session>, StorageError> {
        let meta = match SessionStore::new(&self.db).get(id)? {
            Some(m) => m,
            None => return Ok(None),
        };

        let ws_dir = self.resolve_ws_dir(meta.workstream_id)?;
        let all_messages = self.messages.load(id, &ws_dir).await?;
        let messages = Session::load_compacted(all_messages);

        Ok(Some(Session::from_parts_with_stats(
            meta.id,
            meta.workstream_id,
            meta.created_at,
            messages,
            meta.stats,
        )))
    }

    pub fn update_session_stats(
        &self,
        session_id: Uuid,
        stats: &arawn_core::SessionStats,
    ) -> Result<(), StorageError> {
        SessionStore::new(&self.db).update_stats(session_id, stats)
    }

    // --- Message operations ---

    pub async fn append_message(
        &self,
        session_id: Uuid,
        workstream_dir: &str,
        msg: &Message,
    ) -> Result<(), StorageError> {
        self.messages.append(session_id, workstream_dir, msg).await
    }

    pub async fn load_messages(
        &self,
        session_id: Uuid,
        workstream_dir: &str,
    ) -> Result<Vec<Message>, StorageError> {
        self.messages.load(session_id, workstream_dir).await
    }

    // --- Promotion ---

    /// Promote a scratch session to a workstream.
    /// Updates SQLite metadata, moves the JSONL file, and merges the workspace.
    pub async fn promote_session(
        &self,
        session_id: Uuid,
        new_ws_id: Uuid,
    ) -> Result<(), StorageError> {
        let ws = WorkstreamStore::new(&self.db)
            .get(new_ws_id)?
            .ok_or_else(|| {
                StorageError::InvalidOperation(format!("workstream {new_ws_id} not found"))
            })?;
        let ws_dir = workstream_dir_name(&ws.name, ws.id);

        // Update SQLite — only works if session is currently scratch (workstream_id IS NULL)
        let updated = SessionStore::new(&self.db).update_workstream_id(session_id, new_ws_id)?;
        if !updated {
            return Err(StorageError::InvalidOperation(
                "session is not a scratch session or does not exist".into(),
            ));
        }

        // Move JSONL file from scratch to workstream directory
        if let Err(e) = self
            .messages
            .move_session(session_id, "scratch", &ws_dir)
            .await
        {
            warn!(
                session_id = %session_id,
                ws_dir = %ws_dir,
                error = %e,
                "failed to move JSONL file after SQLite promotion — inconsistent state"
            );
            return Err(e);
        }

        // Move scratch session workspace/ → workstream workspace/ (if it exists)
        let scratch_session_dir = self.messages.sandbox_dir("scratch", session_id, true);
        let scratch_workspace = scratch_session_dir.join("workspace");
        let target_workspace = self
            .messages
            .sandbox_dir(&ws_dir, session_id, false)
            .join("workspace");
        if scratch_workspace.exists() {
            if let Err(e) = tokio::fs::create_dir_all(&target_workspace).await {
                warn!(error = %e, "failed to create target workspace dir");
            }
            if let Err(e) = copy_dir_contents(&scratch_workspace, &target_workspace).await {
                warn!(error = %e, "failed to copy workspace contents during promotion");
            }
            let _ = tokio::fs::remove_dir_all(&scratch_workspace).await;
        }

        Ok(())
    }

    /// Resolve the sandbox root for a session.
    pub fn sandbox_for(&self, workstream_dir: &str, session_id: Uuid, is_scratch: bool) -> PathBuf {
        self.messages
            .sandbox_dir(workstream_dir, session_id, is_scratch)
    }
}

/// Recursively copy directory contents from src to dst.
async fn copy_dir_contents(src: &Path, dst: &Path) -> Result<(), StorageError> {
    let mut entries = tokio::fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type().await?.is_dir() {
            tokio::fs::create_dir_all(&dst_path).await?;
            Box::pin(copy_dir_contents(&src_path, &dst_path)).await?;
        } else {
            tokio::fs::copy(&src_path, &dst_path).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Store) {
        let tmp = TempDir::new().unwrap();
        let store = Store::open(tmp.path()).unwrap();
        (tmp, store)
    }

    #[test]
    fn open_creates_directories_and_db() {
        let tmp = TempDir::new().unwrap();
        let _store = Store::open(tmp.path()).unwrap();

        assert!(tmp.path().join("arawn.db").exists());
        assert!(tmp.path().join("workstreams").is_dir());
    }

    #[test]
    fn open_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let _store1 = Store::open(tmp.path()).unwrap();
        drop(_store1);
        let _store2 = Store::open(tmp.path()).unwrap();
    }

    #[test]
    fn create_and_list_workstreams() {
        let (_tmp, store) = setup();
        let ws = Workstream::new("test", "/tmp/test");
        store.create_workstream(&ws).unwrap();

        let list = store.list_workstreams().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "test");
    }

    #[tokio::test]
    async fn create_scratch_session_and_append_messages() {
        let (_tmp, store) = setup();
        let session = Session::scratch();
        store.create_session(&session).unwrap();

        store
            .append_message(
                session.id,
                "scratch",
                &Message::User {
                    content: "hello".into(),
                },
            )
            .await
            .unwrap();

        let messages = store.load_messages(session.id, "scratch").await.unwrap();
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn load_full_session() {
        let (_tmp, store) = setup();
        let ws = Workstream::new("ws", "/tmp/ws");
        store.create_workstream(&ws).unwrap();

        let session = Session::new(ws.id);
        store.create_session(&session).unwrap();

        store
            .append_message(
                session.id,
                "ws",
                &Message::User {
                    content: "test msg".into(),
                },
            )
            .await
            .unwrap();

        let loaded = store.load_session(session.id).await.unwrap().unwrap();
        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.workstream_id(), Some(ws.id));
        assert_eq!(loaded.messages().len(), 1);
    }

    #[tokio::test]
    async fn promote_session_full_flow() {
        let (_tmp, store) = setup();

        // Create target workstream
        let ws = Workstream::new("target", "/tmp/target");
        store.create_workstream(&ws).unwrap();

        // Create scratch session with messages
        let session = Session::scratch();
        store.create_session(&session).unwrap();

        store
            .append_message(
                session.id,
                "scratch",
                &Message::User {
                    content: "before promotion".into(),
                },
            )
            .await
            .unwrap();

        // Promote
        store.promote_session(session.id, ws.id).await.unwrap();

        // Verify: session now bound to workstream in SQLite
        let meta = store.get_session_meta(session.id).unwrap().unwrap();
        assert_eq!(meta.workstream_id, Some(ws.id));

        // Verify: messages loadable from new workstream location
        let messages = store.load_messages(session.id, "target").await.unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            Message::User { content } => assert_eq!(content, "before promotion"),
            _ => panic!("expected User"),
        }

        // Verify: old scratch location is empty
        let scratch_msgs = store.load_messages(session.id, "scratch").await.unwrap();
        assert!(scratch_msgs.is_empty());
    }

    #[tokio::test]
    async fn promote_bound_session_fails() {
        let (_tmp, store) = setup();
        let ws = Workstream::new("ws", "/tmp/ws");
        store.create_workstream(&ws).unwrap();

        let session = Session::new(ws.id);
        store.create_session(&session).unwrap();

        let ws2 = Workstream::new("ws2", "/tmp/ws2");
        store.create_workstream(&ws2).unwrap();

        let result = store.promote_session(session.id, ws2.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn load_nonexistent_session_returns_none() {
        let (_tmp, store) = setup();
        let result = store.load_session(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn sandbox_for_scratch_is_per_session() {
        let (_tmp, store) = setup();
        let sid = Uuid::nil();
        let dir = store.sandbox_for("scratch", sid, true);
        assert!(dir.to_string_lossy().contains("workstreams/scratch"));
        assert!(dir.to_string_lossy().contains(&sid.to_string()));
        assert!(!dir.to_string_lossy().ends_with("workspace"));
    }

    #[tokio::test]
    async fn sandbox_for_named_is_shared() {
        let (_tmp, store) = setup();
        let sid = Uuid::nil();
        let dir = store.sandbox_for("my-project", sid, false);
        assert!(dir.to_string_lossy().contains("workstreams/my-project"));
        assert!(!dir.to_string_lossy().contains(&sid.to_string()));
    }
}
