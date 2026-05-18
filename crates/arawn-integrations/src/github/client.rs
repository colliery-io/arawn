//! GitHub App authenticated HTTP client (I-0045 T-0317).
//!
//! Two-step auth:
//!
//! 1. Sign a short-lived (≤10 min) RS256 JWT with the App's RSA
//!    private key. The JWT proves "I am app <app_id>".
//! 2. POST that JWT to
//!    `/app/installations/{installation_id}/access_tokens` to mint a
//!    1-hour installation-access-token (IAT). The IAT is what every
//!    real API call uses as a bearer token.
//!
//! [`GithubClient`] caches the IAT + expiry in-memory and re-mints
//! when within `IAT_REFRESH_LEAD` of expiry. The reqwest client
//! itself is the v3 REST API base — `https://api.github.com`.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::error::IntegrationError;

use super::integration::GithubAppConfig;

/// Mint a new IAT when the current one has fewer than this long left.
const IAT_REFRESH_LEAD: Duration = Duration::minutes(5);
/// JWT lifetime. GitHub caps at 10 minutes; pick something well under.
const JWT_LIFETIME: Duration = Duration::minutes(8);
/// GitHub REST API base.
pub const GITHUB_API_BASE: &str = "https://api.github.com";

/// Cached installation-access-token + its expiry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationAccessToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

impl InstallationAccessToken {
    pub fn is_fresh(&self) -> bool {
        self.expires_at - Utc::now() > IAT_REFRESH_LEAD
    }
}

#[derive(Debug, Deserialize)]
struct GithubTokenResponse {
    token: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct JwtClaims {
    iat: i64,
    exp: i64,
    iss: String,
}

/// Auto-refreshing GitHub App API client. Clone-friendly — the
/// cached token sits behind an `Arc<Mutex<...>>`.
#[derive(Clone)]
pub struct GithubClient {
    app: GithubAppConfig,
    installation_id: u64,
    cached: Arc<Mutex<Option<InstallationAccessToken>>>,
    http: Client,
}

impl GithubClient {
    pub fn new(app: GithubAppConfig, installation_id: u64) -> Result<Self, IntegrationError> {
        let http = Client::builder()
            .user_agent(concat!("arawn/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| IntegrationError::Provider(format!("http client: {e}")))?;
        Ok(Self {
            app,
            installation_id,
            cached: Arc::new(Mutex::new(None)),
            http,
        })
    }

    /// Read-only accessor used by tests + diagnostics. Reflects the
    /// in-memory cache; if it's stale, the next call to
    /// [`Self::access_token`] will mint a fresh one.
    pub async fn cached_token(&self) -> Option<InstallationAccessToken> {
        self.cached.lock().await.clone()
    }

    /// Get a current (or freshly-minted) installation-access-token.
    pub async fn access_token(&self) -> Result<InstallationAccessToken, IntegrationError> {
        {
            let guard = self.cached.lock().await;
            if let Some(ref t) = *guard
                && t.is_fresh()
            {
                return Ok(t.clone());
            }
        }
        let fresh = self.mint_installation_token().await?;
        *self.cached.lock().await = Some(fresh.clone());
        Ok(fresh)
    }

    /// Authenticated GET helper. Path is appended to
    /// [`GITHUB_API_BASE`]; the caller owns query construction.
    pub async fn get(&self, path: &str) -> Result<reqwest::Response, IntegrationError> {
        let token = self.access_token().await?;
        let url = format!("{GITHUB_API_BASE}{path}");
        self.http
            .get(&url)
            .bearer_auth(&token.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| IntegrationError::Provider(format!("github GET {path}: {e}")))
    }

    /// Cheap health probe — does the installation list any repos?
    /// Used by [`crate::Integration::is_connected`] indirectly via the
    /// integration's connection check.
    pub async fn health_check(&self) -> Result<(), IntegrationError> {
        let resp = self.get("/installation/repositories?per_page=1").await?;
        if !resp.status().is_success() {
            return Err(IntegrationError::Provider(format!(
                "health check failed: {}",
                resp.status()
            )));
        }
        Ok(())
    }

    /// Mint a fresh installation-access-token.
    async fn mint_installation_token(&self) -> Result<InstallationAccessToken, IntegrationError> {
        let jwt = sign_app_jwt(&self.app)?;
        let url = format!(
            "{GITHUB_API_BASE}/app/installations/{}/access_tokens",
            self.installation_id
        );
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&jwt)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| IntegrationError::Provider(format!("mint IAT: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(IntegrationError::Provider(format!(
                "mint IAT: status {status}, body {body}"
            )));
        }
        let parsed: GithubTokenResponse = resp
            .json()
            .await
            .map_err(|e| IntegrationError::Provider(format!("mint IAT parse: {e}")))?;
        Ok(InstallationAccessToken {
            token: parsed.token,
            expires_at: parsed.expires_at,
        })
    }
}

/// Sign an RS256 JWT proving "I am app <app_id>". Returns the
/// encoded JWT string. iss = app_id; iat = now; exp = now + JWT_LIFETIME.
pub fn sign_app_jwt(app: &GithubAppConfig) -> Result<String, IntegrationError> {
    let now = Utc::now();
    let iat = (now - Duration::seconds(60)).timestamp(); // 60s clock skew tolerance
    let exp = (now + JWT_LIFETIME).timestamp();
    let claims = JwtClaims {
        iat,
        exp,
        iss: app.app_id.clone(),
    };
    let header = Header::new(Algorithm::RS256);
    let key = EncodingKey::from_rsa_pem(app.private_key_pem.as_bytes())
        .map_err(|e| IntegrationError::Provider(format!("invalid RSA private key: {e}")))?;
    encode(&header, &claims, &key)
        .map_err(|e| IntegrationError::Provider(format!("JWT sign: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test RSA key — generated specifically for unit tests, never
    // used against real GitHub. 2048-bit RSA.
    const TEST_KEY: &str = "-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAxbtfWlbeYU1vIfHTwxKQmYBn9pcMq3LqBYCmzytH3EmIwjPL
0eYg9MawvchQGD5ws9MNkApcDXOIIBLImdvCK60lP6ToMSdF87+CkB3WvKKjC3sw
9Rs/9rmRMqq+T7CdYksLrPDB46HCipUDxa+5Gt32SUKnJI/T+iCQq1ldXKfeUUb1
8/o4SMt12LqiNX9bzlNzNwOM9CcdyltMixDeMy7+lXJN3GMOdoVvBJj2vWUF8jHs
MIIEpAIBAAKCAQEAxbtfWlbeYU1vIfHTwxKQmYBn9pcMq3LqBYCmzytH3EmIwjPL
PLACEHOLDER_NOT_A_REAL_KEY
-----END RSA PRIVATE KEY-----";

    #[test]
    fn cached_token_starts_empty() {
        let app = GithubAppConfig {
            app_id: "12345".into(),
            private_key_pem: TEST_KEY.into(),
            app_slug: "arawn-test".into(),
        };
        let client = GithubClient::new(app, 999).unwrap();
        // tokio's runtime-on-demand for a blocking-friendly assertion:
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            assert!(client.cached_token().await.is_none());
        });
    }

    #[test]
    fn fresh_token_window_respects_lead_time() {
        let t = InstallationAccessToken {
            token: "x".into(),
            expires_at: Utc::now() + Duration::minutes(10),
        };
        assert!(t.is_fresh(), "10 min away should be fresh");
        let near_expiry = InstallationAccessToken {
            token: "x".into(),
            expires_at: Utc::now() + Duration::minutes(2),
        };
        assert!(
            !near_expiry.is_fresh(),
            "2 min away should NOT be fresh (lead=5 min)"
        );
    }

    #[test]
    fn jwt_sign_rejects_garbage_key() {
        let app = GithubAppConfig {
            app_id: "12345".into(),
            private_key_pem: "this is not a PEM".into(),
            app_slug: "arawn-test".into(),
        };
        let err = sign_app_jwt(&app).unwrap_err();
        match err {
            IntegrationError::Provider(msg) => {
                assert!(msg.contains("RSA"), "unexpected error: {msg}");
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }
}
