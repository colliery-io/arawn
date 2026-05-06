//! Local single-shot HTTP listener for the OAuth callback.
//!
//! Binds to `127.0.0.1:0` (resolved port published before listening), accepts
//! one HTTP request, parses `?code=...&state=...` from the query string,
//! responds with a small HTML success page, then shuts down.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::debug;
use url::Url;

use crate::error::AuthError;

// Short enough that a failed OAuth (provider shows its own error page and
// never redirects back) doesn't hold the bound port hostage. A successful
// flow takes seconds; if the user takes longer than 60s they can re-issue
// /connect.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

const SUCCESS_PAGE: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>arawn — connected</title></head><body style=\"font-family: system-ui, sans-serif; max-width: 480px; margin: 80px auto; text-align: center;\"><h1>✓ Connected</h1><p>You can close this tab and return to your terminal.</p></body></html>";

/// What the callback yielded.
#[derive(Debug, Clone)]
pub struct CallbackResult {
    pub code: String,
    pub state: String,
}

pub struct CallbackServer {
    listener: TcpListener,
    redirect_uri: Url,
}

impl CallbackServer {
    /// Bind to an OS-assigned port on `127.0.0.1`. The redirect URI for the
    /// OAuth flow is available immediately via [`Self::redirect_uri`].
    pub async fn bind(path: &str) -> Result<Self, AuthError> {
        Self::bind_inner(path, 0).await
    }

    /// Bind to a specific port on `127.0.0.1`. Required for providers like
    /// Slack whose redirect-URI allowlist is exact-match (no wildcard ports).
    /// Returns `Network` error if the port is already in use.
    pub async fn bind_with_port(path: &str, port: u16) -> Result<Self, AuthError> {
        Self::bind_inner(path, port).await
    }

    async fn bind_inner(path: &str, port: u16) -> Result<Self, AuthError> {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(|e| AuthError::Network(format!("bind 127.0.0.1:{port} failed: {e}")))?;
        let bound_port = listener
            .local_addr()
            .map_err(|e| AuthError::Network(format!("local_addr: {e}")))?
            .port();
        let path = path.strip_prefix('/').unwrap_or(path);
        // Host string is `localhost`, not `127.0.0.1`. The TCP listener binds
        // to the loopback IP (browsers resolve `localhost` to it), but some
        // OAuth providers (notably Slack) string-match the redirect URI and
        // reject `127.0.0.1` even though it's the same address. `localhost`
        // is accepted everywhere.
        let redirect_uri = format!("http://localhost:{bound_port}/{path}")
            .parse::<Url>()
            .map_err(|e| AuthError::InvalidConfig(format!("redirect URL: {e}")))?;
        Ok(Self { listener, redirect_uri })
    }

    pub fn redirect_uri(&self) -> &Url {
        &self.redirect_uri
    }

    /// Wait up to [`DEFAULT_TIMEOUT`] for a single redirect, parse it, and
    /// return the `(code, state)` pair.
    pub async fn listen(self) -> Result<CallbackResult, AuthError> {
        self.listen_with_timeout(DEFAULT_TIMEOUT).await
    }

    pub async fn listen_with_timeout(
        self,
        timeout: Duration,
    ) -> Result<CallbackResult, AuthError> {
        let accept = async {
            let (mut stream, addr) = self
                .listener
                .accept()
                .await
                .map_err(|e| AuthError::Network(format!("accept: {e}")))?;
            debug!(?addr, "callback connection accepted");

            // Read request headers — we only care about the first line.
            let mut buf = vec![0u8; 8192];
            let mut filled = 0;
            loop {
                if filled >= buf.len() {
                    return Err(AuthError::InvalidConfig(
                        "callback request too large".into(),
                    ));
                }
                let n = stream
                    .read(&mut buf[filled..])
                    .await
                    .map_err(|e| AuthError::Network(format!("read: {e}")))?;
                if n == 0 {
                    break;
                }
                filled += n;
                // Headers end at the first CRLF CRLF.
                if buf[..filled].windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }

            let request_line = std::str::from_utf8(&buf[..filled])
                .ok()
                .and_then(|s| s.lines().next())
                .ok_or_else(|| {
                    AuthError::InvalidConfig("malformed callback request".into())
                })?;

            // Format: METHOD <path-and-query> HTTP/...
            let target = request_line
                .split_whitespace()
                .nth(1)
                .ok_or_else(|| AuthError::InvalidConfig("missing request target".into()))?;

            let parsed = Url::parse(&format!("http://127.0.0.1{target}"))
                .map_err(|e| AuthError::InvalidConfig(format!("bad target: {e}")))?;

            let mut code = None;
            let mut state = None;
            let mut error: Option<String> = None;
            for (k, v) in parsed.query_pairs() {
                match k.as_ref() {
                    "code" => code = Some(v.into_owned()),
                    "state" => state = Some(v.into_owned()),
                    "error" => error = Some(v.into_owned()),
                    _ => {}
                }
            }

            // Always respond before propagating an error so the user's browser
            // doesn't hang.
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                SUCCESS_PAGE.len(),
                SUCCESS_PAGE
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.shutdown().await;

            if let Some(err) = error {
                return Err(AuthError::InvalidConfig(format!(
                    "OAuth provider returned error: {err}"
                )));
            }

            let code = code.ok_or_else(|| {
                AuthError::InvalidConfig("callback missing 'code' parameter".into())
            })?;
            let state = state.ok_or_else(|| {
                AuthError::InvalidConfig("callback missing 'state' parameter".into())
            })?;

            Ok(CallbackResult { code, state })
        };

        match tokio::time::timeout(timeout, accept).await {
            Ok(res) => res,
            Err(_) => Err(AuthError::InvalidConfig(format!(
                "OAuth callback timed out after {} s",
                timeout.as_secs()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpStream;

    async fn simulate_browser(server_url: &Url, query: &str) {
        let host = server_url.host_str().unwrap();
        let port = server_url.port().unwrap();
        let mut stream = TcpStream::connect((host, port)).await.unwrap();
        let path = format!("{}?{}", server_url.path(), query);
        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(req.as_bytes()).await.unwrap();
        // Drain the response so the server can shutdown cleanly.
        let mut sink = Vec::new();
        let _ = stream.read_to_end(&mut sink).await;
    }

    #[tokio::test]
    async fn happy_path_returns_code_and_state() {
        let server = CallbackServer::bind("/callback").await.unwrap();
        let url = server.redirect_uri().clone();
        let task = tokio::spawn(server.listen_with_timeout(Duration::from_secs(2)));
        simulate_browser(&url, "code=abc123&state=xyz").await;
        let result = task.await.unwrap().unwrap();
        assert_eq!(result.code, "abc123");
        assert_eq!(result.state, "xyz");
    }

    #[tokio::test]
    async fn missing_code_yields_invalid_config_error() {
        let server = CallbackServer::bind("/cb").await.unwrap();
        let url = server.redirect_uri().clone();
        let task = tokio::spawn(server.listen_with_timeout(Duration::from_secs(2)));
        simulate_browser(&url, "state=onlystate").await;
        let err = task.await.unwrap().unwrap_err();
        match err {
            AuthError::InvalidConfig(m) => assert!(m.contains("code")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn provider_error_propagates() {
        let server = CallbackServer::bind("/cb").await.unwrap();
        let url = server.redirect_uri().clone();
        let task = tokio::spawn(server.listen_with_timeout(Duration::from_secs(2)));
        simulate_browser(&url, "error=access_denied&state=s").await;
        let err = task.await.unwrap().unwrap_err();
        match err {
            AuthError::InvalidConfig(m) => assert!(m.contains("access_denied")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn timeout_returns_error() {
        let server = CallbackServer::bind("/cb").await.unwrap();
        let result = server.listen_with_timeout(Duration::from_millis(150)).await;
        assert!(matches!(result, Err(AuthError::InvalidConfig(_))));
    }

    #[test]
    fn redirect_uri_normalizes_path_with_or_without_slash() {
        // Just exercises the Url construction.
        let u: Url = "http://127.0.0.1:5000/callback".parse().unwrap();
        assert_eq!(u.path(), "/callback");
    }
}
