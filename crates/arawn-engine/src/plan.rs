//! Plan mode — observation-only planning with approval.
//!
//! When plan mode is active, only side-effect-free tools are allowed. The agent
//! researches and writes a plan to disk. On exit, the plan is presented for user
//! approval before any actions are taken.
//!
//! Plans are stored in the session's working directory (not centralized),
//! keeping them contextual to the work being done.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::permissions::PermissionMode;
use crate::permissions::PermissionRule;

/// State for plan mode within a session.
#[derive(Debug)]
pub struct PlanModeState {
    inner: RwLock<PlanModeInner>,
}

#[derive(Debug)]
struct PlanModeInner {
    active: bool,
    /// The permission mode that was active before entering plan mode.
    pre_plan_mode: Option<PermissionMode>,
    /// Permission rules that were stripped on entering plan mode (for restoration).
    /// Reserved for future use when auto-mode rule stripping is implemented.
    #[allow(dead_code)]
    stripped_rules: Vec<PermissionRule>,
    /// Path to the current plan file.
    plan_file: Option<PathBuf>,
    /// Human-friendly slug for the plan, cached per session.
    plan_slug: Option<String>,
}

/// Snapshot of plan mode state for tools to read without holding a lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanModeSnapshot {
    pub active: bool,
    pub plan_file: Option<PathBuf>,
    pub plan_slug: Option<String>,
}

impl PlanModeState {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(PlanModeInner {
                active: false,
                pre_plan_mode: None,
                stripped_rules: Vec::new(),
                plan_file: None,
                plan_slug: None,
            }),
        }
    }

    /// Whether plan mode is currently active.
    pub fn is_active(&self) -> bool {
        self.inner.read().unwrap().active
    }

    /// Get a snapshot of the current state.
    pub fn snapshot(&self) -> PlanModeSnapshot {
        let inner = self.inner.read().unwrap();
        PlanModeSnapshot {
            active: inner.active,
            plan_file: inner.plan_file.clone(),
            plan_slug: inner.plan_slug.clone(),
        }
    }

    /// Enter plan mode. The plan file is created in `working_dir` — the session's
    /// workspace — keeping it contextual to the work being done.
    /// Returns the path to the plan file.
    pub fn enter(
        &self,
        current_mode: PermissionMode,
        slug: &str,
        working_dir: &Path,
    ) -> std::io::Result<PathBuf> {
        let plan_file = working_dir.join(format!("{slug}.plan.md"));

        // Create the plan file if it doesn't exist
        if !plan_file.exists() {
            std::fs::write(&plan_file, format!("# Plan: {slug}\n\n"))?;
        }

        let mut inner = self.inner.write().unwrap();
        inner.active = true;
        inner.pre_plan_mode = Some(current_mode);
        inner.plan_file = Some(plan_file.clone());
        inner.plan_slug = Some(slug.to_string());

        info!(slug, ?plan_file, "entered plan mode");
        Ok(plan_file)
    }

    /// Exit plan mode. Returns the pre-plan permission mode for restoration.
    pub fn exit(&self) -> Option<PermissionMode> {
        let mut inner = self.inner.write().unwrap();
        if !inner.active {
            return None;
        }
        inner.active = false;
        let pre_mode = inner.pre_plan_mode.take();
        info!(?pre_mode, "exited plan mode");
        pre_mode
    }

    /// Get the current plan file path (if in plan mode).
    pub fn plan_file(&self) -> Option<PathBuf> {
        self.inner.read().unwrap().plan_file.clone()
    }

    /// Read the current plan content from disk.
    pub fn read_plan(&self) -> Option<String> {
        let plan_file = self.plan_file()?;
        std::fs::read_to_string(&plan_file).ok()
    }

    /// Write plan content to disk.
    pub fn write_plan(&self, content: &str) -> std::io::Result<()> {
        let inner = self.inner.read().unwrap();
        if let Some(ref plan_file) = inner.plan_file {
            std::fs::write(plan_file, content)?;
            debug!(?plan_file, bytes = content.len(), "wrote plan to disk");
        }
        Ok(())
    }

    /// Check if a given file path is the current plan file (for write exceptions).
    pub fn is_plan_file(&self, path: &Path) -> bool {
        let inner = self.inner.read().unwrap();
        inner
            .plan_file
            .as_ref()
            .map(|pf| pf == path)
            .unwrap_or(false)
    }
}

impl Default for PlanModeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a human-friendly slug from a task description.
/// Picks up to 4 lowercase words, joined by hyphens.
pub fn generate_slug(description: &str) -> String {
    let stop_words: &[&str] = &[
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
        "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "shall",
        "can", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into",
        "through", "during", "before", "after", "above", "below", "between", "and", "but", "or",
        "nor", "not", "so", "yet", "both", "either", "neither", "each", "every", "all", "any",
        "few", "more", "most", "other", "some", "such", "no", "only", "own", "same", "than",
        "too", "very", "just", "that", "this", "these", "those", "i", "me", "my", "we", "our",
        "you", "your", "it", "its", "they", "them", "their",
    ];

    let words: Vec<String> = description
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|w| !w.is_empty() && !stop_words.contains(&w.as_str()))
        .take(4)
        .collect();

    if words.is_empty() {
        "plan".to_string()
    } else {
        words.join("-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn generate_slug_basic() {
        assert_eq!(generate_slug("refactor auth middleware"), "refactor-auth-middleware");
    }

    #[test]
    fn generate_slug_strips_stop_words() {
        assert_eq!(
            generate_slug("plan for the database migration strategy"),
            "plan-database-migration-strategy"
        );
    }

    #[test]
    fn generate_slug_max_four_words() {
        assert_eq!(
            generate_slug("implement new caching layer distributed system redesign"),
            "implement-new-caching-layer"
        );
    }

    #[test]
    fn generate_slug_empty() {
        assert_eq!(generate_slug(""), "plan");
        assert_eq!(generate_slug("the a an"), "plan");
    }

    #[test]
    fn generate_slug_special_chars() {
        assert_eq!(generate_slug("fix bug #123 (urgent)"), "fix-bug-123-urgent");
    }

    #[test]
    fn plan_mode_lifecycle() {
        let tmp = TempDir::new().unwrap();
        let state = PlanModeState::new();

        assert!(!state.is_active());

        // Enter plan mode — plan file in session working dir
        let plan_file = state.enter(PermissionMode::Default, "test-plan", tmp.path()).unwrap();
        assert!(state.is_active());
        assert!(plan_file.exists());
        assert_eq!(plan_file, tmp.path().join("test-plan.plan.md"));

        // Write and read plan
        state.write_plan("# My Plan\n\nStep 1: Do stuff").unwrap();
        let content = state.read_plan().unwrap();
        assert!(content.contains("Step 1"));

        // Check plan file detection
        assert!(state.is_plan_file(&plan_file));
        assert!(!state.is_plan_file(Path::new("/some/other/file.md")));

        // Exit plan mode
        let pre_mode = state.exit();
        assert_eq!(pre_mode, Some(PermissionMode::Default));
        assert!(!state.is_active());
    }

    #[test]
    fn exit_when_not_active_returns_none() {
        let state = PlanModeState::new();
        assert_eq!(state.exit(), None);
    }

    #[test]
    fn snapshot_reflects_state() {
        let tmp = TempDir::new().unwrap();
        let state = PlanModeState::new();

        let snap = state.snapshot();
        assert!(!snap.active);
        assert!(snap.plan_file.is_none());

        state.enter(PermissionMode::AcceptEdits, "snap-test", tmp.path()).unwrap();
        let snap = state.snapshot();
        assert!(snap.active);
        assert_eq!(snap.plan_slug.as_deref(), Some("snap-test"));
    }
}
