//! On-disk approval audit log.
//!
//! Every approval decision (Allow-once / Allow-for-session / Deny)
//! appends one JSON line to `<data_dir>/approval-audit.jsonl`. The
//! file is append-only and survives across sessions — but the
//! *allowlist* it implies does not: in T-0276 we only have
//! session-scoped grants. The log is the historical record of what
//! decisions were made, not a source of truth for future allows.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Tier the user picked at the prompt (or the system picked on their
/// behalf, e.g. when no prompter is wired).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalTier {
    AllowOnce,
    AllowForSession,
    Deny,
    /// The user had no prompter wired (non-TUI caller). Fails closed.
    FailedClosed,
}

impl ApprovalTier {
    pub fn as_str(self) -> &'static str {
        match self {
            ApprovalTier::AllowOnce => "allow_once",
            ApprovalTier::AllowForSession => "allow_for_session",
            ApprovalTier::Deny => "deny",
            ApprovalTier::FailedClosed => "failed_closed",
        }
    }
}

/// One row of the audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Unix epoch seconds.
    pub ts: u64,
    /// Optional session id — caller-supplied. `None` when the
    /// decision happens outside a session context.
    pub session_id: Option<String>,
    /// Tool the call was for.
    pub tool_name: String,
    /// Normalised arg shape (cf. `ArgShape::for_tool`).
    pub shape: String,
    /// The tier the user (or fallback) selected.
    pub tier: ApprovalTier,
    /// Free-form rationale — currently the prompt subtitle.
    pub reason: Option<String>,
}

/// Append-only on-disk audit log. Thread-safe.
///
/// `Disabled` is the no-op variant for tests and for callers that
/// did not configure a data dir. Every audit-recording call site
/// is unconditional in `approval/mod.rs`, so the Disabled flavour
/// keeps callers branch-free.
pub enum ApprovalAudit {
    Enabled {
        path: PathBuf,
        lock: Mutex<()>,
    },
    Disabled,
}

impl ApprovalAudit {
    /// Open / create the audit log at `<data_dir>/approval-audit.jsonl`.
    /// Returns `Disabled` if `data_dir` is None.
    pub fn open(data_dir: Option<&Path>) -> Self {
        let Some(dir) = data_dir else {
            return ApprovalAudit::Disabled;
        };
        // Best-effort: if the dir cannot be created, fall back to Disabled
        // with a tracing warning. The approval path must not become a
        // single point of failure.
        if let Err(e) = std::fs::create_dir_all(dir) {
            tracing::warn!(error = %e, dir = %dir.display(), "could not create approval-audit dir");
            return ApprovalAudit::Disabled;
        }
        let path = dir.join("approval-audit.jsonl");
        ApprovalAudit::Enabled {
            path,
            lock: Mutex::new(()),
        }
    }

    /// Append one record. Failures are logged but never bubble — the
    /// audit log is a best-effort observer.
    pub fn record(&self, record: AuditRecord) {
        match self {
            ApprovalAudit::Disabled => {}
            ApprovalAudit::Enabled { path, lock } => {
                let _g = lock.lock().unwrap_or_else(|e| e.into_inner());
                if let Err(e) = append_record(path, &record) {
                    tracing::warn!(error = %e, "approval audit append failed");
                }
            }
        }
    }

    /// Read the entire log back as a `Vec<AuditRecord>`. Used by
    /// tests and the doctor surface. Returns an empty Vec for
    /// Disabled or for a missing file.
    pub fn read_all(&self) -> Vec<AuditRecord> {
        let ApprovalAudit::Enabled { path, .. } = self else {
            return Vec::new();
        };
        let Ok(content) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }
}

fn append_record(path: &Path, record: &AuditRecord) -> std::io::Result<()> {
    let line = serde_json::to_string(record).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
    })?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

/// Build the current unix epoch seconds. Helper to keep the call
/// sites in `mod.rs` short.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn record(tier: ApprovalTier) -> AuditRecord {
        AuditRecord {
            ts: now_secs(),
            session_id: Some("sess".into()),
            tool_name: "shell".into(),
            shape: "shell:ls".into(),
            tier,
            reason: Some("ls test".into()),
        }
    }

    #[test]
    fn disabled_is_silent() {
        let audit = ApprovalAudit::Disabled;
        audit.record(record(ApprovalTier::AllowOnce));
        assert!(audit.read_all().is_empty());
    }

    #[test]
    fn enabled_round_trips_records() {
        let tmp = TempDir::new().unwrap();
        let audit = ApprovalAudit::open(Some(tmp.path()));
        audit.record(record(ApprovalTier::AllowOnce));
        audit.record(record(ApprovalTier::AllowForSession));
        audit.record(record(ApprovalTier::Deny));
        let rows = audit.read_all();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].tier, ApprovalTier::AllowOnce);
        assert_eq!(rows[1].tier, ApprovalTier::AllowForSession);
        assert_eq!(rows[2].tier, ApprovalTier::Deny);
    }

    #[test]
    fn append_creates_parent_dir() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("nested").join("audit");
        let audit = ApprovalAudit::open(Some(&nested));
        audit.record(record(ApprovalTier::AllowOnce));
        assert_eq!(audit.read_all().len(), 1);
    }

    #[test]
    fn audit_handles_missing_file() {
        let tmp = TempDir::new().unwrap();
        let audit = ApprovalAudit::open(Some(tmp.path()));
        // No writes — read_all should be empty, not panic.
        assert!(audit.read_all().is_empty());
    }
}
