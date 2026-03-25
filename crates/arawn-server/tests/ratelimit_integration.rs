//! Rate limiting integration tests.
//!
//! These tests verify that rate limiting works through the actual HTTP request path,
//! returning 429 when limits are exceeded and 200 when within limits.

mod common;

use anyhow::Result;
use arawn_test_utils::TestServer;
use arawn_test_utils::server::TestServerBuilder;

/// Helper: build a test server with rate limiting enabled and a specific RPM.
async fn rate_limited_server(api_rpm: u32) -> Result<TestServer> {
    TestServerBuilder::new()
        .with_rate_limiting(true)
        .with_api_rpm(api_rpm)
        .build()
        .await
}

#[tokio::test]
async fn test_requests_within_limit_succeed() -> Result<()> {
    // Use a generous limit so startup health polls + our requests fit within the burst.
    let server = rate_limited_server(30).await?;

    // A few requests should all succeed.
    for _ in 0..3 {
        let resp = server.get("/api/v1/sessions").send().await?;
        assert_eq!(
            resp.status().as_u16(),
            200,
            "Requests within limit should succeed"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_requests_exceeding_limit_return_429() -> Result<()> {
    // Very low limit: burst of 5 tokens. Health poll during startup uses 1-3.
    // We send enough to exhaust the remaining burst.
    let server = rate_limited_server(5).await?;

    let mut got_429 = false;

    // Send 10 rapid requests — at some point we should hit the limit.
    for _ in 0..10 {
        let resp = server.get("/api/v1/sessions").send().await?;
        if resp.status().as_u16() == 429 {
            got_429 = true;
            break;
        }
    }

    assert!(
        got_429,
        "Should have received a 429 after exceeding the limit"
    );

    Ok(())
}

#[tokio::test]
async fn test_429_response_has_retry_after_header() -> Result<()> {
    let server = rate_limited_server(5).await?;

    // Exhaust the burst.
    let mut last_429_resp = None;
    for _ in 0..10 {
        let resp = server.get("/api/v1/sessions").send().await?;
        if resp.status().as_u16() == 429 {
            last_429_resp = Some(resp);
            break;
        }
    }

    let resp = last_429_resp.expect("Should have received a 429");
    assert!(
        resp.headers().get("retry-after").is_some(),
        "429 response should include Retry-After header"
    );

    Ok(())
}

#[tokio::test]
async fn test_429_response_body_is_json() -> Result<()> {
    let server = rate_limited_server(5).await?;

    // Exhaust the burst.
    let mut last_429_resp = None;
    for _ in 0..10 {
        let resp = server.get("/api/v1/sessions").send().await?;
        if resp.status().as_u16() == 429 {
            last_429_resp = Some(resp);
            break;
        }
    }

    let resp = last_429_resp.expect("Should have received a 429");
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["code"], 429);
    assert!(body["error"].as_str().unwrap().contains("Rate limit"));

    Ok(())
}

#[tokio::test]
async fn test_rate_limit_resets_after_window() -> Result<()> {
    // Use per_minute(60) = 1 request per second replenishment.
    // Burst of 60, so we won't hit the limit easily.
    // Instead, use a very small budget to test replenishment.
    let server = rate_limited_server(5).await?;

    // Exhaust the burst.
    let mut exhausted = false;
    for _ in 0..10 {
        let resp = server.get("/api/v1/sessions").send().await?;
        if resp.status().as_u16() == 429 {
            exhausted = true;
            break;
        }
    }
    assert!(exhausted, "Should have exhausted the burst");

    // Wait for at least one token to replenish.
    // With 5 RPM, replenishment = 60s / 5 = 12 seconds per token.
    // That's too long for a test. Let's use a higher RPM for this test.
    // We'll spawn a separate server for the reset test.
    drop(server);

    // Use 120 RPM = 1 token per 500ms replenishment.
    let server = rate_limited_server(120).await?;

    // Exhaust the burst by sending 125 requests rapidly.
    let mut exhausted = false;
    for _ in 0..125 {
        let resp = server.get("/api/v1/sessions").send().await?;
        if resp.status().as_u16() == 429 {
            exhausted = true;
            break;
        }
    }
    assert!(exhausted, "Should have exhausted the burst");

    // Wait 600ms for a token to replenish (500ms per token + margin).
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;

    // Next request should succeed (token replenished).
    let resp = server.get("/api/v1/sessions").send().await?;
    assert_eq!(
        resp.status().as_u16(),
        200,
        "Request after replenishment window should succeed"
    );

    Ok(())
}

#[tokio::test]
async fn test_different_ips_have_independent_limits() -> Result<()> {
    // Rate limiting is per-IP. Two different X-Forwarded-For IPs
    // should each get their own burst budget.
    // Must enable trust_proxy so the server reads X-Forwarded-For headers.
    let server = TestServerBuilder::new()
        .with_rate_limiting(true)
        .with_api_rpm(5)
        .with_trust_proxy(true)
        .build()
        .await?;

    // Exhaust the limit for IP "10.0.0.1".
    let mut exhausted_ip1 = false;
    for _ in 0..10 {
        let resp = server
            .client
            .get(format!("{}/api/v1/sessions", server.base_url()))
            .bearer_auth(server.token.as_ref().unwrap())
            .header("x-forwarded-for", "10.0.0.1")
            .send()
            .await?;
        if resp.status().as_u16() == 429 {
            exhausted_ip1 = true;
            break;
        }
    }
    assert!(exhausted_ip1, "IP 10.0.0.1 should be rate limited");

    // IP "10.0.0.2" should still have its own budget.
    let resp = server
        .client
        .get(format!("{}/api/v1/sessions", server.base_url()))
        .bearer_auth(server.token.as_ref().unwrap())
        .header("x-forwarded-for", "10.0.0.2")
        .send()
        .await?;
    assert_eq!(
        resp.status().as_u16(),
        200,
        "Different IP should not be affected by another IP's rate limit"
    );

    Ok(())
}

#[tokio::test]
async fn test_rate_limiting_disabled_allows_all_requests() -> Result<()> {
    // Rate limiting disabled — even many requests should succeed.
    let server: TestServer = TestServerBuilder::new()
        .with_rate_limiting(false)
        .with_api_rpm(1) // Very low limit, but disabled, so it shouldn't matter.
        .build()
        .await?;

    for _ in 0..20 {
        let resp = server.get("/api/v1/sessions").send().await?;
        assert_eq!(
            resp.status().as_u16(),
            200,
            "All requests should succeed when rate limiting is disabled"
        );
    }

    Ok(())
}
