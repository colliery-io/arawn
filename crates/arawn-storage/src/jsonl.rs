use std::path::PathBuf;

use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::warn;
use uuid::Uuid;

use arawn_core::Message;

use crate::error::StorageError;

/// JSONL-based message persistence.
///
/// Layout (under data_dir/workstreams/):
///   scratch/<session-uuid>/messages.jsonl      — scratch sessions (each gets own workspace)
///   <ws-name>/<session-uuid>/messages.jsonl    — named workstream sessions
pub struct JsonlMessageStore {
    data_dir: PathBuf,
}

impl JsonlMessageStore {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// Append a message to the session's JSONL file.
    pub async fn append(
        &self,
        session_id: Uuid,
        workstream_dir: &str,
        msg: &Message,
    ) -> Result<(), StorageError> {
        let path = self.session_path(session_id, workstream_dir);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let is_new = !path.exists();

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        // Write version header on first message (new files only)
        if is_new {
            file.write_all(b"{\"_version\":1}\n").await?;
        }

        let mut line = serde_json::to_string(msg)?;
        line.push('\n');
        file.write_all(line.as_bytes()).await?;
        Ok(())
    }

    /// Load all messages for a session from its JSONL file.
    pub async fn load(
        &self,
        session_id: Uuid,
        workstream_dir: &str,
    ) -> Result<Vec<Message>, StorageError> {
        let path = self.session_path(session_id, workstream_dir);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut messages = Vec::new();
        let mut line_num = 0u64;

        while let Some(line) = lines.next_line().await? {
            line_num += 1;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Skip version header lines (future migration support)
            if trimmed.starts_with(r#"{"_version""#) {
                continue;
            }
            match serde_json::from_str::<Message>(trimmed) {
                Ok(msg) => messages.push(msg),
                Err(e) => {
                    warn!(
                        ?path,
                        line_num,
                        error = %e,
                        content = &trimmed[..trimmed.len().min(100)],
                        "skipping malformed JSONL line"
                    );
                }
            }
        }

        Ok(messages)
    }

    /// Move a session's JSONL file from one workstream directory to another.
    /// Used during session promotion.
    pub async fn move_session(
        &self,
        session_id: Uuid,
        from_dir: &str,
        to_dir: &str,
    ) -> Result<(), StorageError> {
        let from = self.session_path(session_id, from_dir);
        let to = self.session_path(session_id, to_dir);

        if !from.exists() {
            // Nothing to move — session had no messages yet
            return Ok(());
        }

        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::rename(&from, &to).await?;
        Ok(())
    }

    /// Resolve the filesystem path for a session's JSONL file.
    /// Layout: workstreams/<ws-dir>/<session-uuid>/messages.jsonl
    fn session_path(&self, session_id: Uuid, workstream_dir: &str) -> PathBuf {
        self.data_dir
            .join("workstreams")
            .join(workstream_dir)
            .join(session_id.to_string())
            .join("messages.jsonl")
    }

    /// Get the path for a session (exposed for testing/debugging).
    pub fn path_for(&self, session_id: Uuid, workstream_dir: &str) -> PathBuf {
        self.session_path(session_id, workstream_dir)
    }

    /// Resolve the sandbox root for a session.
    /// Scratch: workstreams/scratch/<session-uuid>/
    /// Named:  workstreams/<ws-name>/
    ///
    /// The `workspace/` subdirectory within this root is a convention for
    /// agent-created artifacts, but the entire session/workstream folder
    /// is the sandbox boundary.
    pub fn sandbox_dir(&self, workstream_dir: &str, session_id: Uuid, is_scratch: bool) -> PathBuf {
        if is_scratch {
            self.data_dir
                .join("workstreams")
                .join(workstream_dir)
                .join(session_id.to_string())
        } else {
            self.data_dir.join("workstreams").join(workstream_dir)
        }
    }
}

/// Resolve a workstream directory name: use name if non-empty, fall back to UUID.
pub fn workstream_dir_name(name: &str, id: Uuid) -> String {
    if name.is_empty() {
        id.to_string()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::ToolUse;
    use serde_json::json;
    use tempfile::TempDir;

    fn setup() -> (TempDir, JsonlMessageStore) {
        let tmp = TempDir::new().unwrap();
        let store = JsonlMessageStore::new(tmp.path());
        (tmp, store)
    }

    #[tokio::test]
    async fn append_and_load_roundtrip() {
        let (_tmp, store) = setup();
        let session_id = Uuid::new_v4();

        let msgs = vec![
            Message::User {
                content: "hello".into(),
            },
            Message::Assistant {
                content: "hi there".into(),
                tool_uses: vec![],
            },
            Message::User {
                content: "thanks".into(),
            },
        ];

        for msg in &msgs {
            store.append(session_id, "test-ws", msg).await.unwrap();
        }

        let loaded = store.load(session_id, "test-ws").await.unwrap();
        assert_eq!(loaded.len(), 3);

        match &loaded[0] {
            Message::User { content } => assert_eq!(content, "hello"),
            _ => panic!("expected User"),
        }
        match &loaded[1] {
            Message::Assistant { content, .. } => assert_eq!(content, "hi there"),
            _ => panic!("expected Assistant"),
        }
        match &loaded[2] {
            Message::User { content } => assert_eq!(content, "thanks"),
            _ => panic!("expected User"),
        }
    }

    #[tokio::test]
    async fn append_twice_accumulates() {
        let (_tmp, store) = setup();
        let session_id = Uuid::new_v4();

        store
            .append(
                session_id,
                "myws",
                &Message::User {
                    content: "first".into(),
                },
            )
            .await
            .unwrap();

        store
            .append(
                session_id,
                "myws",
                &Message::User {
                    content: "second".into(),
                },
            )
            .await
            .unwrap();

        let loaded = store.load(session_id, "myws").await.unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[tokio::test]
    async fn load_nonexistent_returns_empty() {
        let (_tmp, store) = setup();
        let loaded = store.load(Uuid::new_v4(), "nope").await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn scratch_session_path() {
        let (_tmp, store) = setup();
        let session_id = Uuid::new_v4();

        store
            .append(
                session_id,
                "scratch",
                &Message::User {
                    content: "scratch msg".into(),
                },
            )
            .await
            .unwrap();

        let loaded = store.load(session_id, "scratch").await.unwrap();
        assert_eq!(loaded.len(), 1);

        let path = store.path_for(session_id, "scratch");
        assert!(path.to_string_lossy().contains("workstreams/scratch"));
        assert!(path.to_string_lossy().contains("messages.jsonl"));
    }

    #[tokio::test]
    async fn move_session_relocates_file() {
        let (_tmp, store) = setup();
        let session_id = Uuid::new_v4();

        // Write to scratch
        store
            .append(
                session_id,
                "scratch",
                &Message::User {
                    content: "before promotion".into(),
                },
            )
            .await
            .unwrap();

        let scratch_path = store.path_for(session_id, "scratch");
        assert!(scratch_path.exists());

        // Move to named workstream
        store
            .move_session(session_id, "scratch", "my-project")
            .await
            .unwrap();

        // Old path gone, new path exists
        assert!(!scratch_path.exists());
        let new_path = store.path_for(session_id, "my-project");
        assert!(new_path.exists());

        // Messages loadable from new location
        let loaded = store.load(session_id, "my-project").await.unwrap();
        assert_eq!(loaded.len(), 1);
        match &loaded[0] {
            Message::User { content } => assert_eq!(content, "before promotion"),
            _ => panic!("expected User"),
        }
    }

    #[tokio::test]
    async fn move_nonexistent_session_is_ok() {
        let (_tmp, store) = setup();
        store
            .move_session(Uuid::new_v4(), "scratch", "target")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn jsonl_each_line_is_valid_json() {
        let (_tmp, store) = setup();
        let session_id = Uuid::new_v4();

        store
            .append(
                session_id,
                "test-ws",
                &Message::User {
                    content: "line 1".into(),
                },
            )
            .await
            .unwrap();
        store
            .append(
                session_id,
                "test-ws",
                &Message::Assistant {
                    content: "line 2".into(),
                    tool_uses: vec![ToolUse {
                        id: "c1".into(),
                        name: "think".into(),
                        input: json!({"thought": "hmm"}),
                    }],
                },
            )
            .await
            .unwrap();

        let path = store.path_for(session_id, "test-ws");
        let raw = tokio::fs::read_to_string(&path).await.unwrap();
        for line in raw.lines() {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(parsed.is_object());
        }
    }

    #[test]
    fn sandbox_dir_scratch_is_per_session() {
        let store = JsonlMessageStore::new("/data");
        let sid = Uuid::nil();
        let dir = store.sandbox_dir("scratch", sid, true);
        assert_eq!(
            dir,
            PathBuf::from(format!("/data/workstreams/scratch/{sid}"))
        );
    }

    #[test]
    fn sandbox_dir_named_is_shared() {
        let store = JsonlMessageStore::new("/data");
        let sid = Uuid::nil();
        let dir = store.sandbox_dir("my-project", sid, false);
        assert_eq!(dir, PathBuf::from("/data/workstreams/my-project"));
    }

    #[test]
    fn workstream_dir_name_prefers_name() {
        let id = Uuid::nil();
        assert_eq!(workstream_dir_name("scratch", id), "scratch");
        assert_eq!(workstream_dir_name("my-project", id), "my-project");
    }

    #[test]
    fn workstream_dir_name_falls_back_to_uuid() {
        let id = Uuid::nil();
        assert_eq!(workstream_dir_name("", id), id.to_string());
    }

    #[tokio::test]
    async fn load_skips_malformed_lines() {
        let (_tmp, store) = setup();
        let session_id = Uuid::new_v4();
        let path = store.session_path(session_id, "test-ws");
        tokio::fs::create_dir_all(path.parent().unwrap())
            .await
            .unwrap();

        // Get the correct serialization format
        let msg1 = serde_json::to_string(&Message::User { content: "first".into() }).unwrap();
        let msg2 = serde_json::to_string(&Message::User { content: "second".into() }).unwrap();
        let msg3 = serde_json::to_string(&Message::User { content: "third".into() }).unwrap();

        // Write a mix of valid and invalid lines
        let content = format!(
            "{{\"_version\":1}}\n\
             {msg1}\n\
             THIS IS NOT VALID JSON\n\
             {msg2}\n\
             \n\
             also bad\n\
             {msg3}\n"
        );
        tokio::fs::write(&path, content).await.unwrap();

        let loaded = store.load(session_id, "test-ws").await.unwrap();
        // Should have 3 valid messages, skipping the 2 bad lines and version header
        assert_eq!(loaded.len(), 3);
    }

    #[tokio::test]
    async fn new_file_has_version_header() {
        let (_tmp, store) = setup();
        let session_id = Uuid::new_v4();

        store
            .append(
                session_id,
                "test-ws",
                &Message::User {
                    content: "hello".into(),
                },
            )
            .await
            .unwrap();

        let path = store.path_for(session_id, "test-ws");
        let raw = tokio::fs::read_to_string(&path).await.unwrap();
        let first_line = raw.lines().next().unwrap();
        assert!(
            first_line.contains("\"_version\""),
            "first line should be version header, got: {first_line}"
        );
    }
}
