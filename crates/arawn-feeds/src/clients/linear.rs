//! Linear — what feeds need from Linear, plus the production adapter
//! over `arawn-integrations::linear`.
//!
//! One call: a page of the issues assigned to or created by the key's
//! user (`linear/my-issues`). Templates depend on [`LinearFeedClient`];
//! tests fake it externally.

use std::sync::Arc;

use arawn_integrations::IntegrationError;
use arawn_integrations::linear::LinearIntegration;
pub use arawn_integrations::linear::{Issue as LinearIssue, IssuePage as LinearIssuePage};
use async_trait::async_trait;

use crate::error::FeedError;

#[async_trait]
pub trait LinearFeedClient: Send + Sync {
    /// One page of the issues assigned to or created by the key's user,
    /// updated after `since` (RFC3339; `None` = no floor), from the page
    /// cursor `after`. Gives the user's id with the page, so a template
    /// can tell "assigned to me" from "created by me".
    async fn my_issues_page(
        &self,
        since: Option<&str>,
        after: Option<&str>,
    ) -> Result<(String, LinearIssuePage), FeedError>;
}

pub struct RealLinearClient {
    integration: Arc<LinearIntegration>,
}

impl RealLinearClient {
    pub fn new(integration: Arc<LinearIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl LinearFeedClient for RealLinearClient {
    async fn my_issues_page(
        &self,
        since: Option<&str>,
        after: Option<&str>,
    ) -> Result<(String, LinearIssuePage), FeedError> {
        let client = self.integration.client().map_err(integ_err)?;
        client.my_issues_page(since, after).await.map_err(integ_err)
    }
}

fn integ_err(e: IntegrationError) -> FeedError {
    match e {
        IntegrationError::RateLimited { retry_after } => FeedError::RateLimited { retry_after },
        // A revoked key, or no key saved: the user must run setup again.
        IntegrationError::Auth(_) | IntegrationError::NotConnected(_) => {
            FeedError::Auth(format!("{} — run: arawn setup linear", e.user_message()))
        }
        IntegrationError::Provider(msg) if msg.contains("arawn setup linear") => {
            FeedError::Auth(msg)
        }
        IntegrationError::Provider(msg) if msg.contains("decode reply") => FeedError::Schema(msg),
        other => FeedError::Provider(other.user_message()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn rate_limit_and_auth_keep_their_meaning() {
        let d = Some(Duration::from_secs(30));
        assert!(matches!(
            integ_err(IntegrationError::RateLimited { retry_after: d }),
            FeedError::RateLimited { retry_after } if retry_after == d
        ));
        assert!(matches!(
            integ_err(IntegrationError::NotConnected("linear".into())),
            FeedError::Auth(m) if m.contains("arawn setup linear")
        ));
        assert!(matches!(
            integ_err(IntegrationError::Provider(
                "linear is not set up — run: arawn setup linear".into()
            )),
            FeedError::Auth(_)
        ));
        assert!(matches!(
            integ_err(IntegrationError::Provider("linear: boom".into())),
            FeedError::Provider(_)
        ));
    }
}
