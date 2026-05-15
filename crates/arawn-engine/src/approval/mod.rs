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
//!   Every decision appends to the on-disk audit log.
//!
//! Out of scope (per T-0276): cross-session allowlists. The audit
//! log persists, but the *allowlist* lives in memory and is cleared
//! at session boundaries.

pub mod allowlist;
pub mod audit;

pub use allowlist::{ArgShape, SessionAllowlist};
pub use audit::{ApprovalAudit, ApprovalTier, AuditRecord, now_secs};
