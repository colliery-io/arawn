//! Thin GraphQL client over `https://api.linear.app/graphql`.
//!
//! No schema codegen and no GraphQL crate: each operation is a named
//! document in [`super::queries`], posted with its variables. The HTTP
//! hop sits behind [`LinearTransport`] so tests can answer with canned
//! bodies and inspect what was sent.
//!
//! Error mapping (verified 2026-10-07 against linear.app/developers):
//! - A spent rate limit is **HTTP 400** with a GraphQL error whose
//!   `extensions.code` is `RATELIMITED`. The wait comes from
//!   `X-RateLimit-Requests-Reset` (epoch milliseconds).
//! - A bad or revoked key is HTTP 401 and/or `AUTHENTICATION_ERROR`.
//! - Any other entry in `errors` is a provider error naming the messages.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::error::IntegrationError;

use super::queries;

/// The one Linear API endpoint.
pub const LINEAR_API_URL: &str = "https://api.linear.app/graphql";

/// One HTTP reply, reduced to what the error mapping needs.
#[derive(Debug, Clone, Default)]
pub struct GraphqlReply {
    pub status: u16,
    /// `X-RateLimit-Requests-Reset`, else `X-RateLimit-Complexity-Reset`:
    /// when the spent limit resets, as epoch milliseconds.
    pub rate_limit_reset_ms: Option<i64>,
    pub body: String,
}

/// The HTTP hop. [`HttpTransport`] in production; tests use a fake.
#[async_trait]
pub trait LinearTransport: Send + Sync {
    /// POST a GraphQL request body (`{query, variables}`) with the key.
    async fn post(&self, api_key: &str, body: &Value) -> Result<GraphqlReply, IntegrationError>;
}

/// `reqwest` transport to [`LINEAR_API_URL`].
#[derive(Debug, Clone, Default)]
pub struct HttpTransport {
    http: reqwest::Client,
}

impl HttpTransport {
    /// Build the request. A personal key goes in `Authorization` as is:
    /// Linear rejects `Bearer <personal key>`.
    fn request(&self, api_key: &str, body: &Value) -> Result<reqwest::Request, IntegrationError> {
        self.http
            .post(LINEAR_API_URL)
            .header(reqwest::header::AUTHORIZATION, api_key)
            .header(reqwest::header::USER_AGENT, "arawn/0.1")
            .json(body)
            .build()
            .map_err(|e| IntegrationError::Provider(format!("linear request: {e}")))
    }
}

#[async_trait]
impl LinearTransport for HttpTransport {
    async fn post(&self, api_key: &str, body: &Value) -> Result<GraphqlReply, IntegrationError> {
        let req = self.request(api_key, body)?;
        let resp = self
            .http
            .execute(req)
            .await
            .map_err(|e| IntegrationError::Provider(format!("network: {e}")))?;
        let header_ms = |name: &str| {
            resp.headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<i64>().ok())
        };
        let rate_limit_reset_ms = header_ms("x-ratelimit-requests-reset")
            .or_else(|| header_ms("x-ratelimit-complexity-reset"));
        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| IntegrationError::Provider(format!("read body: {e}")))?;
        Ok(GraphqlReply {
            status,
            rate_limit_reset_ms,
            body,
        })
    }
}

/// The key's user and workspace, from [`queries::VIEWER`].
#[derive(Debug, Clone, Deserialize)]
pub struct Viewer {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    pub organization: Organization,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Organization {
    pub name: String,
}

/// Linear GraphQL client for one API key.
#[derive(Clone)]
pub struct LinearClient {
    api_key: String,
    transport: Arc<dyn LinearTransport>,
}

impl LinearClient {
    /// Client over the real HTTP transport.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_transport(api_key, Arc::new(HttpTransport::default()))
    }

    pub fn with_transport(api_key: impl Into<String>, transport: Arc<dyn LinearTransport>) -> Self {
        Self {
            api_key: api_key.into(),
            transport,
        }
    }

    /// Run one document and decode its `data`.
    pub async fn execute<T: DeserializeOwned>(
        &self,
        document: &str,
        variables: Value,
    ) -> Result<T, IntegrationError> {
        let body = json!({ "query": document, "variables": variables });
        let reply = self.transport.post(&self.api_key, &body).await?;
        decode(reply, chrono::Utc::now().timestamp_millis())
    }

    /// The key's user and workspace.
    pub async fn viewer(&self) -> Result<Viewer, IntegrationError> {
        #[derive(Deserialize)]
        struct Data {
            viewer: Viewer,
        }
        let d: Data = self.execute(queries::VIEWER, json!({})).await?;
        Ok(d.viewer)
    }
}

/// Map a reply to `data` or to an [`IntegrationError`]. `now_ms` is the
/// wall clock, for the rate-limit wait.
fn decode<T: DeserializeOwned>(reply: GraphqlReply, now_ms: i64) -> Result<T, IntegrationError> {
    let parsed: Option<Value> = serde_json::from_str(&reply.body).ok();
    let errors: &[Value] = parsed
        .as_ref()
        .and_then(|v| v.get("errors"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let has_code = |code: &str| {
        errors
            .iter()
            .any(|e| e.pointer("/extensions/code").and_then(Value::as_str) == Some(code))
    };

    if has_code("RATELIMITED") || reply.status == 429 {
        let retry_after = reply
            .rate_limit_reset_ms
            .map(|reset| Duration::from_millis(reset.saturating_sub(now_ms).max(0) as u64));
        return Err(IntegrationError::RateLimited { retry_after });
    }
    if reply.status == 401 || has_code("AUTHENTICATION_ERROR") {
        return Err(IntegrationError::Auth(arawn_auth::AuthError::AuthExpired));
    }
    if !errors.is_empty() {
        let messages: Vec<&str> = errors
            .iter()
            .filter_map(|e| e.get("message").and_then(Value::as_str))
            .collect();
        return Err(IntegrationError::Provider(format!(
            "linear: {}",
            if messages.is_empty() {
                "unknown GraphQL error".to_string()
            } else {
                messages.join("; ")
            }
        )));
    }
    if !(200..300).contains(&reply.status) {
        return Err(IntegrationError::Provider(format!(
            "linear: HTTP {}: {}",
            reply.status, reply.body
        )));
    }
    let data = parsed
        .and_then(|mut v| v.get_mut("data").map(Value::take))
        .filter(|d| !d.is_null())
        .ok_or_else(|| IntegrationError::Provider("linear: reply has no data".into()))?;
    serde_json::from_value(data)
        .map_err(|e| IntegrationError::Provider(format!("linear: decode reply: {e}")))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Canned replies in order; records every request body.
    #[derive(Default)]
    pub(crate) struct FakeTransport {
        pub replies: Mutex<std::collections::VecDeque<GraphqlReply>>,
        pub sent: Mutex<Vec<(String, Value)>>,
    }

    impl FakeTransport {
        pub(crate) fn ok(bodies: &[Value]) -> Arc<Self> {
            let t = Self::default();
            for b in bodies {
                t.replies.lock().unwrap().push_back(GraphqlReply {
                    status: 200,
                    rate_limit_reset_ms: None,
                    body: b.to_string(),
                });
            }
            Arc::new(t)
        }

        pub(crate) fn sent(&self) -> Vec<(String, Value)> {
            self.sent.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl LinearTransport for FakeTransport {
        async fn post(
            &self,
            api_key: &str,
            body: &Value,
        ) -> Result<GraphqlReply, IntegrationError> {
            self.sent
                .lock()
                .unwrap()
                .push((api_key.to_string(), body.clone()));
            self.replies
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| IntegrationError::Provider("fake: no reply left".into()))
        }
    }

    pub(crate) fn viewer_body() -> Value {
        json!({"data": {"viewer": {
            "id": "u1", "name": "Ada", "email": "ada@example.com",
            "organization": {"name": "Acme"}
        }}})
    }

    #[test]
    fn request_sends_the_key_without_bearer() {
        let req = HttpTransport::default()
            .request("lin_api_abc", &json!({"query": queries::VIEWER}))
            .unwrap();
        assert_eq!(req.url().as_str(), LINEAR_API_URL);
        assert_eq!(req.method(), reqwest::Method::POST);
        let auth = req.headers().get("authorization").unwrap();
        assert_eq!(auth, "lin_api_abc");
        assert!(!auth.to_str().unwrap().contains("Bearer"));
        let body: Value = serde_json::from_slice(req.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(body["query"], queries::VIEWER);
    }

    #[test]
    fn ratelimited_maps_to_rate_limited_with_wait_from_reset_header() {
        let reply = GraphqlReply {
            status: 400,
            rate_limit_reset_ms: Some(1_000_090_000),
            body: json!({"errors": [{
                "message": "Rate limit exceeded",
                "extensions": {"code": "RATELIMITED"}
            }]})
            .to_string(),
        };
        match decode::<Value>(reply, 1_000_000_000) {
            Err(IntegrationError::RateLimited { retry_after }) => {
                assert_eq!(retry_after, Some(Duration::from_secs(90)))
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn rate_limit_reset_in_the_past_waits_zero() {
        let reply = GraphqlReply {
            status: 400,
            rate_limit_reset_ms: Some(5),
            body: json!({"errors": [{"message": "x", "extensions": {"code": "RATELIMITED"}}]})
                .to_string(),
        };
        assert!(matches!(
            decode::<Value>(reply, 10),
            Err(IntegrationError::RateLimited { retry_after: Some(d) }) if d.is_zero()
        ));
    }

    #[test]
    fn graphql_error_maps_to_provider_error_with_its_message() {
        let reply = GraphqlReply {
            status: 200,
            rate_limit_reset_ms: None,
            body: json!({"data": null, "errors": [{
                "message": "Entity not found: Issue",
                "extensions": {"code": "INVALID_INPUT"}
            }]})
            .to_string(),
        };
        match decode::<Value>(reply, 0) {
            Err(IntegrationError::Provider(m)) => {
                assert!(m.contains("Entity not found: Issue"), "{m}")
            }
            other => panic!("expected Provider, got {other:?}"),
        }
    }

    #[test]
    fn bad_key_maps_to_an_auth_error() {
        for (status, code) in [(401, "AUTHENTICATION_ERROR"), (400, "AUTHENTICATION_ERROR")] {
            let reply = GraphqlReply {
                status,
                rate_limit_reset_ms: None,
                body: json!({"errors": [{"message": "auth", "extensions": {"code": code}}]})
                    .to_string(),
            };
            assert!(matches!(
                decode::<Value>(reply, 0),
                Err(IntegrationError::Auth(_))
            ));
        }
    }

    #[tokio::test]
    async fn viewer_posts_the_named_query_and_decodes() {
        let t = FakeTransport::ok(&[viewer_body()]);
        let c = LinearClient::with_transport("k", t.clone());
        let v = c.viewer().await.unwrap();
        assert_eq!(v.name, "Ada");
        assert_eq!(v.organization.name, "Acme");
        let sent = t.sent();
        assert_eq!(sent[0].0, "k");
        assert_eq!(sent[0].1["query"], queries::VIEWER);
    }
}
