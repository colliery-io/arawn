//! Google Drive-specific Hub builder. Most plumbing is shared via
//! [`crate::google_common`].

use arawn_auth::OAuthProviderConfig;
use google_drive3::DriveHub as GoogleDriveHub;

use crate::error::IntegrationError;
use crate::google_common::{ArawnGetToken, HttpsConnector, TokenStoreHandle, build_https_client};

use super::integration::SERVICE_NAME;

/// Concrete DriveHub the integration exposes.
pub type DriveHub = GoogleDriveHub<HttpsConnector>;

/// Open the persisted Drive token, build the hyper-util client + auth
/// adapter, and return a fully-wired Hub. Returns `NotConnected` if the
/// user hasn't run `/connect google_drive` yet.
pub fn client_from_token_store(
    data_dir: std::path::PathBuf,
    oauth_config: OAuthProviderConfig,
) -> Result<DriveHub, IntegrationError> {
    let store = TokenStoreHandle::new(data_dir, SERVICE_NAME);
    let token = store
        .load_token()?
        .ok_or_else(|| IntegrationError::NotConnected(SERVICE_NAME.to_string()))?;
    let auth = ArawnGetToken::new(token, oauth_config, store);
    Ok(GoogleDriveHub::new(build_https_client(), auth))
}
