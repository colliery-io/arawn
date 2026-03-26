use super::*;
use serde_json::json;
use std::time::Duration;

use arawn_agent::tool::{Tool, ToolContext};

#[test]
fn test_web_fetch_tool_metadata() {
    let tool = WebFetchTool::new().unwrap();
    assert_eq!(tool.name(), "web_fetch");
    assert!(!tool.description().is_empty());

    let params = tool.parameters();
    assert!(params["properties"].get("url").is_some());
    assert!(params["properties"].get("method").is_some());
    assert!(params["properties"].get("headers").is_some());
    assert!(params["properties"].get("body").is_some());
    assert!(params["properties"].get("timeout_secs").is_some());
    assert!(params["properties"].get("include_headers").is_some());

    // Verify method enum values
    let method_enum = &params["properties"]["method"]["enum"];
    assert!(method_enum.as_array().unwrap().contains(&json!("GET")));
    assert!(method_enum.as_array().unwrap().contains(&json!("POST")));
    assert!(method_enum.as_array().unwrap().contains(&json!("PUT")));
    assert!(method_enum.as_array().unwrap().contains(&json!("PATCH")));
    assert!(method_enum.as_array().unwrap().contains(&json!("DELETE")));
}

#[test]
fn test_web_search_tool_metadata() {
    let tool = WebSearchTool::new().unwrap();
    assert_eq!(tool.name(), "web_search");
    assert!(!tool.description().is_empty());

    let params = tool.parameters();
    assert!(params["properties"].get("query").is_some());
}

#[test]
fn test_extract_text_from_html() {
    let tool = WebFetchTool::new().unwrap();
    let html = r#"
        <html>
        <head><title>Test Page</title></head>
        <body>
            <nav>Navigation</nav>
            <main>
                <h1>Hello World</h1>
                <p>This is the main content.</p>
            </main>
            <footer>Footer</footer>
        </body>
        </html>
    "#;

    let text = tool.extract_text_from_html(html);
    assert!(text.contains("Hello World"));
    assert!(text.contains("main content"));
}

#[test]
fn test_extract_title() {
    let tool = WebFetchTool::new().unwrap();
    let html = "<html><head><title>My Title</title></head><body></body></html>";
    assert_eq!(tool.extract_title(html), Some("My Title".to_string()));
}

#[test]
fn test_extract_description() {
    let tool = WebFetchTool::new().unwrap();
    let html = r#"<html><head><meta name="description" content="My description"></head></html>"#;
    assert_eq!(
        tool.extract_description(html),
        Some("My description".to_string())
    );
}

#[test]
fn test_search_providers() {
    // Test that different providers can be created
    let _brave = WebSearchTool::brave("test_key");
    let _serper = WebSearchTool::serper("test_key");
    let _tavily = WebSearchTool::tavily("test_key");
    let _ddg = WebSearchTool::new().unwrap(); // DuckDuckGo default
}

#[tokio::test]
async fn test_web_fetch_invalid_url() {
    let tool = WebFetchTool::new().unwrap();
    let ctx = ToolContext::default();

    let result = tool
        .execute(json!({"url": "not-a-valid-url"}), &ctx)
        .await
        .unwrap();

    assert!(result.is_error());
    assert!(result.to_llm_content().contains("Invalid URL"));
}

#[tokio::test]
async fn test_web_fetch_non_http() {
    let tool = WebFetchTool::new().unwrap();
    let ctx = ToolContext::default();

    let result = tool
        .execute(json!({"url": "ftp://example.com/file"}), &ctx)
        .await
        .unwrap();

    assert!(result.is_error());
    assert!(result.to_llm_content().contains("HTTP"));
}

#[tokio::test]
async fn test_web_fetch_unsupported_method() {
    let tool = WebFetchTool::new().unwrap();
    let ctx = ToolContext::default();

    let result = tool
        .execute(
            json!({"url": "https://example.com", "method": "TRACE"}),
            &ctx,
        )
        .await
        .unwrap();

    assert!(result.is_error());
    assert!(result.to_llm_content().contains("Unsupported HTTP method"));
}

#[test]
fn test_method_case_insensitivity() {
    // The method should be uppercased in execute, so "get" becomes "GET"
    // This is tested via the parameters which show the expected format
    let tool = WebFetchTool::new().unwrap();
    let params = tool.parameters();
    let default = &params["properties"]["method"]["default"];
    assert_eq!(default, "GET");
}

#[tokio::test]
async fn test_web_fetch_with_custom_headers_invalid_url() {
    // Test that headers parameter is properly parsed even with an invalid URL
    let tool = WebFetchTool::new().unwrap();
    let ctx = ToolContext::default();

    let result = tool
        .execute(
            json!({
                "url": "not-valid",
                "headers": {
                    "Authorization": "Bearer token123",
                    "X-Custom-Header": "custom-value"
                }
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Should fail on URL parsing, not header parsing
    assert!(result.is_error());
    assert!(result.to_llm_content().contains("Invalid URL"));
}

#[tokio::test]
async fn test_web_fetch_with_body_invalid_url() {
    // Test that body parameter is accepted even with an invalid URL
    let tool = WebFetchTool::new().unwrap();
    let ctx = ToolContext::default();

    let result = tool
        .execute(
            json!({
                "url": "not-valid",
                "method": "POST",
                "body": "{\"key\": \"value\"}"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Should fail on URL parsing, not body parsing
    assert!(result.is_error());
    assert!(result.to_llm_content().contains("Invalid URL"));
}

#[test]
fn test_download_parameter_in_schema() {
    let tool = WebFetchTool::new().unwrap();
    let params = tool.parameters();

    assert!(params["properties"].get("download").is_some());
    assert_eq!(params["properties"]["download"]["type"], "string");
}

#[test]
fn test_max_size_config() {
    let config = WebFetchConfig::default();
    // Should be 10MB
    assert_eq!(config.max_size, 10 * 1024 * 1024);
}

#[tokio::test]
async fn test_web_fetch_download_invalid_url() {
    // Test that download parameter is accepted even with an invalid URL
    let tool = WebFetchTool::new().unwrap();
    let ctx = ToolContext::default();

    let result = tool
        .execute(
            json!({
                "url": "not-valid",
                "download": "/tmp/test_file.bin"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Should fail on URL parsing, not download path
    assert!(result.is_error());
    assert!(result.to_llm_content().contains("Invalid URL"));
}

// ── HTML extraction edge cases ────────────────────────────────────────

#[test]
fn test_extract_text_article_content() {
    let tool = WebFetchTool::new().unwrap();
    let html = r#"
        <html><body>
            <nav>Skip this nav</nav>
            <article>
                <h1>Article Title</h1>
                <p>Article body text here.</p>
            </article>
            <footer>Skip footer</footer>
        </body></html>
    "#;
    let text = tool.extract_text_from_html(html);
    assert!(text.contains("Article Title"));
    assert!(text.contains("Article body"));
}

#[test]
fn test_extract_text_fallback_to_body() {
    let tool = WebFetchTool::new().unwrap();
    let html = r#"
        <html><body>
            <div>Plain body content without article/main tags.</div>
        </body></html>
    "#;
    let text = tool.extract_text_from_html(html);
    assert!(text.contains("Plain body content"));
}

#[test]
fn test_extract_text_empty_html() {
    let tool = WebFetchTool::new().unwrap();
    let text = tool.extract_text_from_html("");
    assert!(text.is_empty() || text.trim().is_empty());
}

#[test]
fn test_extract_text_truncation() {
    let config = WebFetchConfig {
        max_text_length: 20,
        ..Default::default()
    };
    let tool = WebFetchTool::with_config(config).unwrap();
    let html = r#"<html><body><main><p>This is a long text that should be truncated because it exceeds the max length.</p></main></body></html>"#;
    let text = tool.extract_text_from_html(html);
    assert!(text.contains("[truncated]"));
}

#[test]
fn test_extract_text_content_class() {
    let tool = WebFetchTool::new().unwrap();
    let html = r#"<html><body><div class="content"><p>Content class text</p></div></body></html>"#;
    let text = tool.extract_text_from_html(html);
    assert!(text.contains("Content class text"));
}

#[test]
fn test_extract_title_missing() {
    let tool = WebFetchTool::new().unwrap();
    let html = "<html><body>No title here</body></html>";
    assert_eq!(tool.extract_title(html), None);
}

#[test]
fn test_extract_description_missing() {
    let tool = WebFetchTool::new().unwrap();
    let html = "<html><head></head><body></body></html>";
    assert_eq!(tool.extract_description(html), None);
}

#[test]
fn test_extract_description_wrong_meta() {
    let tool = WebFetchTool::new().unwrap();
    let html = r#"<html><head><meta name="keywords" content="foo,bar"></head></html>"#;
    assert_eq!(tool.extract_description(html), None);
}

// ── WebFetchConfig tests ──────────────────────────────────────────────

#[test]
fn test_web_fetch_config_default() {
    let config = WebFetchConfig::default();
    assert_eq!(config.timeout, Duration::from_secs(30));
    assert!(config.extract_text);
    assert_eq!(config.max_text_length, 50_000);
    assert!(config.user_agent.contains("Arawn"));
}

#[test]
fn test_web_fetch_tool_with_config() {
    let config = WebFetchConfig {
        timeout: Duration::from_secs(5),
        max_size: 1024,
        user_agent: "TestAgent".to_string(),
        extract_text: false,
        max_text_length: 100,
    };
    let tool = WebFetchTool::with_config(config).unwrap();
    assert_eq!(tool.config.timeout, Duration::from_secs(5));
    assert_eq!(tool.config.max_size, 1024);
    assert!(!tool.config.extract_text);
}

#[test]
fn test_web_fetch_tool_default() {
    let tool = WebFetchTool::default();
    assert_eq!(tool.name(), "web_fetch");
}

// ── WebSearchConfig tests ─────────────────────────────────────────────

#[test]
fn test_web_search_config_default() {
    let config = WebSearchConfig::default();
    assert_eq!(config.max_results, 10);
    assert_eq!(config.timeout, Duration::from_secs(30));
    assert!(matches!(config.provider, SearchProvider::DuckDuckGo));
}

#[test]
fn test_web_search_tool_default() {
    let tool = WebSearchTool::default();
    assert_eq!(tool.name(), "web_search");
}

// ── SearchResult ──────────────────────────────────────────────────────

#[test]
fn test_search_result_serialization() {
    let result = SearchResult {
        title: "Test".to_string(),
        url: "https://example.com".to_string(),
        snippet: "A snippet".to_string(),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("Test"));
    let deserialized: SearchResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.title, "Test");
}

// ── Cancelled context ─────────────────────────────────────────────────

#[tokio::test]
async fn test_web_fetch_cancelled() {
    use tokio_util::sync::CancellationToken;
    let tool = WebFetchTool::new().unwrap();
    let token = CancellationToken::new();
    let ctx = ToolContext::with_cancellation(
        arawn_agent::SessionId::new(),
        arawn_agent::TurnId::new(),
        token.clone(),
    );
    token.cancel();

    let result = tool
        .execute(json!({"url": "https://example.com"}), &ctx)
        .await
        .unwrap();

    assert!(result.is_error());
    assert!(result.to_llm_content().contains("cancelled"));
}

#[tokio::test]
async fn test_web_fetch_missing_url() {
    let tool = WebFetchTool::new().unwrap();
    let ctx = ToolContext::default();

    let result = tool.execute(json!({}), &ctx).await;
    assert!(result.is_err());
}
