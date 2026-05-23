//! Google Calendar-specific Hub builder. Most plumbing is shared via
//! [`crate::google_common`].

use arawn_auth::OAuthProviderConfig;
use google_calendar3::CalendarHub as GoogleCalendarHub;

use crate::error::IntegrationError;
use crate::google_common::{ArawnGetToken, HttpsConnector, TokenStoreHandle, build_https_client};

use super::integration::SERVICE_NAME;

/// Concrete CalendarHub the integration exposes.
pub type CalendarHub = GoogleCalendarHub<HttpsConnector>;

/// Open the persisted Calendar token, build the hyper-util client + auth
/// adapter, and return a fully-wired Hub. Returns `NotConnected` if the
/// user hasn't run `/connect google_calendar` yet.
pub fn client_from_token_store(
    data_dir: std::path::PathBuf,
    oauth_config: OAuthProviderConfig,
) -> Result<CalendarHub, IntegrationError> {
    let store = TokenStoreHandle::new(data_dir, SERVICE_NAME);
    let token = store
        .load_token()?
        .ok_or_else(|| IntegrationError::NotConnected(SERVICE_NAME.to_string()))?;
    let auth = ArawnGetToken::new(token, oauth_config, store);
    Ok(GoogleCalendarHub::new(build_https_client(), auth))
}
