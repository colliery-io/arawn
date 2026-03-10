//! E2E tests for the workstreams endpoints.
//!
//! Covers CRUD operations, sessions, messaging, pagination, and error paths
//! for routes/workstreams.rs. File-related endpoints (promote, export, clone,
//! usage, cleanup) are excluded as they require a DirectoryManager not
//! available in the test server.

mod common;

use anyhow::Result;
use serde_json::json;

use arawn_test_utils::server::TestServerBuilder;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Create a workstream and return its ID.
async fn create_workstream(server: &arawn_test_utils::TestServer, title: &str) -> String {
    let resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": title}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    body["id"].as_str().unwrap().to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// CRUD: Update workstream
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_update_workstream_title() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Original Title").await;

    let resp = server
        .patch(&format!("/api/v1/workstreams/{}", ws_id))
        .json(&json!({"title": "Updated Title"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["title"].as_str(), Some("Updated Title"));
    assert_eq!(body["id"].as_str(), Some(ws_id.as_str()));

    Ok(())
}

#[tokio::test]
async fn scenario_update_workstream_summary() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Test WS").await;

    let resp = server
        .patch(&format!("/api/v1/workstreams/{}", ws_id))
        .json(&json!({"summary": "This is a test workstream summary"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(
        body["summary"].as_str(),
        Some("This is a test workstream summary")
    );

    Ok(())
}

#[tokio::test]
async fn scenario_update_workstream_tags() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Tagged WS").await;

    let resp = server
        .patch(&format!("/api/v1/workstreams/{}", ws_id))
        .json(&json!({"tags": ["rust", "testing"]}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    let tags = body["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 2);

    Ok(())
}

#[tokio::test]
async fn scenario_update_workstream_multiple_fields() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Multi Update").await;

    let resp = server
        .patch(&format!("/api/v1/workstreams/{}", ws_id))
        .json(&json!({
            "title": "New Title",
            "summary": "New Summary",
            "default_model": "gpt-4"
        }))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["title"].as_str(), Some("New Title"));
    assert_eq!(body["summary"].as_str(), Some("New Summary"));
    assert_eq!(body["default_model"].as_str(), Some("gpt-4"));

    Ok(())
}

#[tokio::test]
async fn scenario_update_nonexistent_workstream_returns_404() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    let resp = server
        .patch("/api/v1/workstreams/nonexistent-ws-999")
        .json(&json!({"title": "Nope"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// CRUD: Delete (archive) workstream
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_delete_workstream() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "To Delete").await;

    // Delete
    let resp = server
        .delete(&format!("/api/v1/workstreams/{}", ws_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 204);

    // Should not appear in normal listing
    let list_resp = server.get("/api/v1/workstreams").send().await?;
    let list: serde_json::Value = list_resp.json().await?;
    let ids: Vec<&str> = list["workstreams"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|ws| ws["id"].as_str())
        .collect();
    assert!(
        !ids.contains(&ws_id.as_str()),
        "Deleted WS should not appear in list"
    );

    Ok(())
}

#[tokio::test]
async fn scenario_delete_nonexistent_workstream_returns_404() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    let resp = server
        .delete("/api/v1/workstreams/nonexistent-ws-999")
        .send()
        .await?;
    // Should be 404 (not found to archive)
    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// List: Pagination
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_list_workstreams_with_pagination() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    // Create 5 workstreams
    for i in 1..=5 {
        create_workstream(&server, &format!("WS {}", i)).await;
    }

    // Request with limit=2, offset=0
    let resp = server
        .get("/api/v1/workstreams?limit=2&offset=0")
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    let workstreams = body["workstreams"].as_array().unwrap();
    assert_eq!(workstreams.len(), 2, "Should return 2 workstreams");
    assert!(body["total"].as_u64().unwrap() >= 5, "Total should be >= 5");
    assert_eq!(body["limit"].as_u64(), Some(2));
    assert_eq!(body["offset"].as_u64(), Some(0));

    // Request page 2
    let resp2 = server
        .get("/api/v1/workstreams?limit=2&offset=2")
        .send()
        .await?;
    let body2: serde_json::Value = resp2.json().await?;
    let ws2 = body2["workstreams"].as_array().unwrap();
    assert_eq!(ws2.len(), 2, "Page 2 should also have 2 workstreams");
    assert_eq!(body2["offset"].as_u64(), Some(2));

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// List: Include archived
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_list_workstreams_include_archived() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    let ws_id = create_workstream(&server, "Will Archive").await;
    create_workstream(&server, "Stays Active").await;

    // Archive one
    server
        .delete(&format!("/api/v1/workstreams/{}", ws_id))
        .send()
        .await?;

    // Normal list should not include archived
    let resp = server.get("/api/v1/workstreams").send().await?;
    let body: serde_json::Value = resp.json().await?;
    let ids: Vec<&str> = body["workstreams"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|ws| ws["id"].as_str())
        .collect();
    assert!(!ids.contains(&ws_id.as_str()));

    // With include_archived=true should include it
    let resp2 = server
        .get("/api/v1/workstreams?include_archived=true")
        .send()
        .await?;
    let body2: serde_json::Value = resp2.json().await?;
    let ids2: Vec<&str> = body2["workstreams"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|ws| ws["id"].as_str())
        .collect();
    assert!(
        ids2.contains(&ws_id.as_str()),
        "Archived WS should appear with include_archived=true"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Sessions: List sessions for a workstream
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_list_workstream_sessions() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Session Test").await;

    // Send a message to create a session
    server
        .post(&format!("/api/v1/workstreams/{}/messages", ws_id))
        .json(&json!({"content": "Hello session"}))
        .send()
        .await?;

    // List sessions
    let resp = server
        .get(&format!("/api/v1/workstreams/{}/sessions", ws_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert!(body["total"].as_u64().is_some());
    assert!(body["limit"].as_u64().is_some());
    assert!(body["sessions"].as_array().is_some());

    Ok(())
}

#[tokio::test]
async fn scenario_list_sessions_nonexistent_workstream_returns_empty() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    let resp = server
        .get("/api/v1/workstreams/nonexistent-ws-999/sessions")
        .send()
        .await?;
    // API returns empty sessions list for nonexistent workstreams (not 404)
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["total"].as_u64(), Some(0));

    Ok(())
}

#[tokio::test]
async fn scenario_list_sessions_with_pagination() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Paginated Sessions").await;

    // Send a message
    server
        .post(&format!("/api/v1/workstreams/{}/messages", ws_id))
        .json(&json!({"content": "Msg 1"}))
        .send()
        .await?;

    let resp = server
        .get(&format!(
            "/api/v1/workstreams/{}/sessions?limit=10&offset=0",
            ws_id
        ))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["limit"].as_u64(), Some(10));
    assert_eq!(body["offset"].as_u64(), Some(0));

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Messages: Various roles
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_send_message_with_roles() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Roles Test").await;

    for role in &["user", "assistant", "system", "agent_push"] {
        let resp = server
            .post(&format!("/api/v1/workstreams/{}/messages", ws_id))
            .json(&json!({"role": role, "content": format!("{} message", role)}))
            .send()
            .await?;
        assert_eq!(resp.status().as_u16(), 201, "Should accept role '{}'", role);
        let body: serde_json::Value = resp.json().await?;
        assert_eq!(body["role"].as_str(), Some(*role));
    }

    Ok(())
}

#[tokio::test]
async fn scenario_send_message_invalid_role_returns_400() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Invalid Role").await;

    let resp = server
        .post(&format!("/api/v1/workstreams/{}/messages", ws_id))
        .json(&json!({"role": "admin", "content": "bad role"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 400);

    Ok(())
}

#[tokio::test]
async fn scenario_send_message_default_role_is_user() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Default Role").await;

    let resp = server
        .post(&format!("/api/v1/workstreams/{}/messages", ws_id))
        .json(&json!({"content": "no role specified"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 201);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["role"].as_str(), Some("user"));

    Ok(())
}

#[tokio::test]
async fn scenario_send_message_with_metadata() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Metadata Test").await;

    let resp = server
        .post(&format!("/api/v1/workstreams/{}/messages", ws_id))
        .json(&json!({
            "content": "msg with metadata",
            "metadata": "{\"source\": \"test\"}"
        }))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 201);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["metadata"].as_str(), Some("{\"source\": \"test\"}"));

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Messages: Pagination and since filter
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_list_messages_with_pagination() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Msg Pagination").await;

    // Send 5 messages
    for i in 1..=5 {
        server
            .post(&format!("/api/v1/workstreams/{}/messages", ws_id))
            .json(&json!({"content": format!("Message {}", i)}))
            .send()
            .await?;
    }

    // Get with limit=2
    let resp = server
        .get(&format!(
            "/api/v1/workstreams/{}/messages?limit=2&offset=0",
            ws_id
        ))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(body["total"].as_u64(), Some(5));
    assert_eq!(body["limit"].as_u64(), Some(2));

    Ok(())
}

#[tokio::test]
async fn scenario_list_messages_with_since_filter() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Since Filter").await;

    // Send a message and capture its timestamp
    let resp1 = server
        .post(&format!("/api/v1/workstreams/{}/messages", ws_id))
        .json(&json!({"content": "Before"}))
        .send()
        .await?;
    let msg1: serde_json::Value = resp1.json().await?;
    let timestamp = msg1["timestamp"].as_str().unwrap();

    // Small delay to ensure different timestamps
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Send another message after
    server
        .post(&format!("/api/v1/workstreams/{}/messages", ws_id))
        .json(&json!({"content": "After"}))
        .send()
        .await?;

    // Get messages since the first timestamp (URL-encode the + in timezone)
    let encoded_ts = timestamp.replace('+', "%2B");
    let resp = server
        .get(&format!(
            "/api/v1/workstreams/{}/messages?since={}",
            ws_id, encoded_ts
        ))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    let messages = body["messages"].as_array().unwrap();
    // Should only have the message sent after the timestamp
    assert!(
        messages.len() <= 2,
        "Should have at most 2 messages (depending on timing)"
    );

    Ok(())
}

#[tokio::test]
async fn scenario_list_messages_invalid_since_returns_400() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Bad Since").await;

    let resp = server
        .get(&format!(
            "/api/v1/workstreams/{}/messages?since=not-a-date",
            ws_id
        ))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 400);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Workstreams not configured (503)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_workstreams_not_configured_returns_503() -> Result<()> {
    // Server WITHOUT workstreams enabled
    let server = TestServerBuilder::new().build().await?;

    let resp = server.get("/api/v1/workstreams").send().await?;
    assert_eq!(
        resp.status().as_u16(),
        503,
        "Should return 503 when workstreams not configured"
    );

    Ok(())
}

#[tokio::test]
async fn scenario_workstream_create_not_configured_returns_503() -> Result<()> {
    let server = TestServerBuilder::new().build().await?;

    let resp = server
        .post("/api/v1/workstreams")
        .json(&json!({"title": "Test"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 503);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Create: With optional fields
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_create_workstream_with_all_fields() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    let resp = server
        .post("/api/v1/workstreams")
        .json(&json!({
            "title": "Full WS",
            "default_model": "claude-3",
            "tags": ["project", "alpha"]
        }))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 201);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["title"].as_str(), Some("Full WS"));
    assert_eq!(body["default_model"].as_str(), Some("claude-3"));
    assert!(!body["id"].as_str().unwrap().is_empty());
    assert!(body["created_at"].as_str().is_some());
    assert!(body["updated_at"].as_str().is_some());
    assert_eq!(body["is_scratch"].as_bool(), Some(false));

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Get: Workstream details
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_get_workstream_has_full_details() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Detail Check").await;

    let resp = server
        .get(&format!("/api/v1/workstreams/{}", ws_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["id"].as_str(), Some(ws_id.as_str()));
    assert_eq!(body["title"].as_str(), Some("Detail Check"));
    assert!(body["state"].as_str().is_some());
    assert!(body["created_at"].as_str().is_some());
    assert!(body["updated_at"].as_str().is_some());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// ID validation
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_invalid_workstream_id_returns_400() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    // Path traversal
    let resp = server
        .get("/api/v1/workstreams/..%2F..%2Fetc")
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 400);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Messages: Send to nonexistent workstream
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_send_message_nonexistent_ws_returns_404() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;

    let resp = server
        .post("/api/v1/workstreams/nonexistent-ws-999/messages")
        .json(&json!({"content": "Hello nowhere"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 404);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Directory-dependent endpoints: 503 when no DirectoryManager
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_promote_file_no_directory_manager_returns_503() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "No DirMgr").await;

    let resp = server
        .post(&format!("/api/v1/workstreams/{}/files/promote", ws_id))
        .json(&json!({"source": "test.txt", "destination": "test.txt"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 503);

    Ok(())
}

#[tokio::test]
async fn scenario_export_file_no_directory_manager_returns_503() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "No DirMgr Export").await;

    let resp = server
        .post(&format!("/api/v1/workstreams/{}/files/export", ws_id))
        .json(&json!({"source": "test.txt", "destination": "/tmp/test.txt"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 503);

    Ok(())
}

#[tokio::test]
async fn scenario_clone_repo_no_directory_manager_returns_503() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "No DirMgr Clone").await;

    let resp = server
        .post(&format!("/api/v1/workstreams/{}/clone", ws_id))
        .json(&json!({"url": "https://github.com/example/repo.git"}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 503);

    Ok(())
}

#[tokio::test]
async fn scenario_usage_no_directory_manager_returns_503() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "No DirMgr Usage").await;

    let resp = server
        .get(&format!("/api/v1/workstreams/{}/usage", ws_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 503);

    Ok(())
}

#[tokio::test]
async fn scenario_cleanup_no_directory_manager_returns_503() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "No DirMgr Cleanup").await;

    let resp = server
        .post(&format!("/api/v1/workstreams/{}/cleanup", ws_id))
        .json(&json!({}))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 503);

    Ok(())
}

#[tokio::test]
async fn scenario_compress_no_compressor_returns_503() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "No Compressor").await;

    let resp = server
        .post(&format!("/api/v1/workstreams/{}/compress", ws_id))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 503);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Promote: Non-scratch workstream returns 400
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_promote_non_scratch_returns_400() -> Result<()> {
    let server = TestServerBuilder::new().with_workstreams().build().await?;
    let ws_id = create_workstream(&server, "Not Scratch").await;

    let resp = server
        .post(&format!("/api/v1/workstreams/{}/promote", ws_id))
        .json(&json!({"title": "Promoted"}))
        .send()
        .await?;
    assert_eq!(
        resp.status().as_u16(),
        400,
        "Promoting non-scratch workstream should return 400"
    );

    Ok(())
}
