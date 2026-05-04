//! Slack client wiring. slack-morphism's `SlackHyperClient` constructs its
//! own native-roots HTTPS connector and we hand it the bot token at call
//! time. Slack tokens don't expire by default — no refresh adapter needed.

use std::sync::Arc;

use arawn_auth::Token;
use slack_morphism::prelude::{
    SlackApiToken, SlackApiTokenType, SlackApiTokenValue, SlackClient,
    SlackClientHyperConnector, SlackClientHyperHttpsConnector, SlackClientSession,
    SlackHyperClient,
};

/// Bundle the slack-morphism client + token a tool needs to make API calls.
/// Constructed per-tool-call by [`super::integration::SlackIntegration::context`].
pub struct SlackContext {
    pub client: Arc<SlackHyperClient>,
    pub token: SlackApiToken,
}

impl SlackContext {
    /// Convenience: open a slack-morphism session against the bundled token.
    /// Sessions are zero-cost wrappers around `(client, token)` references.
    pub fn session(&self) -> SlackClientSession<'_, SlackClientHyperHttpsConnector> {
        self.client.open_session(&self.token)
    }
}

/// Build a [`SlackContext`] from a persisted `arawn_auth::Token`. The
/// HyperClient is wrapped in an `Arc` so multiple tool calls can share it
/// without rebuilding the connector — but constructing a new one per call
/// is also fine; slack-morphism's connector is internally Arc'd.
pub fn build_slack_client(token: &Token) -> SlackContext {
    let connector = SlackClientHyperConnector::new()
        .expect("rustls native roots available for Slack client");
    let client = Arc::new(SlackClient::new(connector));
    let api_token = SlackApiToken::new(SlackApiTokenValue::new(token.access.clone()))
        .with_token_type(SlackApiTokenType::Bot);
    SlackContext { client, token: api_token }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rvstruct::ValueStruct;

    #[test]
    fn build_constructs_bot_token_from_access() {
        // rustls 0.23 with both ring and aws-lc-rs visible in the workspace
        // doesn't auto-pick a default crypto provider; install ring at test
        // start. Production code installs once at server startup via the
        // first integration to actually fire a TLS connection. Idempotent.
        let _ = rustls::crypto::ring::default_provider().install_default();

        let token = Token {
            access: "xoxb-test-token".into(),
            refresh: None,
            expires_at: Some(Utc::now() + chrono::Duration::days(365 * 10)),
            scope: Some("channels:read,chat:write".into()),
            token_type: "Bearer".into(),
        };
        let ctx = build_slack_client(&token);
        // The wrapped value should round-trip through the ValueStruct.
        assert_eq!(ctx.token.token_value.value(), "xoxb-test-token");
        assert_eq!(ctx.token.token_type, Some(SlackApiTokenType::Bot));
    }
}
