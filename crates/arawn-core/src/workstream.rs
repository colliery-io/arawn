//! The `Workstream` scope abstraction.
//!
//! A workstream is "a thing you track" (a person, a project, a hobby,
//! an initiative). Each has its own knowledge base under
//! `~/.arawn/workstreams/<name>/memory.db` and a description that
//! feeds extractor prompts in Phase 4.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Reserved workstream slug — auto-created on first boot and undeletable.
pub const SCRATCH_NAME: &str = "scratch";

/// Validation for workstream slugs. ASCII only, no leading punctuation,
/// no spaces. Lowercase enforced so we don't end up with `name` + `Name` + `NAME`.
pub fn validate_name(name: &str) -> Result<(), WorkstreamNameError> {
    if name.is_empty() {
        return Err(WorkstreamNameError::Empty);
    }
    if name.len() > 64 {
        return Err(WorkstreamNameError::TooLong);
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(WorkstreamNameError::BadLeading);
    }
    for c in chars {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
            return Err(WorkstreamNameError::BadChar(c));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkstreamNameError {
    Empty,
    TooLong,
    BadLeading,
    BadChar(char),
}

impl std::fmt::Display for WorkstreamNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "workstream name cannot be empty"),
            Self::TooLong => write!(f, "workstream name exceeds 64 characters"),
            Self::BadLeading => {
                write!(f, "workstream name must start with a lowercase letter or digit")
            }
            Self::BadChar(c) => write!(
                f,
                "workstream name contains invalid character '{c}' (allowed: a-z, 0-9, '_', '-')"
            ),
        }
    }
}

impl std::error::Error for WorkstreamNameError {}

/// A workstream — the primary organizational unit.
#[derive(Debug, Clone)]
pub struct Workstream {
    /// Stable Uuid retained for session linkage compatibility. The
    /// addressing primitive in user-facing code is `name`.
    pub id: Uuid,
    /// Slug (`^[a-z0-9][a-z0-9_-]*$`, 1-64 chars). Primary key in the
    /// registry table; used in paths and slash commands. The
    /// constructor does NOT validate — the registry does. Keeps the
    /// in-memory type permissive for tests and one-off shells.
    pub name: String,
    /// Human label shown in `/workstream list`. Defaults to `name`
    /// when omitted.
    pub display_name: String,
    /// Free text that feeds extractor prompts (Phase 4). Empty until set.
    pub description: String,
    /// On-disk root for this workstream's data (KB lives at
    /// `<root_dir>/memory.db`).
    pub root_dir: PathBuf,
    /// Feed ids bound to this workstream. Stored as metadata; acted
    /// on by the extractor in Phase 4.
    pub bindings: Vec<String>,
    /// Soft-delete flag.
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Workstream {
    pub fn new(name: impl Into<String>, root_dir: impl Into<PathBuf>) -> Self {
        let now = Utc::now();
        let name = name.into();
        Self {
            id: Uuid::new_v4(),
            display_name: name.clone(),
            name,
            description: String::new(),
            root_dir: root_dir.into(),
            bindings: Vec::new(),
            archived: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create the default scratch workstream for ad-hoc sessions.
    pub fn scratch(root_dir: impl Into<PathBuf>) -> Self {
        Self::new(SCRATCH_NAME, root_dir)
    }

    pub fn is_scratch(&self) -> bool {
        self.name == SCRATCH_NAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workstream_creation_uses_name_as_display_by_default() {
        let ws = Workstream::new("home-maintenance", "/tmp/home-maint");
        assert_eq!(ws.name, "home-maintenance");
        assert_eq!(ws.display_name, "home-maintenance");
        assert!(!ws.archived);
        assert!(ws.bindings.is_empty());
        assert!(ws.description.is_empty());
    }

    #[test]
    fn scratch_workstream() {
        let ws = Workstream::scratch("/tmp/scratch");
        assert_eq!(ws.name, SCRATCH_NAME);
        assert!(ws.is_scratch());
    }

    #[test]
    fn workstream_ids_are_unique() {
        let ws1 = Workstream::new("a", "/tmp/a");
        let ws2 = Workstream::new("b", "/tmp/b");
        assert_ne!(ws1.id, ws2.id);
    }

    #[test]
    fn name_validation_accepts_valid_slugs() {
        for ok in ["pat", "auth-migration", "team_a", "1on1-pat", "a"] {
            assert!(validate_name(ok).is_ok(), "expected '{ok}' to validate");
        }
    }

    #[test]
    fn name_validation_rejects_invalid_slugs() {
        assert_eq!(validate_name(""), Err(WorkstreamNameError::Empty));
        assert_eq!(validate_name("-leading"), Err(WorkstreamNameError::BadLeading));
        assert_eq!(validate_name("UpperCase"), Err(WorkstreamNameError::BadLeading));
        assert_eq!(validate_name("with space"), Err(WorkstreamNameError::BadChar(' ')));
        assert_eq!(validate_name("with.dot"), Err(WorkstreamNameError::BadChar('.')));
        let too_long: String = "a".repeat(65);
        assert_eq!(validate_name(&too_long), Err(WorkstreamNameError::TooLong));
    }
}
