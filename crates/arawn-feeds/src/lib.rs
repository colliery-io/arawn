//! Continual data feeds — opinionated, configurable, local-first
//! ingestion across personal + watched spaces.
//!
//! See `.metis/initiatives/ARAWN-I-0039/initiative.md` for the design.
//!
//! Three core concepts:
//!
//! - **Template** — a named, parameterized fetch+write recipe owned by
//!   an integration. Pure Rust trait impls, one file per template.
//!   Templates own their on-disk storage layout under their feed dir;
//!   only `meta.json` is reserved for the runtime.
//!
//! - **Feed** — a configured instance of a template (which provider,
//!   which template, what params, what cadence). Persisted in a
//!   `feeds` table in arawn.db; cloacina cron schedules are derived
//!   from the table at server boot + on `/watch`.
//!
//! - **Run** — one execution. Templates are not cloacina pipelines —
//!   each feed registers as a thin `Runtime::register_workflow` over
//!   a single generic `FeedDispatchTask`, and a per-feed
//!   `register_cron_workflow` schedule. cloacina handles catchup,
//!   retry, audit, single-instance enforcement.

pub mod cadence;
pub mod clients;
pub mod dispatch;
pub mod error;
pub mod layout;
pub mod meta;
pub mod registry;
pub mod runtime;
pub mod store;
pub mod template;
pub mod templates;
pub mod types;

pub use cadence::{MIN_CADENCE, validate_cadence};
pub use clients::{
    CalendarFeedClient, ChannelKind, FeedClients, NoopClients, RealCalendarClient, RealClients,
    RealSlackClient, SlackAuthInfo, SlackFeedClient, SlackHistoryPage, classify_channel_id,
};
pub use dispatch::{FeedDispatchTask, FeedRuntimeContext, run_feed};
pub use runtime::{CloacinaRunner, FeedRuntime, feed_workflow_name, start};
pub use error::FeedError;
pub use layout::DataLayout;
pub use meta::MetaStore;
pub use registry::FeedTemplateRegistry;
pub use store::{FeedRecord, FeedStore, new_record};
pub use template::{FeedTemplate, RunOutcome, TemplateCtx};
pub use templates::default_registry;
pub use types::{FeedDefaults, FeedMeta, RunSummary, TemplateParams};
