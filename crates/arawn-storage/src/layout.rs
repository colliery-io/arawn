use std::path::{Path, PathBuf};

use tracing::debug;

use crate::error::StorageError;

/// A declarative description of the expected directory tree.
/// This is the filesystem equivalent of database migrations — a versioned
/// schema for the directory structure that can evolve over time.
pub struct DataLayout {
    directories: Vec<PathBuf>,
}

impl DataLayout {
    /// The current layout version (V1).
    pub fn v1() -> Self {
        Self {
            directories: vec![
                PathBuf::from("workstreams"),
                PathBuf::from("plugins/tools"),
                PathBuf::from("plugins/build"),
                PathBuf::from("prompts"),
            ],
        }
    }

    /// Reconcile the actual directory tree against the declaration.
    /// Creates any missing directories. Idempotent — safe to call multiple times.
    pub fn ensure(&self, data_dir: &Path) -> Result<(), StorageError> {
        for dir in &self.directories {
            let full_path = data_dir.join(dir);
            if !full_path.exists() {
                debug!(path = ?full_path, "creating directory");
                std::fs::create_dir_all(&full_path)?;
            }
        }
        Ok(())
    }

    /// Return the list of declared directories (for testing/inspection).
    pub fn directories(&self) -> &[PathBuf] {
        &self.directories
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn ensure_creates_directories_on_fresh_dir() {
        let tmp = TempDir::new().unwrap();
        let layout = DataLayout::v1();

        layout.ensure(tmp.path()).unwrap();

        assert!(tmp.path().join("workstreams").is_dir());
        assert!(tmp.path().join("plugins/tools").is_dir());
        assert!(tmp.path().join("plugins/build").is_dir());
        assert!(tmp.path().join("prompts").is_dir());
    }

    #[test]
    fn ensure_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let layout = DataLayout::v1();

        layout.ensure(tmp.path()).unwrap();
        layout.ensure(tmp.path()).unwrap();

        assert!(tmp.path().join("workstreams").is_dir());
        assert!(tmp.path().join("plugins/tools").is_dir());
        assert!(tmp.path().join("plugins/build").is_dir());
        assert!(tmp.path().join("prompts").is_dir());
    }

    #[test]
    fn v1_declares_expected_directories() {
        let layout = DataLayout::v1();
        let dirs = layout.directories();
        assert_eq!(dirs.len(), 4);
        assert!(dirs.contains(&PathBuf::from("workstreams")));
        assert!(dirs.contains(&PathBuf::from("plugins/tools")));
        assert!(dirs.contains(&PathBuf::from("plugins/build")));
    }
}
