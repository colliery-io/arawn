//! GitHub feed templates (I-0045).
//!
//! All three templates share the disk layout:
//!
//! ```text
//! <feed_dir>/notifications/<id>.json   # raw GitHub Notification
//! <feed_dir>/issues_and_prs/<owner>__<repo>__<number>.json
//! <feed_dir>/review_queue/<owner>__<repo>__<number>.json
//! ```
//!
//! Each template advances a per-feed `latest_updated_iso` cursor so
//! the next tick uses `since=<iso>` rather than a full table scan.

pub mod issues_and_prs;
pub mod notifications;

pub use issues_and_prs::IssuesAndPrsTemplate;
pub use notifications::NotificationsTemplate;
