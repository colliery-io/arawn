//! Interactive approval workflow for sensitive tool calls.
//!
//! This module composes with `permissions/`. The split:
//! - **Permissions decide *whether* to ask.** Rules + permission
//!   mode produce one of `Allowed | Denied | Ask`. The first two
//!   short-circuit; only `Ask` reaches this module.
//! - **Approval handles the interaction.** When the user answers
//!   "Allow Once" the call proceeds for this one invocation. "Allow
//!   For Session" populates the [`allowlist::SessionAllowlist`]
//!   keyed by `(tool_name, ArgShape)`. "Deny" blocks the call.
//!
//! The on-disk audit log (`audit.rs`) was ripped — it was wired into
//! `PermissionChecker::with_approval_audit` but the setter was never
//! called with `Some(...)` in production. The in-memory audit on
//! `PermissionChecker` (`SharedAudit`/`AuditEntry`) backs the
//! `/permissions` UI; that's what stayed.

pub mod allowlist;

pub use allowlist::{ArgShape, SessionAllowlist, target_parent_dir};
