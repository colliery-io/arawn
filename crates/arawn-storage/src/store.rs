use std::path::{Path, PathBuf};

use tracing::info;
use uuid::Uuid;

use arawn_core::{Lens, Message, Session};

use crate::database::Database;
use crate::error::StorageError;
use crate::jsonl::{JsonlMessageStore, lens_dir_name};
use crate::layout::DataLayout;
use crate::lens_store::LensStore;
use crate::session_store::{SessionMeta, SessionStore};

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
    /// Access the underlying `Database` for crates that need to
    /// construct their own per-table stores (e.g. arawn-extractor's
    /// `ExtractorCursorStore`).
    pub fn database(&self) -> &Database {
        &self.db
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Get the JSONL message store (for direct access in service layer).
    pub fn message_store(&self) -> &JsonlMessageStore {
        &self.messages
    }

    // --- Lens operations ---

    pub fn create_lens(&self, ws: &Lens) -> Result<(), StorageError> {
        // `scratch` is reserved by the registry's `create` path — route
        // callers through `ensure_scratch` for backward compatibility
        // with code that pre-dated the registry refactor.
        if ws.name == arawn_core::SCRATCH_NAME {
            self.ensure_scratch_lens()?;
            return Ok(());
        }
        let store = LensStore::new(&self.db);
        store.create(ws)?;

        // Create lens directory under lenses/<name>/
        let ws_dir = self.data_dir.join("lenses").join(&ws.name);
        std::fs::create_dir_all(&ws_dir)?;

        Ok(())
    }

    pub fn get_lens(&self, id: Uuid) -> Result<Option<Lens>, StorageError> {
        LensStore::new(&self.db).get(id)
    }

    pub fn find_lens_by_name(&self, name: &str) -> Result<Option<Lens>, StorageError> {
        LensStore::new(&self.db).find_by_name(name)
    }

    pub fn list_lenses(&self) -> Result<Vec<Lens>, StorageError> {
        LensStore::new(&self.db).list()
    }

    pub fn list_all_lenses(&self) -> Result<Vec<Lens>, StorageError> {
        LensStore::new(&self.db).list_all()
    }

    pub fn update_lens_description(
        &self,
        name: &str,
        description: &str,
    ) -> Result<(), StorageError> {
        LensStore::new(&self.db).update_description(name, description)
    }

    pub fn add_lens_binding(&self, name: &str, feed_id: &str) -> Result<(), StorageError> {
        LensStore::new(&self.db).add_binding(name, feed_id)
    }

    pub fn remove_lens_binding(&self, name: &str, feed_id: &str) -> Result<(), StorageError> {
        LensStore::new(&self.db).remove_binding(name, feed_id)
    }

    /// Find the lens (by name) that has the given `feed_id` in
    /// its bindings list. Returns `Ok(None)` when no lens owns
    /// the feed (e.g. unbound system feeds, or feed_ids that fell out
    /// of the registry). The `feeds` table itself has no
    /// `lens_id` column today; the mapping lives in the
    /// `lenses.bindings` JSON array per lens row, so this
    /// scans active (non-archived) lenses in updated-at order.
    ///
    /// Cache the result at the call site — lenses change rarely
    /// at runtime and per-row lookups would be wasteful.
    pub fn find_lens_for_feed(&self, feed_id: &str) -> Result<Option<String>, StorageError> {
        let lenses = LensStore::new(&self.db).list()?;
        for ws in lenses {
            if ws.bindings.iter().any(|b| b == feed_id) {
                return Ok(Some(ws.name));
            }
        }
        Ok(None)
    }

    pub fn soft_delete_lens(&self, name: &str) -> Result<(), StorageError> {
        LensStore::new(&self.db).soft_delete(name)
    }

    /// Idempotently ensure the `scratch` lens exists. Safe to
    /// call on every boot; the on-disk dir is created if absent.
    pub fn ensure_scratch_lens(&self) -> Result<Lens, StorageError> {
        let scratch_dir = self.data_dir.join("lenses").join("scratch");
        let _ = std::fs::create_dir_all(&scratch_dir);
        LensStore::new(&self.db).ensure_scratch(&scratch_dir)
    }

    // --- Session operations ---

    pub fn create_session(&self, session: &Session) -> Result<(), StorageError> {
        SessionStore::new(&self.db).create(session)
    }

    pub fn get_session_meta(&self, id: Uuid) -> Result<Option<SessionMeta>, StorageError> {
        SessionStore::new(&self.db).get(id)
    }

    pub fn list_sessions_for_lens(&self, ws_id: Uuid) -> Result<Vec<SessionMeta>, StorageError> {
        SessionStore::new(&self.db).list_for_lens(ws_id)
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

        // Check lens-bound sessions
        let lenses = LensStore::new(&self.db).list()?;
        for ws in &lenses {
            let ws_dir = lens_dir_name(&ws.name, ws.id);
            let sessions = session_store.list_for_lens(ws.id)?;
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

    /// Resolve the directory name for a lens by UUID.
    /// Uses name if available, falls back to UUID string.
    fn resolve_ws_dir(&self, ws_id: Option<Uuid>) -> Result<String, StorageError> {
        match ws_id {
            Some(id) => {
                let ws = LensStore::new(&self.db).get(id)?.ok_or_else(|| {
                    StorageError::InvalidOperation(format!("lens {id} not found"))
                })?;
                Ok(lens_dir_name(&ws.name, ws.id))
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

        let ws_dir = self.resolve_ws_dir(meta.lens_id)?;
        let all_messages = self.messages.load(id, &ws_dir).await?;
        let messages = Session::load_compacted(all_messages);

        Ok(Some(Session::from_parts_with_stats(
            meta.id,
            meta.lens_id,
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
        lens_dir: &str,
        msg: &Message,
    ) -> Result<(), StorageError> {
        self.messages.append(session_id, lens_dir, msg).await
    }

    pub async fn load_messages(
        &self,
        session_id: Uuid,
        lens_dir: &str,
    ) -> Result<Vec<Message>, StorageError> {
        self.messages.load(session_id, lens_dir).await
    }

    /// Resolve the sandbox root for a session.
    pub fn sandbox_for(&self, lens_dir: &str, session_id: Uuid, is_scratch: bool) -> PathBuf {
        self.messages.sandbox_dir(lens_dir, session_id, is_scratch)
    }
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
        assert!(tmp.path().join("lenses").is_dir());
    }

    #[test]
    fn open_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let _store1 = Store::open(tmp.path()).unwrap();
        drop(_store1);
        let _store2 = Store::open(tmp.path()).unwrap();
    }

    #[test]
    fn create_and_list_lenses() {
        let (_tmp, store) = setup();
        let ws = Lens::new("test", "/tmp/test");
        store.create_lens(&ws).unwrap();

        let list = store.list_lenses().unwrap();
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
        let ws = Lens::new("ws", "/tmp/ws");
        store.create_lens(&ws).unwrap();

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
        assert_eq!(loaded.lens_id(), Some(ws.id));
        assert_eq!(loaded.messages().len(), 1);
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
        assert!(dir.to_string_lossy().contains("lenses/scratch"));
        assert!(dir.to_string_lossy().contains(&sid.to_string()));
        assert!(!dir.to_string_lossy().ends_with("workspace"));
    }

    #[tokio::test]
    async fn sandbox_for_named_is_shared() {
        let (_tmp, store) = setup();
        let sid = Uuid::nil();
        let dir = store.sandbox_for("my-project", sid, false);
        assert!(dir.to_string_lossy().contains("lenses/my-project"));
        assert!(!dir.to_string_lossy().contains(&sid.to_string()));
    }
}
