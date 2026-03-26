use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use url::Url;

use arawn_agent::Result;
use arawn_agent::tool::{GatedParam, Tool, ToolContext, ToolResult};

use super::validate_url_not_ssrf;

/// Configuration for web fetching.
#[derive(Debug, Clone)]
pub struct WebFetchConfig {
    /// Request timeout.
    pub timeout: Duration,
    /// Maximum response size in bytes.
    pub max_size: usize,
    /// User agent string.
    pub user_agent: String,
    /// Whether to extract text from HTML.
    pub extract_text: bool,
    /// Maximum text length to return.
    pub max_text_length: usize,
}

impl Default for WebFetchConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_size: 10 * 1024 * 1024, // 10MB for in-memory responses
            user_agent: concat!("Arawn/", env!("CARGO_PKG_VERSION"), " (Research Agent)")
                .to_string(),
            extract_text: true,
            max_text_length: 50_000,
        }
    }
}

/// Tool for fetching web page content.
#[derive(Debug, Clone)]
pub struct WebFetchTool {
    client: Client,
    pub(crate) config: WebFetchConfig,
}

impl WebFetchTool {
    /// Create a new web fetch tool with default configuration.
    pub fn new() -> std::result::Result<Self, reqwest::Error> {
        Self::with_config(WebFetchConfig::default())
    }

    /// Create a web fetch tool with custom configuration.
    pub fn with_config(config: WebFetchConfig) -> std::result::Result<Self, reqwest::Error> {
        let client = Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .build()?;

        Ok(Self { client, config })
    }

    /// Extract readable text from HTML.
    pub(crate) fn extract_text_from_html(&self, html: &str) -> String {
        let document = Html::parse_document(html);

        // Remove script and style elements
        let mut text_parts = Vec::new();

        // Try to get main content areas first
        let content_selectors = [
            "article",
            "main",
            "[role='main']",
            ".content",
            "#content",
            ".post-content",
            ".entry-content",
        ];

        let mut found_content = false;
        for selector_str in content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    if !text.trim().is_empty() {
                        text_parts.push(text);
                        found_content = true;
                    }
                }
            }
            if found_content {
                break;
            }
        }

        // Fall back to body if no content areas found
        if !found_content && let Ok(body_selector) = Selector::parse("body") {
            for element in document.select(&body_selector) {
                // Skip script, style, nav, footer elements
                let text = element.text().collect::<Vec<_>>().join(" ");
                text_parts.push(text);
            }
        }

        // Clean up the text
        let text = text_parts.join("\n\n");
        let text = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        // Collapse multiple whitespace
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");

        // Truncate if needed
        if text.len() > self.config.max_text_length {
            format!("{}...[truncated]", &text[..self.config.max_text_length])
        } else {
            text
        }
    }

    /// Extract title from HTML.
    pub(crate) fn extract_title(&self, html: &str) -> Option<String> {
        let document = Html::parse_document(html);
        if let Ok(selector) = Selector::parse("title") {
            document
                .select(&selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
        } else {
            None
        }
    }

    /// Extract meta description from HTML.
    pub(crate) fn extract_description(&self, html: &str) -> Option<String> {
        let document = Html::parse_document(html);
        if let Ok(selector) = Selector::parse("meta[name='description']") {
            document
                .select(&selector)
                .next()
                .and_then(|el| el.value().attr("content").map(|s| s.to_string()))
        } else {
            None
        }
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            tracing::error!("failed to build default HTTP client: {e}");
            // Construct with a bare client — requests will fail at call time
            // rather than panicking the entire server at startup
            Self {
                client: reqwest::Client::new(),
                config: Default::default(),
            }
        })
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL. Supports all HTTP methods, custom headers, and request bodies. Returns the page content, status code, and metadata."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch"
                },
                "method": {
                    "type": "string",
                    "description": "HTTP method to use. Defaults to GET.",
                    "enum": ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"],
                    "default": "GET"
                },
                "headers": {
                    "type": "object",
                    "description": "Custom request headers as key-value pairs",
                    "additionalProperties": { "type": "string" }
                },
                "body": {
                    "type": "string",
                    "description": "Request body (for POST, PUT, PATCH). Can be JSON string or plain text."
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Request timeout in seconds. Defaults to 30.",
                    "minimum": 1,
                    "maximum": 300
                },
                "raw": {
                    "type": "boolean",
                    "description": "If true, return raw HTML instead of extracted text. Defaults to false.",
                    "default": false
                },
                "include_headers": {
                    "type": "boolean",
                    "description": "If true, include response headers in the result. Defaults to false.",
                    "default": false
                },
                "download": {
                    "type": "string",
                    "description": "File path to save the response body to. Streams directly to disk, bypassing size limits. Returns file metadata instead of content. Useful for binary files (images, PDFs, etc.)."
                }
            },
            "required": ["url"]
        })
    }

    fn gated_params(&self) -> Vec<GatedParam> {
        vec![GatedParam::WritePath("download")]
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult> {
        if ctx.is_cancelled() {
            return Ok(ToolResult::error("Operation cancelled"));
        }

        let url_str = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| arawn_agent::AgentError::Tool("Missing 'url' parameter".to_string()))?;

        let method = params
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let raw = params.get("raw").and_then(|v| v.as_bool()).unwrap_or(false);
        let include_headers = params
            .get("include_headers")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let custom_headers = params.get("headers").and_then(|v| v.as_object());
        let body = params.get("body").and_then(|v| v.as_str());
        let timeout_secs = params.get("timeout_secs").and_then(|v| v.as_u64());
        let download_path = params.get("download").and_then(|v| v.as_str());

        // Validate URL
        let url = match Url::parse(url_str) {
            Ok(u) => u,
            Err(e) => return Ok(ToolResult::error(format!("Invalid URL: {}", e))),
        };

        // Only allow http/https
        if url.scheme() != "http" && url.scheme() != "https" {
            return Ok(ToolResult::error("Only HTTP and HTTPS URLs are supported"));
        }

        // SSRF protection: reject private/loopback/link-local/cloud-metadata IPs
        if let Err(msg) = validate_url_not_ssrf(&url).await {
            return Ok(ToolResult::error(msg));
        }

        // Build the request with the appropriate method
        let mut request = match method.as_str() {
            "GET" => self.client.get(url.as_str()),
            "POST" => self.client.post(url.as_str()),
            "PUT" => self.client.put(url.as_str()),
            "PATCH" => self.client.patch(url.as_str()),
            "DELETE" => self.client.delete(url.as_str()),
            "HEAD" => self.client.head(url.as_str()),
            "OPTIONS" => self.client.request(reqwest::Method::OPTIONS, url.as_str()),
            _ => {
                return Ok(ToolResult::error(format!(
                    "Unsupported HTTP method: {}",
                    method
                )));
            }
        };

        // Add custom headers
        if let Some(headers) = custom_headers {
            for (key, value) in headers {
                if let Some(val_str) = value.as_str() {
                    request = request.header(key.as_str(), val_str);
                }
            }
        }

        // Add request body
        if let Some(body_str) = body {
            request = request.body(body_str.to_string());
        }

        // Apply custom timeout if specified
        if let Some(secs) = timeout_secs {
            request = request.timeout(Duration::from_secs(secs));
        }

        // Send the request
        let response = match request.send().await {
            Ok(r) => r,
            Err(e) => return Ok(ToolResult::error(format!("Failed to fetch URL: {}", e))),
        };

        let status = response.status();
        let status_code = status.as_u16();
        let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();

        // Capture response headers if requested
        let response_headers: Option<serde_json::Map<String, Value>> = if include_headers {
            Some(
                response
                    .headers()
                    .iter()
                    .filter_map(|(name, value)| {
                        value.to_str().ok().map(|v| (name.to_string(), json!(v)))
                    })
                    .collect(),
            )
        } else {
            None
        };

        // For non-success status, still return the response but include error info
        let is_error = !status.is_success();

        // Get content type and content length
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();

        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        // Handle download to file - stream directly to disk, bypassing size limits
        if let Some(path_str) = download_path {
            let path = Path::new(path_str);

            // Reject path traversal attempts (e.g., ../../etc/passwd)
            for component in path.components() {
                if matches!(component, std::path::Component::ParentDir) {
                    return Ok(ToolResult::error(format!(
                        "Path traversal not allowed in download path: {}",
                        path.display()
                    )));
                }
            }

            // FsGate validation: check if the download path is allowed
            if let Some(gate) = &ctx.fs_gate
                && let Err(e) = gate.validate_write(path)
            {
                return Ok(ToolResult::error(format!(
                    "Download path denied by filesystem gate: {}",
                    e
                )));
            }

            // Create parent directories if needed
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
                && let Err(e) = tokio::fs::create_dir_all(parent).await
            {
                return Ok(ToolResult::error(format!(
                    "Failed to create directory: {}",
                    e
                )));
            }

            // Stream response body to file
            let mut file = match tokio::fs::File::create(path).await {
                Ok(f) => f,
                Err(e) => {
                    return Ok(ToolResult::error(format!("Failed to create file: {}", e)));
                }
            };

            let mut bytes_written: u64 = 0;
            let mut stream = response.bytes_stream();
            use futures::StreamExt;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        bytes_written += chunk.len() as u64;
                        if let Err(e) = file.write_all(&chunk).await {
                            return Ok(ToolResult::error(format!(
                                "Failed to write to file: {}",
                                e
                            )));
                        }
                    }
                    Err(e) => {
                        return Ok(ToolResult::error(format!(
                            "Failed to read response stream: {}",
                            e
                        )));
                    }
                }
            }

            if let Err(e) = file.flush().await {
                return Ok(ToolResult::error(format!("Failed to flush file: {}", e)));
            }

            // Return metadata about the downloaded file
            let mut result = json!({
                "url": url_str,
                "method": method,
                "status": status_code,
                "status_text": status_text,
                "downloaded": true,
                "path": path_str,
                "size": bytes_written,
                "content_type": content_type
            });
            if let Some(expected_size) = content_length {
                result["expected_size"] = json!(expected_size);
            }
            if is_error {
                result["error"] = json!(true);
            }
            if let Some(headers) = response_headers {
                result["headers"] = json!(headers);
            }
            return Ok(ToolResult::json(result));
        }

        // For HEAD requests, we don't read the body
        if method == "HEAD" {
            let mut result = json!({
                "url": url_str,
                "method": method,
                "status": status_code,
                "status_text": status_text,
                "content_type": content_type
            });
            if let Some(headers) = response_headers {
                result["headers"] = json!(headers);
            }
            return Ok(ToolResult::json(result));
        }

        // Check if response will exceed size limit - if so, auto-download to temp file
        let auto_download_path = if let Some(size) = content_length {
            if size > self.config.max_size as u64 {
                // Generate temp file path based on URL
                let filename = url
                    .path_segments()
                    .and_then(|mut s| s.next_back())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("download");
                let temp_path = std::env::temp_dir().join("arawn_downloads").join(format!(
                    "{}_{}",
                    uuid::Uuid::new_v4(),
                    filename
                ));
                Some(temp_path)
            } else {
                None
            }
        } else {
            None
        };

        // If we need to auto-download due to size, stream to temp file
        if let Some(temp_path) = auto_download_path {
            // Create parent directories
            if let Some(parent) = temp_path.parent()
                && let Err(e) = tokio::fs::create_dir_all(parent).await
            {
                return Ok(ToolResult::error(format!(
                    "Failed to create temp directory: {}",
                    e
                )));
            }

            let mut file = match tokio::fs::File::create(&temp_path).await {
                Ok(f) => f,
                Err(e) => {
                    return Ok(ToolResult::error(format!(
                        "Failed to create temp file: {}",
                        e
                    )));
                }
            };

            let mut bytes_written: u64 = 0;
            let mut stream = response.bytes_stream();
            use futures::StreamExt;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        bytes_written += chunk.len() as u64;
                        if let Err(e) = file.write_all(&chunk).await {
                            return Ok(ToolResult::error(format!(
                                "Failed to write to temp file: {}",
                                e
                            )));
                        }
                    }
                    Err(e) => {
                        return Ok(ToolResult::error(format!(
                            "Failed to read response stream: {}",
                            e
                        )));
                    }
                }
            }

            if let Err(e) = file.flush().await {
                return Ok(ToolResult::error(format!(
                    "Failed to flush temp file: {}",
                    e
                )));
            }

            let path_str = temp_path.display().to_string();
            let mut result = json!({
                "url": url_str,
                "method": method,
                "status": status_code,
                "status_text": status_text,
                "downloaded": true,
                "auto_downloaded": true,
                "reason": format!("Response exceeded {} byte limit", self.config.max_size),
                "path": path_str,
                "size": bytes_written,
                "content_type": content_type
            });
            if let Some(expected_size) = content_length {
                result["expected_size"] = json!(expected_size);
            }
            if is_error {
                result["error"] = json!(true);
            }
            if let Some(headers) = response_headers {
                result["headers"] = json!(headers);
            }
            return Ok(ToolResult::json(result));
        }

        // Read body with size limit - fallback to auto-download if exceeded during read
        let bytes = match response.bytes().await {
            Ok(b) => {
                if b.len() > self.config.max_size {
                    // Content-Length wasn't provided but response is too large
                    // Save to temp file and return that
                    let filename = url
                        .path_segments()
                        .and_then(|mut s| s.next_back())
                        .filter(|s| !s.is_empty())
                        .unwrap_or("download");
                    let temp_path = std::env::temp_dir().join("arawn_downloads").join(format!(
                        "{}_{}",
                        uuid::Uuid::new_v4(),
                        filename
                    ));

                    if let Some(parent) = temp_path.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }

                    if let Err(e) = tokio::fs::write(&temp_path, &b).await {
                        return Ok(ToolResult::error(format!(
                            "Response too large ({} bytes) and failed to save to temp file: {}",
                            b.len(),
                            e
                        )));
                    }

                    let path_str = temp_path.display().to_string();
                    let mut result = json!({
                        "url": url_str,
                        "method": method,
                        "status": status_code,
                        "status_text": status_text,
                        "downloaded": true,
                        "auto_downloaded": true,
                        "reason": format!("Response exceeded {} byte limit", self.config.max_size),
                        "path": path_str,
                        "size": b.len(),
                        "content_type": content_type
                    });
                    if is_error {
                        result["error"] = json!(true);
                    }
                    if let Some(headers) = response_headers {
                        result["headers"] = json!(headers);
                    }
                    return Ok(ToolResult::json(result));
                }
                b
            }
            Err(e) => return Ok(ToolResult::error(format!("Failed to read response: {}", e))),
        };

        let response_body = String::from_utf8_lossy(&bytes).to_string();

        // Build base result with status info
        let build_result = |content: Value, extra: Option<serde_json::Map<String, Value>>| {
            let mut result = json!({
                "url": url_str,
                "method": method,
                "status": status_code,
                "status_text": status_text,
                "content_type": content_type,
                "content": content
            });
            if is_error {
                result["error"] = json!(true);
            }
            if let Some(headers) = &response_headers {
                result["headers"] = json!(headers);
            }
            if let Some(extra_fields) = extra {
                for (k, v) in extra_fields {
                    result[k] = v;
                }
            }
            result
        };

        // Return raw HTML if requested
        if raw {
            return Ok(ToolResult::json(build_result(json!(response_body), None)));
        }

        // Extract text from HTML
        if content_type.contains("text/html") {
            let title = self.extract_title(&response_body);
            let description = self.extract_description(&response_body);
            let text = self.extract_text_from_html(&response_body);

            let mut extra = serde_json::Map::new();
            if let Some(t) = title {
                extra.insert("title".to_string(), json!(t));
            }
            if let Some(d) = description {
                extra.insert("description".to_string(), json!(d));
            }

            Ok(ToolResult::json(build_result(json!(text), Some(extra))))
        } else {
            // Return raw content for non-HTML
            let truncated = if response_body.len() > self.config.max_text_length {
                format!(
                    "{}...[truncated]",
                    &response_body[..self.config.max_text_length]
                )
            } else {
                response_body
            };

            Ok(ToolResult::json(build_result(json!(truncated), None)))
        }
    }
}
