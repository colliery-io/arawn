//! GitHub App installation callback handler (I-0045 T-0317).
//!
//! GitHub Apps install via a public URL — `https://github.com/apps/
//! <slug>/installations/new` — which redirects to our locally-bound
//! callback after the user picks org + repos. The callback carries
//! `installation_id`, `setup_action`, and (if we asked for it) a
//! short-lived `code` for user-to-server auth. For read-only v1 we
//! only care about `installation_id`.
//!
//! This module composes [`arawn_auth::CallbackServer::listen_raw`]
//! (T-0317 added that method on the auth crate) with a CSRF check
//! and turns the raw callback into a typed [`GithubInstallOutcome`].

use arawn_auth::CallbackServer;
use rand::Rng;
use url::Url;

use crate::error::IntegrationError;
use crate::integration::ConnectContext;

/// Successful install: we now know which installation_id to use when
/// minting access tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct GithubInstallOutcome {
    pub installation_id: u64,
    pub setup_action: String,
}

/// Drive the GitHub App install flow end-to-end. The caller hands in
/// the app's public install URL (`https://github.com/apps/<slug>/
/// installations/new`); we tack on a `state` CSRF token, publish the
/// URL via the [`ConnectContext`], wait for the callback, and verify
/// the state.
pub async fn run_install_flow(
    app_install_url: Url,
    callback_path: &str,
    ctx: &dyn ConnectContext,
) -> Result<GithubInstallOutcome, IntegrationError> {
    let callback = CallbackServer::bind(callback_path).await?;
    let redirect_uri = callback.redirect_uri().clone();
    let csrf = random_csrf();

    // GitHub takes both `state` (CSRF) and the redirect target on the
    // install URL when the app's "Setup URL" is `<our redirect_uri>`
    // (configured in the App settings on github.com). We include
    // `state` here so we can verify the callback isn't a stray hit.
    let mut url = app_install_url;
    url.query_pairs_mut()
        .append_pair("state", &csrf)
        .append_pair("redirect_uri", redirect_uri.as_str());

    ctx.publish_auth_url(&url).await;
    ctx.publish_progress("waiting for GitHub App install (5 min)…")
        .await;

    let raw = callback.listen_raw().await?;

    // CSRF check. If GitHub didn't echo state, treat as failure.
    let returned_state = raw.params.get("state").cloned().unwrap_or_default();
    if returned_state != csrf {
        return Err(IntegrationError::Provider(format!(
            "CSRF state mismatch on GitHub install callback (expected {csrf}, got {returned_state})"
        )));
    }

    let installation_id = raw
        .params
        .get("installation_id")
        .ok_or_else(|| {
            IntegrationError::Provider(
                "GitHub install callback missing installation_id parameter".into(),
            )
        })?
        .parse::<u64>()
        .map_err(|e| {
            IntegrationError::Provider(format!("installation_id is not a u64: {e}"))
        })?;

    let setup_action = raw
        .params
        .get("setup_action")
        .cloned()
        .unwrap_or_else(|| "install".to_string());

    Ok(GithubInstallOutcome {
        installation_id,
        setup_action,
    })
}

fn random_csrf() -> String {
    let mut buf = [0u8; 24];
    rand::thread_rng().fill(&mut buf);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csrf_tokens_are_unique_and_url_safe() {
        let a = random_csrf();
        let b = random_csrf();
        assert_ne!(a, b);
        // URL_SAFE_NO_PAD contains only A-Z a-z 0-9 - _
        for c in a.chars() {
            assert!(
                c.is_ascii_alphanumeric() || c == '-' || c == '_',
                "unexpected char {c}"
            );
        }
    }
}
