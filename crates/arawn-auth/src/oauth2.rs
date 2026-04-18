//! Provider-agnostic OAuth2 + PKCE.
//!
//! Implements the parts of RFC 6749 + RFC 7636 we actually need:
//! authorization-code flow with PKCE (S256), token exchange, refresh.
//! Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.

use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use rand::Rng;
use rand::distributions::Alphanumeric;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use crate::error::AuthError;

/// Static configuration for an OAuth2 provider — not the user's credentials.
#[derive(Debug, Clone)]
pub struct OAuthProviderConfig {
    /// Authorization endpoint (e.g., `https://accounts.google.com/o/oauth2/v2/auth`).
    pub auth_url: Url,
    /// Token endpoint.
    pub token_url: Url,
    /// OAuth client ID (typically a public string).
    pub client_id: String,
    /// OAuth client secret. Some providers (Slack) require it for
    /// confidential clients; PKCE-only public clients can use an empty string.
    pub client_secret: String,
    /// Requested scopes.
    pub scopes: Vec<String>,
}

/// A user's OAuth credential — what `TokenStore` persists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub access: String,
    pub refresh: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scope: Option<String>,
    #[serde(default = "default_token_type")]
    pub token_type: String,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

impl Token {
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(exp) => Utc::now() >= exp,
            None => false,
        }
    }
}

/// What `OAuthClient::start_flow` hands back.
#[derive(Debug, Clone)]
pub struct AuthRequest {
    /// URL to open in the user's browser.
    pub authorization_url: Url,
    /// CSRF state — verify against the value returned in the callback.
    pub csrf_state: String,
    /// PKCE verifier — supply to `exchange_code`.
    pub pkce_verifier: String,
}

pub struct OAuthClient {
    config: OAuthProviderConfig,
    http: reqwest::Client,
}

impl OAuthClient {
    pub fn new(config: OAuthProviderConfig) -> Self {
        Self::with_http(
            config,
            reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("default reqwest client builds"),
        )
    }

    pub fn with_http(config: OAuthProviderConfig, http: reqwest::Client) -> Self {
        Self { config, http }
    }

    /// Generate a PKCE verifier + challenge + CSRF state and build the
    /// authorization URL the user's browser should hit.
    ///
    /// `redirect_uri` is the URL the callback server is listening on
    /// (`http://127.0.0.1:<port>/<path>`). Pass the same value to
    /// [`Self::exchange_code`].
    pub fn start_flow(&self, redirect_uri: &Url) -> AuthRequest {
        let pkce_verifier = generate_pkce_verifier();
        let pkce_challenge = pkce_challenge_s256(&pkce_verifier);
        let csrf_state = generate_state();

        let mut url = self.config.auth_url.clone();
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("response_type", "code");
            q.append_pair("client_id", &self.config.client_id);
            q.append_pair("redirect_uri", redirect_uri.as_str());
            q.append_pair("state", &csrf_state);
            q.append_pair("code_challenge", &pkce_challenge);
            q.append_pair("code_challenge_method", "S256");
            if !self.config.scopes.is_empty() {
                q.append_pair("scope", &self.config.scopes.join(" "));
            }
            // Google needs these to return a refresh token on subsequent grants.
            q.append_pair("access_type", "offline");
            q.append_pair("prompt", "consent");
        }

        AuthRequest {
            authorization_url: url,
            csrf_state,
            pkce_verifier,
        }
    }

    /// Exchange an authorization code for a [`Token`].
    pub async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &Url,
        pkce_verifier: &str,
    ) -> Result<Token, AuthError> {
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
            ("client_id", self.config.client_id.as_str()),
            ("code_verifier", pkce_verifier),
        ];
        if !self.config.client_secret.is_empty() {
            form.push(("client_secret", self.config.client_secret.as_str()));
        }
        self.post_token(&form).await
    }

    /// Use a refresh token to mint a new access token.
    pub async fn refresh(&self, refresh_token: &str) -> Result<Token, AuthError> {
        let mut form = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", self.config.client_id.as_str()),
        ];
        if !self.config.client_secret.is_empty() {
            form.push(("client_secret", self.config.client_secret.as_str()));
        }
        match self.post_token(&form).await {
            Ok(mut t) => {
                // Some providers (Google) don't return a refresh on refresh —
                // preserve the old one.
                if t.refresh.is_none() {
                    t.refresh = Some(refresh_token.to_string());
                }
                Ok(t)
            }
            Err(AuthError::ApiError { status, .. }) if status == 400 || status == 401 => {
                Err(AuthError::AuthExpired)
            }
            Err(e) => Err(e),
        }
    }

    async fn post_token(&self, form: &[(&str, &str)]) -> Result<Token, AuthError> {
        let resp = self
            .http
            .post(self.config.token_url.clone())
            .form(form)
            .send()
            .await
            .map_err(|e| AuthError::Network(e.to_string()))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| AuthError::Network(e.to_string()))?;

        if !status.is_success() {
            return Err(AuthError::ApiError { status: status.as_u16(), body: text });
        }

        let raw: TokenResponse = serde_json::from_str(&text)
            .map_err(|e| AuthError::Decode(format!("{e}: {text}")))?;

        let expires_at = raw
            .expires_in
            .map(|secs| Utc::now() + chrono::Duration::seconds(secs as i64));

        Ok(Token {
            access: raw.access_token,
            refresh: raw.refresh_token,
            expires_at,
            scope: raw.scope,
            token_type: raw.token_type.unwrap_or_else(default_token_type),
        })
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    token_type: Option<String>,
}

// ---------------------------------------------------------------------------
// PKCE helpers
// ---------------------------------------------------------------------------

/// 64-character URL-safe random string. RFC 7636 §4.1 allows 43–128 chars.
fn generate_pkce_verifier() -> String {
    let mut rng = rand::thread_rng();
    (&mut rng)
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

fn pkce_challenge_s256(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    (&mut rng)
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_rfc_7636_example() {
        // RFC 7636 Appendix B test vector
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert_eq!(pkce_challenge_s256(verifier), expected);
    }

    #[test]
    fn pkce_verifier_length() {
        let v = generate_pkce_verifier();
        assert_eq!(v.len(), 64);
        assert!(v.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn state_length() {
        let s = generate_state();
        assert_eq!(s.len(), 32);
    }

    #[test]
    fn start_flow_includes_required_params() {
        let cfg = OAuthProviderConfig {
            auth_url: "https://example.com/auth".parse().unwrap(),
            token_url: "https://example.com/token".parse().unwrap(),
            client_id: "client-xyz".into(),
            client_secret: "".into(),
            scopes: vec!["read".into(), "write".into()],
        };
        let client = OAuthClient::new(cfg);
        let redirect: Url = "http://127.0.0.1:1234/callback".parse().unwrap();
        let req = client.start_flow(&redirect);

        let url = req.authorization_url.as_str();
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=client-xyz"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A1234%2Fcallback"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains(&format!("state={}", req.csrf_state)));
        assert!(url.contains("scope=read+write"));
    }

    /// Tiny in-process HTTP stub for the OAuth token endpoint.
    /// Returns `(server_url, completion_handle)`. The handle resolves once
    /// the stub has handled one request.
    async fn spawn_token_stub(
        status: u16,
        body: &'static str,
    ) -> (Url, tokio::task::JoinHandle<Vec<u8>>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url: Url = format!("http://127.0.0.1:{port}/token").parse().unwrap();

        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let mut filled = 0;
            loop {
                let n = stream.read(&mut buf[filled..]).await.unwrap();
                if n == 0 { break; }
                filled += n;
                if buf[..filled].windows(4).any(|w| w == b"\r\n\r\n") { break; }
            }
            let mut more = vec![0u8; 4096];
            let _ = tokio::time::timeout(
                std::time::Duration::from_millis(50),
                stream.read(&mut more),
            )
            .await;

            let status_text = match status {
                200 => "200 OK",
                400 => "400 Bad Request",
                401 => "401 Unauthorized",
                _ => "500 Internal Server Error",
            };
            let response = format!(
                "HTTP/1.1 {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.shutdown().await.ok();
            buf[..filled].to_vec()
        });

        (url, handle)
    }

    fn client_with_token_url(token_url: Url) -> OAuthClient {
        OAuthClient::new(OAuthProviderConfig {
            auth_url: "https://example.com/auth".parse().unwrap(),
            token_url,
            client_id: "cid".into(),
            client_secret: "secret".into(),
            scopes: vec!["read".into()],
        })
    }

    #[tokio::test]
    async fn exchange_code_decodes_token_response() {
        let body = r#"{"access_token":"AT","refresh_token":"RT","expires_in":3600,"scope":"read","token_type":"Bearer"}"#;
        let (url, handle) = spawn_token_stub(200, body).await;
        let client = client_with_token_url(url);
        let redirect: Url = "http://127.0.0.1:1/cb".parse().unwrap();

        let token = client.exchange_code("the-code", &redirect, "verifier").await.unwrap();
        assert_eq!(token.access, "AT");
        assert_eq!(token.refresh.as_deref(), Some("RT"));
        assert!(token.expires_at.is_some() && !token.is_expired());

        let request = String::from_utf8(handle.await.unwrap()).unwrap();
        assert!(request.contains("grant_type=authorization_code"));
        assert!(request.contains("code=the-code"));
        assert!(request.contains("code_verifier=verifier"));
        assert!(request.contains("client_secret=secret"));
    }

    #[tokio::test]
    async fn refresh_failure_with_400_returns_auth_expired() {
        let body = r#"{"error":"invalid_grant"}"#;
        let (url, handle) = spawn_token_stub(400, body).await;
        let client = client_with_token_url(url);
        let err = client.refresh("rt-old").await.unwrap_err();
        assert!(matches!(err, AuthError::AuthExpired));
        let _ = handle.await;
    }

    #[tokio::test]
    async fn refresh_preserves_refresh_token_when_provider_omits_it() {
        let body = r#"{"access_token":"NEW","expires_in":3600,"token_type":"Bearer"}"#;
        let (url, handle) = spawn_token_stub(200, body).await;
        let client = client_with_token_url(url);
        let token = client.refresh("rt-keep").await.unwrap();
        assert_eq!(token.access, "NEW");
        assert_eq!(token.refresh.as_deref(), Some("rt-keep"));
        let _ = handle.await;
    }

    #[test]
    fn token_is_expired_respects_expiration_time() {
        let past = Token {
            access: "x".into(),
            refresh: None,
            expires_at: Some(Utc::now() - chrono::Duration::seconds(60)),
            scope: None,
            token_type: "Bearer".into(),
        };
        assert!(past.is_expired());

        let future = Token {
            access: "x".into(),
            refresh: None,
            expires_at: Some(Utc::now() + chrono::Duration::seconds(3600)),
            scope: None,
            token_type: "Bearer".into(),
        };
        assert!(!future.is_expired());

        let no_expiry = Token {
            access: "x".into(),
            refresh: None,
            expires_at: None,
            scope: None,
            token_type: "Bearer".into(),
        };
        assert!(!no_expiry.is_expired());
    }
}
