use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::StreamExt;
use lru::LruCache;
use serde_json::{Value, json};

use arawn_llm::{ChatContent, ChatMessage, ChatRequest};

use crate::context::ToolContext;
use crate::error::EngineError;
use crate::tool::{Tool, ToolOutput};

/// Cache TTL: 15 minutes.
const CACHE_TTL: Duration = Duration::from_secs(15 * 60);

/// Maximum cache entries.
const CACHE_MAX_ENTRIES: usize = 64;

/// Max content size before truncation (100KB).
const MAX_CONTENT_BYTES: usize = 102_400;

/// Cached fetch result.
struct CacheEntry {
    content: String,
    content_type: String,
    fetched_at: Instant,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.fetched_at.elapsed() > CACHE_TTL
    }
}

/// Fetches content from a URL, converts HTML to markdown, caches results,
/// and optionally uses an LLM to extract information via the `prompt` parameter.
pub struct WebFetchTool {
    cache: Arc<Mutex<LruCache<String, CacheEntry>>>,
}

impl WebFetchTool {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(CACHE_MAX_ENTRIES).unwrap(),
            ))),
        }
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL and optionally extract specific information. \
         The URL must be a complete, valid URL (e.g. https://example.com/path). \
         Do NOT pass partial or truncated URLs. HTTP URLs are automatically upgraded to HTTPS. \
         If `prompt` is provided, an AI model extracts only the relevant information. \
         If `prompt` is omitted, the raw content is returned (HTML converted to markdown)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch content from (must be a fully-formed valid URL; HTTP URLs are automatically upgraded to HTTPS)"
                },
                "prompt": {
                    "type": "string",
                    "description": "The prompt to run on the fetched content, describing what information to extract from the page"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Tool("missing 'url' parameter".into()))?;

        let prompt = params.get("prompt").and_then(|v| v.as_str()).unwrap_or("");

        // Upgrade http to https
        let url = if url.starts_with("http://") {
            url.replacen("http://", "https://", 1)
        } else {
            url.to_string()
        };

        // Check cache first
        let cached = {
            let mut cache = self.cache.lock().unwrap();
            cache.get(&url).and_then(|entry| {
                if !entry.is_expired() {
                    Some((entry.content.clone(), entry.content_type.clone()))
                } else {
                    None
                }
            })
        };
        if let Some((content, content_type)) = cached {
            let text = process_content(&content, &content_type);
            return finish(ctx, prompt, &url, text).await;
        }

        // Fetch
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| EngineError::Tool(format!("client error: {e}")))?;

        let response = client
            .get(&url)
            .header("User-Agent", "Arawn/0.1")
            .send()
            .await
            .map_err(|e| EngineError::Tool(format!("fetch error: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Ok(ToolOutput::error(format!("HTTP {status}")));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = response
            .text()
            .await
            .map_err(|e| EngineError::Tool(format!("read error: {e}")))?;

        // Cache the raw response
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(
                url.clone(),
                CacheEntry {
                    content: body.clone(),
                    content_type: content_type.clone(),
                    fetched_at: Instant::now(),
                },
            );
        }

        let text = process_content(&body, &content_type);
        finish(ctx, prompt, &url, text).await
    }
}

/// Convert HTML to markdown, or return non-HTML as-is. Truncate to MAX_CONTENT_BYTES.
fn process_content(body: &str, content_type: &str) -> String {
    let mut text = if content_type.contains("html") {
        html_to_markdown(body)
    } else {
        body.to_string()
    };

    if text.len() > MAX_CONTENT_BYTES {
        text.truncate(MAX_CONTENT_BYTES);
        text.push_str("\n\n[Content truncated due to length...]");
    }

    text
}

/// Convert HTML to markdown using htmd (Turndown-equivalent).
fn html_to_markdown(html: &str) -> String {
    htmd::convert(html).unwrap_or_else(|_| {
        // Fallback to basic tag stripping if htmd fails
        strip_html_tags(html)
    })
}

/// Fallback: simple HTML tag stripper (used if htmd fails).
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut last_was_whitespace = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                if !last_was_whitespace {
                    result.push(' ');
                    last_was_whitespace = true;
                }
            }
            _ if !in_tag => {
                if ch.is_whitespace() {
                    if !last_was_whitespace {
                        result.push(' ');
                        last_was_whitespace = true;
                    }
                } else {
                    result.push(ch);
                    last_was_whitespace = false;
                }
            }
            _ => {}
        }
    }

    result.trim().to_string()
}

/// If we have an LLM and a prompt, summarize. Otherwise return the content directly.
async fn finish(
    ctx: &ToolContext,
    prompt: &str,
    url: &str,
    text: String,
) -> Result<ToolOutput, EngineError> {
    if !prompt.is_empty()
        && let (Some(llm), Some(model)) = (ctx.llm(), ctx.model()) {
            return summarize_with_llm(llm, model, prompt, url, &text).await;
        }
    Ok(ToolOutput::success(text))
}

async fn summarize_with_llm(
    llm: &Arc<dyn arawn_llm::LlmClient>,
    model: &str,
    prompt: &str,
    url: &str,
    content: &str,
) -> Result<ToolOutput, EngineError> {
    let request = ChatRequest {
        model: model.to_string(),
        system_prompt: Some(
            "You are a web content extraction assistant. Given fetched web page content \
             and a user prompt, extract and return only the information the user asked for. \
             Be concise and accurate. Do not add commentary beyond what was requested."
                .to_string(),
        ),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(format!(
                "URL: {url}\n\n--- Page Content ---\n{content}\n--- End Content ---\n\n{prompt}"
            )),
            tool_calls: vec![],
            tool_call_id: None,
        }],
        tools: vec![],
        max_tokens: Some(4096),
    };

    let mut stream = llm
        .stream(request)
        .await
        .map_err(|e| EngineError::Tool(format!("LLM summarization error: {e}")))?;

    let mut result = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(arawn_llm::ChatChunk::TextDelta { text }) => result.push_str(&text),
            Ok(arawn_llm::ChatChunk::Done { .. }) => break,
            Err(e) => return Err(EngineError::Tool(format!("LLM stream error: {e}"))),
            _ => {}
        }
    }

    Ok(ToolOutput::success(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use arawn_core::Workstream;
    use arawn_llm::MockLlmClient;
    use arawn_llm::MockResponse;
    use uuid::Uuid;

    fn test_ctx() -> ToolContext {
        let ws = Workstream::scratch("/tmp/test");
        ToolContext::new(&ws, Uuid::new_v4())
    }

    fn test_ctx_with_mock(responses: Vec<MockResponse>) -> (ToolContext, Arc<MockLlmClient>) {
        let mock = Arc::new(MockLlmClient::new(responses));
        let ws = Workstream::scratch("/tmp/test");
        let ctx =
            ToolContext::new(&ws, Uuid::new_v4()).with_llm(mock.clone(), "test-model".to_string());
        (ctx, mock)
    }

    // --- HTML to markdown tests ---

    #[test]
    fn html_to_markdown_headings() {
        let md = html_to_markdown("<h1>Title</h1><p>Body text</p>");
        assert!(md.contains("Title"));
        assert!(md.contains("Body text"));
    }

    #[test]
    fn html_to_markdown_links() {
        let md = html_to_markdown(r#"<a href="https://example.com">Link</a>"#);
        assert!(md.contains("Link"));
        assert!(md.contains("https://example.com"));
    }

    #[test]
    fn html_to_markdown_lists() {
        let md = html_to_markdown("<ul><li>One</li><li>Two</li></ul>");
        assert!(md.contains("One"));
        assert!(md.contains("Two"));
    }

    #[test]
    fn html_to_markdown_code() {
        let md = html_to_markdown("<pre><code>fn main() {}</code></pre>");
        assert!(md.contains("fn main()"));
    }

    #[test]
    fn non_html_passthrough() {
        let text = process_content(r#"{"key": "value"}"#, "application/json");
        assert_eq!(text, r#"{"key": "value"}"#);
    }

    // --- Strip tags fallback ---

    #[test]
    fn strip_tags_basic() {
        assert_eq!(strip_html_tags("<p>hello</p>"), "hello");
    }

    #[test]
    fn strip_tags_collapses_whitespace() {
        assert_eq!(
            strip_html_tags("<div>  hello   world  </div>"),
            "hello world"
        );
    }

    // --- Cache tests ---

    #[test]
    fn cache_entry_expiry() {
        let fresh = CacheEntry {
            content: "test".into(),
            content_type: "text/html".into(),
            fetched_at: Instant::now(),
        };
        assert!(!fresh.is_expired());

        let expired = CacheEntry {
            content: "test".into(),
            content_type: "text/html".into(),
            fetched_at: Instant::now() - Duration::from_secs(16 * 60),
        };
        assert!(expired.is_expired());
    }

    #[test]
    fn cache_stores_and_retrieves() {
        let tool = WebFetchTool::new();
        {
            let mut cache = tool.cache.lock().unwrap();
            cache.put(
                "https://example.com".into(),
                CacheEntry {
                    content: "<h1>Cached</h1>".into(),
                    content_type: "text/html".into(),
                    fetched_at: Instant::now(),
                },
            );
        }
        {
            let mut cache = tool.cache.lock().unwrap();
            let entry = cache.get("https://example.com");
            assert!(entry.is_some());
            assert!(!entry.unwrap().is_expired());
        }
    }

    // --- Truncation ---

    #[test]
    fn large_content_truncated() {
        let big = "x".repeat(200_000);
        let result = process_content(&big, "text/plain");
        assert!(result.len() <= MAX_CONTENT_BYTES + 50); // +50 for the truncation notice
        assert!(result.contains("[Content truncated"));
    }

    // --- Schema ---

    #[test]
    fn schema_is_valid() {
        let tool = WebFetchTool::new();
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["url"].is_object());
        assert!(schema["properties"]["prompt"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("url")));
        assert!(required.contains(&json!("prompt")));
    }

    #[test]
    fn http_upgraded_description() {
        let tool = WebFetchTool::new();
        assert!(tool.description().contains("HTTPS"));
    }

    // --- LLM summarization tests ---

    #[tokio::test]
    async fn summarize_with_mock_llm() {
        let (ctx, mock) = test_ctx_with_mock(vec![MockResponse::text(
            "The page title is Example Domain.",
        )]);

        let result = summarize_with_llm(
            ctx.llm().unwrap(),
            ctx.model().unwrap(),
            "What is the title of this page?",
            "https://example.com",
            "# Example Domain\n\nThis domain is for use in illustrative examples.",
        )
        .await
        .unwrap();

        assert_eq!(result.content, "The page title is Example Domain.");
        assert!(!result.is_error);
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn summarize_sends_correct_request_shape() {
        let (ctx, mock) = test_ctx_with_mock(vec![MockResponse::text("extracted info")]);

        let result = summarize_with_llm(
            ctx.llm().unwrap(),
            ctx.model().unwrap(),
            "Find the price",
            "https://shop.example.com/item",
            "Widget — $29.99 — In stock",
        )
        .await
        .unwrap();

        assert_eq!(result.content, "extracted info");
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn execute_without_llm_returns_raw_text() {
        let ctx = test_ctx();
        assert!(ctx.llm().is_none());
    }

    #[tokio::test]
    async fn summarize_empty_content() {
        let (ctx, _mock) =
            test_ctx_with_mock(vec![MockResponse::text("The page appears to be empty.")]);

        let result = summarize_with_llm(
            ctx.llm().unwrap(),
            ctx.model().unwrap(),
            "Summarize",
            "https://example.com/empty",
            "",
        )
        .await
        .unwrap();

        assert_eq!(result.content, "The page appears to be empty.");
    }

    #[tokio::test]
    async fn summarize_multipart_response() {
        use arawn_llm::ChatChunk;

        let (ctx, _mock) = test_ctx_with_mock(vec![MockResponse::raw(vec![
            ChatChunk::TextDelta {
                text: "Part one. ".into(),
            },
            ChatChunk::TextDelta {
                text: "Part two.".into(),
            },
            ChatChunk::Done { usage: None },
        ])]);

        let result = summarize_with_llm(
            ctx.llm().unwrap(),
            ctx.model().unwrap(),
            "Summarize",
            "https://example.com",
            "Some content",
        )
        .await
        .unwrap();

        assert_eq!(result.content, "Part one. Part two.");
    }
}
