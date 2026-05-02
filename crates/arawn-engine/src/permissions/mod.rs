//! Permission system — allow/deny/ask rules with tool-level access control.
//!
//! The permission system sits between the engine and tool execution, evaluating
//! rules against each tool call before it runs. Rules support tool name matching
//! (exact or glob) with optional content patterns.

mod checker;
mod config;
mod prompt;
mod rules;

pub use checker::{
    AuditEntry, DecisionReason, ModalOption, ModalPrompt, ModalRequest,
    PermissionChecker, PermissionMode, PermissionResponse, PermissionSnapshot,
    SessionGrants, SharedAudit, new_shared_audit,
};
pub use config::{PermissionConfig, load_merged_permissions, load_permissions_from_file};
pub use prompt::{CliModalPrompt, MockModalPrompt};
pub use rules::{PermissionDecision, PermissionRule, RuleKind, RuleMatcher};
