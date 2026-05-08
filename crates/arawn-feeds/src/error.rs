//! Error type used by templates and the runtime.

use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    /// OAuth token expired/revoked, scope removed, or otherwise unable to
    /// authenticate against the upstream provider. Pauses the feed until
    /// the user reconnects.
    #[error("auth failed: {0}")]
    Auth(String),

    /// Upstream provider returned a rate-limit response. cloacina's retry
    /// policy will honor `retry_after` if provided.
    #[error("rate limited{}", retry_after.as_ref().map(|d| format!(" (retry after {}s)", d.as_secs())).unwrap_or_default())]
    RateLimited { retry_after: Option<Duration> },

    /// Disk write / read failed (permission, disk full, corrupt meta.json).
    #[error("storage error: {0}")]
    Storage(String),

    /// Provider response didn't match what we expected — likely an API
    /// schema change. Templates emit this instead of panicking on bad
    /// JSON so we can surface a helpful "the provider's API changed"
    /// signal to the user.
    #[error("schema mismatch: {0}")]
    Schema(String),

    /// Catch-all for other provider-side failures (network, 5xx, etc).
    #[error("provider error: {0}")]
    Provider(String),

    /// Template parameters didn't validate (missing required field,
    /// unknown channel id, etc). Surfaced at registration time so we
    /// don't schedule a feed that's guaranteed to fail.
    #[error("invalid template params: {0}")]
    InvalidParams(String),
}
