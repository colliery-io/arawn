//! Atomic read/write of `meta.json` at a feed dir root.
//!
//! Why atomic: a torn write (process killed mid-write) shouldn't leave
//! `meta.json` half-formed. We write to a sibling temp file then rename
//! into place — POSIX guarantees rename-over is atomic on the same
//! filesystem.

use std::path::Path;

use crate::error::FeedError;
use crate::types::FeedMeta;

const META_FILENAME: &str = "meta.json";

pub struct MetaStore;

impl MetaStore {
    /// Read `feed_dir/meta.json`. Returns `None` if the file doesn't
    /// exist (first run). Returns `Err` only on actual I/O / parse
    /// failure of an existing file.
    pub fn read(feed_dir: &Path) -> Result<Option<FeedMeta>, FeedError> {
        let path = feed_dir.join(META_FILENAME);
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s)
                .map(Some)
                .map_err(|e| FeedError::Storage(format!("parse {}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(FeedError::Storage(format!(
                "read {}: {e}",
                path.display()
            ))),
        }
    }

    /// Atomically write `meta.json` to `feed_dir`. Creates the dir if it
    /// doesn't exist. Uses a sibling temp file + rename so a process
    /// kill mid-write leaves the prior version intact.
    pub fn write(feed_dir: &Path, meta: &FeedMeta) -> Result<(), FeedError> {
        std::fs::create_dir_all(feed_dir).map_err(|e| {
            FeedError::Storage(format!("create dir {}: {e}", feed_dir.display()))
        })?;

        let path = feed_dir.join(META_FILENAME);
        let tmp = feed_dir.join(format!("{META_FILENAME}.tmp"));

        let body = serde_json::to_vec_pretty(meta)
            .map_err(|e| FeedError::Storage(format!("serialize meta: {e}")))?;

        std::fs::write(&tmp, &body)
            .map_err(|e| FeedError::Storage(format!("write {}: {e}", tmp.display())))?;

        std::fs::rename(&tmp, &path).map_err(|e| {
            FeedError::Storage(format!(
                "rename {} -> {}: {e}",
                tmp.display(),
                path.display()
            ))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TemplateParams;
    use serde_json::json;
    use tempfile::tempdir;

    fn sample_meta() -> FeedMeta {
        FeedMeta::new(
            "stub/echo",
            TemplateParams::new(json!({ "message": "hi" })),
            json!({ "run_count": 0 }),
        )
    }

    #[test]
    fn read_returns_none_when_missing() {
        let tmp = tempdir().unwrap();
        let result = MetaStore::read(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn write_then_read_round_trips() {
        let tmp = tempdir().unwrap();
        let meta = sample_meta();
        MetaStore::write(tmp.path(), &meta).unwrap();
        let loaded = MetaStore::read(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.template, "stub/echo");
        assert_eq!(loaded.params.get_str("message"), Some("hi"));
    }

    #[test]
    fn write_creates_feed_dir_if_missing() {
        let tmp = tempdir().unwrap();
        let nested = tmp.path().join("a/b/c");
        let meta = sample_meta();
        MetaStore::write(&nested, &meta).unwrap();
        assert!(nested.join("meta.json").exists());
    }

    #[test]
    fn atomic_write_does_not_corrupt_on_replace() {
        // Write v1, then overwrite with v2. Reading should return v2,
        // never anything in between.
        let tmp = tempdir().unwrap();
        let mut meta = sample_meta();
        MetaStore::write(tmp.path(), &meta).unwrap();

        meta.run_count = 5;
        meta.cursor = json!({ "run_count": 5 });
        MetaStore::write(tmp.path(), &meta).unwrap();

        let loaded = MetaStore::read(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.run_count, 5);
        assert_eq!(loaded.cursor["run_count"], 5);
    }
}
