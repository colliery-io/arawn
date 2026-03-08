//! Workstream route integration tests.
//!
//! These tests verify the workstream CRUD, message, and session APIs
//! through the HTTP server.

mod common;

use anyhow::Result;
use serde_json::json;

// ── CRUD basics ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_workstream() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    let resp = server
        .post("/api/v1/workstreams")
        .json(&json!({
            "title": "Test Project",
            "default_model": "gpt-4"
        }))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 201, "Should return 201 Created");

    let body: serde_json::Value = resp.json().await?;
    assert!(
        body.get("id").and_then(|v| v.as_str()).is_some(),
        "Should have id"
    );
    assert_eq!(body["title"].as_str(), Some("Test Project"));
    assert_eq!(body["state"].as_str(), Some("active"));
    assert_eq!(body["is_scratch"].as_bool(), Some(false));
    assert_eq!(body["default_model"].as_str(), Some("gpt-4"));
    assert!(body.get("created_at").is_some(), "Should have created_at");
    assert!(body.get("updated_at").is_some(), "Should have updated_at");

    Ok(())
}

#[tokio::test]
async fn test_create_workstream_with_tags() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    let resp = server
        .post("/api/v1/workstreams")
        .json(&json!({
            "title": "Tagged Project",
            "default_model": "gpt-4",
            "tags": ["rust", "ai"]
        }))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 201);

    let body: serde_json::Value = resp.json().await?;
    let tags = body
        .get("tags")
        .and_then(|v| v.as_array())
        .expect("Should have tags array");

    assert_eq!(tags.len(), 2);
    assert!(tags.iter().any(|t| t.as_str() == Some("rust")));
    assert!(tags.iter().any(|t| t.as_str() == Some("ai")));

    Ok(())
}

#[tokio::test]
async fn test_list_workstreams() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Create two workstreams
    server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "Project Alpha"}))
        .send()
        .await?;

    server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "Project Beta"}))
        .send()
        .await?;

    let resp = server.get("/api/v1/workstreams").send().await?;
    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await?;
    let workstreams = body["workstreams"]
        .as_array()
        .expect("Should have workstreams array");

    // At least the 2 created workstreams (scratch may or may not exist)
    assert!(
        body["total"].as_u64().unwrap_or(0) >= 2,
        "Total should include at least the 2 created workstreams"
    );
    assert!(workstreams.len() >= 2);
    assert!(body.get("limit").is_some(), "Should have limit");
    assert!(body.get("offset").is_some(), "Should have offset");

    Ok(())
}

#[tokio::test]
async fn test_list_workstreams_no_created() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    let resp = server.get("/api/v1/workstreams").send().await?;
    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await?;
    let _workstreams = body["workstreams"]
        .as_array()
        .expect("Should have workstreams array");

    // Response should be well-formed with pagination fields
    assert!(body.get("total").is_some(), "Should have total");
    assert!(body.get("limit").is_some(), "Should have limit");
    assert!(body.get("offset").is_some(), "Should have offset");

    Ok(())
}

#[tokio::test]
async fn test_get_workstream() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Create a workstream
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({
            "title": "Fetch Me",
            "default_model": "claude-3"
        }))
        .send()
        .await?;
    assert_eq!(create_resp.status().as_u16(), 201);

    let created: serde_json::Value = create_resp.json().await?;
    let id = created["id"].as_str().expect("Should have id");

    // GET by id
    let resp = server
        .get(&format!("/api/v1/workstreams/{}", id))
        .send()
        .await?;
    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["id"].as_str(), Some(id));
    assert_eq!(body["title"].as_str(), Some("Fetch Me"));
    assert_eq!(body["default_model"].as_str(), Some("claude-3"));
    assert_eq!(body["state"].as_str(), Some("active"));

    Ok(())
}

#[tokio::test]
async fn test_get_workstream_not_found() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    let resp = server
        .get("/api/v1/workstreams/nonexistent-ws-id")
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

#[tokio::test]
async fn test_update_workstream_title() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Create a workstream
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "Old Title"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let id = created["id"].as_str().expect("Should have id");

    // PATCH title
    let body = json!({"title": "New Title"});
    let resp = server
        .client
        .patch(format!("{}/api/v1/workstreams/{}", server.base_url(), id))
        .bearer_auth(server.token.as_deref().unwrap_or(""))
        .json(&body)
        .send()
        .await?;

    assert!(resp.status().is_success());

    let updated: serde_json::Value = resp.json().await?;
    assert_eq!(updated["title"].as_str(), Some("New Title"));

    Ok(())
}

#[tokio::test]
async fn test_update_workstream_summary() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Create a workstream
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "Summary Test"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let id = created["id"].as_str().expect("Should have id");

    // PATCH summary
    let body = json!({"summary": "A detailed summary of this workstream."});
    let resp = server
        .client
        .patch(format!("{}/api/v1/workstreams/{}", server.base_url(), id))
        .bearer_auth(server.token.as_deref().unwrap_or(""))
        .json(&body)
        .send()
        .await?;

    assert!(resp.status().is_success());

    let updated: serde_json::Value = resp.json().await?;
    assert_eq!(
        updated["summary"].as_str(),
        Some("A detailed summary of this workstream.")
    );

    Ok(())
}

#[tokio::test]
async fn test_delete_workstream() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Create a workstream
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "To Be Archived"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let id = created["id"].as_str().expect("Should have id");

    // DELETE (archive)
    let resp = server
        .delete(&format!("/api/v1/workstreams/{}", id))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 204);

    Ok(())
}

#[tokio::test]
async fn test_delete_workstream_not_found() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    let resp = server
        .delete("/api/v1/workstreams/nonexistent-ws-id")
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

// ── Messages ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_send_message() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Create a workstream
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "Message Test"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let id = created["id"].as_str().expect("Should have id");

    // Send a message
    let resp = server
        .post(&format!("/api/v1/workstreams/{}/messages", id))
        .json(&json!({
            "content": "Hello, world!",
            "role": "user"
        }))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 201);

    let msg: serde_json::Value = resp.json().await?;
    assert!(
        msg.get("id").and_then(|v| v.as_str()).is_some(),
        "Should have id"
    );
    assert_eq!(msg["workstream_id"].as_str(), Some(id));
    assert_eq!(msg["role"].as_str(), Some("user"));
    assert_eq!(msg["content"].as_str(), Some("Hello, world!"));
    assert!(msg.get("timestamp").is_some(), "Should have timestamp");

    Ok(())
}

#[tokio::test]
async fn test_send_message_default_role() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Create a workstream
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "Default Role Test"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let id = created["id"].as_str().expect("Should have id");

    // Send message without role
    let resp = server
        .post(&format!("/api/v1/workstreams/{}/messages", id))
        .json(&json!({"content": "No explicit role"}))
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 201);

    let msg: serde_json::Value = resp.json().await?;
    assert_eq!(
        msg["role"].as_str(),
        Some("user"),
        "Role should default to 'user'"
    );

    Ok(())
}

#[tokio::test]
async fn test_list_messages() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Create a workstream
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "List Messages Test"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let id = created["id"].as_str().expect("Should have id");

    // Send two messages
    server
        .post(&format!("/api/v1/workstreams/{}/messages", id))
        .json(&json!({"content": "First message", "role": "user"}))
        .send()
        .await?;

    server
        .post(&format!("/api/v1/workstreams/{}/messages", id))
        .json(&json!({"content": "Second message", "role": "assistant"}))
        .send()
        .await?;

    // List messages
    let resp = server
        .get(&format!("/api/v1/workstreams/{}/messages", id))
        .send()
        .await?;

    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await?;
    let messages = body["messages"]
        .as_array()
        .expect("Should have messages array");

    assert!(body["total"].as_u64().unwrap_or(0) >= 2);
    assert!(messages.len() >= 2);
    assert!(body.get("limit").is_some(), "Should have limit");
    assert!(body.get("offset").is_some(), "Should have offset");

    Ok(())
}

#[tokio::test]
async fn test_list_messages_empty() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Create a workstream
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "Empty Messages Test"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let id = created["id"].as_str().expect("Should have id");

    // List messages (should be empty)
    let resp = server
        .get(&format!("/api/v1/workstreams/{}/messages", id))
        .send()
        .await?;

    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await?;
    let messages = body["messages"]
        .as_array()
        .expect("Should have messages array");

    assert_eq!(messages.len(), 0, "Should have no messages");
    assert_eq!(body["total"].as_u64(), Some(0));

    Ok(())
}

// ── Sessions ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_workstream_sessions() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Create a workstream
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "Sessions Test"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let id = created["id"].as_str().expect("Should have id");

    // List sessions (initially empty)
    let resp = server
        .get(&format!("/api/v1/workstreams/{}/sessions", id))
        .send()
        .await?;

    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await?;
    assert!(
        body.get("sessions").and_then(|v| v.as_array()).is_some(),
        "Should have sessions array"
    );
    assert!(body.get("total").is_some(), "Should have total");
    assert!(body.get("limit").is_some(), "Should have limit");
    assert!(body.get("offset").is_some(), "Should have offset");

    Ok(())
}

// ── Edge cases ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_invalid_workstream_id() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Path traversal is handled by the router — may return 404 (no matching route)
    let resp = server.get("/api/v1/workstreams/../foo").send().await?;

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 404,
        "Path traversal ID should return 400 or 404, got {}",
        status
    );

    Ok(())
}

#[tokio::test]
async fn test_send_message_invalid_role() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Create a workstream
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "Invalid Role Test"}))
        .send()
        .await?;
    let created: serde_json::Value = create_resp.json().await?;
    let id = created["id"].as_str().expect("Should have id");

    // Send message with invalid role
    let resp = server
        .post(&format!("/api/v1/workstreams/{}/messages", id))
        .json(&json!({
            "content": "Bad role",
            "role": "invalid"
        }))
        .send()
        .await?;

    assert_eq!(
        resp.status().as_u16(),
        400,
        "Invalid role should return 400"
    );

    Ok(())
}

#[tokio::test]
async fn test_workstreams_not_configured() -> Result<()> {
    // Start server WITHOUT .with_workstreams()
    let server = common::TestServer::builder().build().await?;

    let resp = server.get("/api/v1/workstreams").send().await?;

    assert_eq!(
        resp.status().as_u16(),
        503,
        "Should return 503 when workstreams are not configured"
    );

    Ok(())
}

#[tokio::test]
async fn test_delete_nonexistent_workstream_id() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Deleting a non-existent workstream should return 404
    let resp = server
        .delete("/api/v1/workstreams/does-not-exist-at-all")
        .send()
        .await?;

    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

#[tokio::test]
async fn test_create_and_get_roundtrip() -> Result<()> {
    let server = common::TestServer::builder()
        .with_workstreams()
        .build()
        .await?;

    // Create with all fields
    let create_resp = server
        .post("/api/v1/workstreams")
        .json(&json!({
            "title": "Roundtrip Project",
            "default_model": "claude-3-opus",
            "tags": ["integration", "test", "roundtrip"]
        }))
        .send()
        .await?;

    assert_eq!(create_resp.status().as_u16(), 201);
    let created: serde_json::Value = create_resp.json().await?;
    let id = created["id"].as_str().expect("Should have id");

    // GET the same workstream
    let get_resp = server
        .get(&format!("/api/v1/workstreams/{}", id))
        .send()
        .await?;

    assert!(get_resp.status().is_success());
    let fetched: serde_json::Value = get_resp.json().await?;

    // Verify all fields match
    assert_eq!(fetched["id"].as_str(), Some(id));
    assert_eq!(fetched["title"].as_str(), Some("Roundtrip Project"));
    assert_eq!(fetched["default_model"].as_str(), Some("claude-3-opus"));
    assert_eq!(fetched["state"].as_str(), Some("active"));
    assert_eq!(fetched["is_scratch"].as_bool(), Some(false));
    assert_eq!(
        fetched["created_at"].as_str(),
        created["created_at"].as_str()
    );

    // Verify tags roundtrip
    let tags = fetched
        .get("tags")
        .and_then(|v| v.as_array())
        .expect("GET response should include tags");
    assert_eq!(tags.len(), 3);
    assert!(tags.iter().any(|t| t.as_str() == Some("integration")));
    assert!(tags.iter().any(|t| t.as_str() == Some("test")));
    assert!(tags.iter().any(|t| t.as_str() == Some("roundtrip")));

    Ok(())
}
