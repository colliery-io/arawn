use std::path::PathBuf;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A workstream — the primary organizational unit.
/// Partitions data at the filesystem level.
#[derive(Debug, Clone)]
pub struct Workstream {
    pub id: Uuid,
    pub name: String,
    pub root_dir: PathBuf,
    pub created_at: DateTime<Utc>,
}

impl Workstream {
    pub fn new(name: impl Into<String>, root_dir: impl Into<PathBuf>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            root_dir: root_dir.into(),
            created_at: Utc::now(),
        }
    }

    /// Create the default scratch workstream for ad-hoc sessions.
    pub fn scratch(root_dir: impl Into<PathBuf>) -> Self {
        Self::new("scratch", root_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workstream_creation() {
        let ws = Workstream::new("Home Maintenance", "/tmp/home-maint");
        assert_eq!(ws.name, "Home Maintenance");
        assert_eq!(ws.root_dir, PathBuf::from("/tmp/home-maint"));
    }

    #[test]
    fn scratch_workstream() {
        let ws = Workstream::scratch("/tmp/scratch");
        assert_eq!(ws.name, "scratch");
        assert_eq!(ws.root_dir, PathBuf::from("/tmp/scratch"));
    }

    #[test]
    fn workstream_ids_are_unique() {
        let ws1 = Workstream::new("a", "/tmp/a");
        let ws2 = Workstream::new("b", "/tmp/b");
        assert_ne!(ws1.id, ws2.id);
    }
}
