//! Gmail-specific client glue. Most plumbing is shared via
//! [`crate::google_common`]; this module just builds the typed `Gmail<C>` Hub.

use arawn_auth::OAuthProviderConfig;
use google_gmail1::Gmail;

use crate::error::IntegrationError;
use crate::google_common::{ArawnGetToken, HttpsConnector, TokenStoreHandle, build_https_client};

use super::integration::SERVICE_NAME;

/// Concrete Gmail Hub the integration exposes. Tools call methods on this.
pub type GmailHub = Gmail<HttpsConnector>;

/// Open the persisted Gmail token, build the hyper-util client + auth
/// adapter, and return a fully-wired Hub. Returns `NotConnected` if the
/// user hasn't run `/connect gmail` yet.
pub fn client_from_token_store(
    data_dir: std::path::PathBuf,
    oauth_config: OAuthProviderConfig,
) -> Result<GmailHub, IntegrationError> {
    let store = TokenStoreHandle::new(data_dir, SERVICE_NAME);
    let token = store
        .load_token()?
        .ok_or_else(|| IntegrationError::NotConnected(SERVICE_NAME.to_string()))?;
    let auth = ArawnGetToken::new(token, oauth_config, store);
    Ok(Gmail::new(build_https_client(), auth))
}
