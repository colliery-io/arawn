use async_trait::async_trait;
use serde_json::{Value, json};

use crate::context::ToolContext;
use crate::error::EngineError;
use crate::tool::{Tool, ToolCategory, ToolOutput};

/// Searches the web and returns results to inform responses.
pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web using DuckDuckGo and return results with titles, URLs, and snippets. \
         Use this for discovering information on the internet — NOT for searching local files (use grep instead). \
         Use `allowed_domains` to restrict results to specific sites, or \
         `blocked_domains` to exclude sites. \
         If a web_search returns 'No results found', try rephrasing the query or broadening the search. \
         Do not repeat the exact same query more than once."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "minLength": 2,
                    "description": "The search query to use"
                },
                "allowed_domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Only include search results from these domains"
                },
                "blocked_domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Never include search results from these domains"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Tool("missing 'query' parameter".into()))?;

        let allowed_domains: Vec<String> = params
            .get("allowed_domains")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let blocked_domains: Vec<String> = params
            .get("blocked_domains")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Append site: restriction to query if allowed_domains specified
        let effective_query = if !allowed_domains.is_empty() {
            let site_clause = allowed_domains
                .iter()
                .map(|d| format!("site:{d}"))
                .collect::<Vec<_>>()
                .join(" OR ");
            format!("{query} ({site_clause})")
        } else {
            query.to_string()
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| EngineError::Tool(format!("client error: {e}")))?;

        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencod(&effective_query)
        );

        let response = client
            .get(&url)
            .header("User-Agent", "Arawn/0.1")
            .send()
            .await
            .map_err(|e| EngineError::Tool(format!("search error: {e}")))?;

        let body = response
            .text()
            .await
            .map_err(|e| EngineError::Tool(format!("read error: {e}")))?;

        let mut results = parse_ddg_results(&body, 10);

        // Filter out blocked domains
        if !blocked_domains.is_empty() {
            results.retain(|r| !blocked_domains.iter().any(|d| r.url.contains(d.as_str())));
        }

        if results.is_empty() {
            return Ok(ToolOutput::success("No results found."));
        }

        let mut output = String::new();
        for (i, result) in results.iter().enumerate() {
            output.push_str(&format!(
                "{}. {}\n   {}\n   {}\n\n",
                i + 1,
                result.title,
                result.url,
                result.snippet
            ));
        }

        Ok(ToolOutput::success(output.trim()))
    }
}

struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

fn parse_ddg_results(html: &str, max: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();

    for block in html.split("class=\"result__a\"") {
        if results.len() >= max {
            break;
        }
        if block.contains("class=\"result__snippet\"") {
            let title = extract_tag_content(block, ">");
            let url = extract_href(block);
            let snippet = extract_after_class(block, "result__snippet");

            if !title.is_empty() && !url.is_empty() {
                results.push(SearchResult {
                    title: strip_tags(&title),
                    url,
                    snippet: strip_tags(&snippet),
                });
            }
        }
    }

    results
}

fn extract_tag_content(html: &str, after: &str) -> String {
    if let Some(start) = html.find(after) {
        let rest = &html[start + after.len()..];
        if let Some(end) = rest.find('<') {
            return rest[..end].trim().to_string();
        }
    }
    String::new()
}

fn extract_href(html: &str) -> String {
    if let Some(start) = html.find("href=\"") {
        let rest = &html[start + 6..];
        if let Some(end) = rest.find('"') {
            let url = &rest[..end];
            if let Some(udurl) = url.find("uddg=") {
                let decoded = &url[udurl + 5..];
                return urldecod(decoded);
            }
            return url.to_string();
        }
    }
    String::new()
}

fn extract_after_class(html: &str, class: &str) -> String {
    let pattern = format!("class=\"{}\"", class);
    if let Some(start) = html.find(&pattern) {
        let rest = &html[start + pattern.len()..];
        if let Some(tag_end) = rest.find('>') {
            let content = &rest[tag_end + 1..];
            if let Some(end) = content.find("</") {
                return content[..end].trim().to_string();
            }
        }
    }
    String::new()
}

fn strip_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result.trim().to_string()
}

fn urlencod(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

fn urldecod(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else if c == '&' {
            break;
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencod_spaces() {
        assert_eq!(urlencod("hello world"), "hello+world");
    }

    #[test]
    fn urlencod_special_chars() {
        assert_eq!(urlencod("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn urldecod_percent() {
        assert_eq!(urldecod("hello%20world"), "hello world");
    }

    #[test]
    fn urldecod_stops_at_ampersand() {
        assert_eq!(urldecod("hello&extra=1"), "hello");
    }

    #[test]
    fn urldecod_plus_to_space() {
        assert_eq!(urldecod("hello+world"), "hello world");
    }

    #[test]
    fn strip_tags_removes_html() {
        assert_eq!(strip_tags("<b>bold</b> text"), "bold text");
    }

    #[test]
    fn strip_tags_empty() {
        assert_eq!(strip_tags(""), "");
    }

    #[test]
    fn schema_is_valid() {
        let tool = WebSearchTool;
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["properties"]["allowed_domains"].is_object());
        assert!(schema["properties"]["blocked_domains"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("query")));
    }

    #[test]
    fn parse_ddg_results_empty_html() {
        let results = parse_ddg_results("", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn parse_ddg_results_no_results() {
        let html = "<html><body>No results</body></html>";
        let results = parse_ddg_results(html, 5);
        assert!(results.is_empty());
    }

    #[test]
    fn parse_ddg_results_respects_max() {
        // Simulate two result blocks
        let html = r#"
            class="result__a" href="https://a.com">Title A</a>
            class="result__snippet">Snippet A</span>
            class="result__a" href="https://b.com">Title B</a>
            class="result__snippet">Snippet B</span>
        "#;
        let results = parse_ddg_results(html, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Title A");
    }

    #[test]
    fn parse_ddg_results_extracts_fields() {
        let html = r#"
            class="result__a" href="https://example.com/page">Example Page</a>
            class="result__snippet">This is a snippet about examples</span>
        "#;
        let results = parse_ddg_results(html, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Example Page");
        assert_eq!(results[0].url, "https://example.com/page");
        assert!(results[0].snippet.contains("snippet about examples"));
    }

    #[test]
    fn blocked_domains_filter() {
        let mut results = vec![
            SearchResult {
                title: "Good".into(),
                url: "https://good.com/page".into(),
                snippet: "ok".into(),
            },
            SearchResult {
                title: "Bad".into(),
                url: "https://bad.com/page".into(),
                snippet: "blocked".into(),
            },
            SearchResult {
                title: "Also Good".into(),
                url: "https://also-good.com/page".into(),
                snippet: "ok".into(),
            },
        ];

        let blocked = vec!["bad.com".to_string()];
        results.retain(|r| !blocked.iter().any(|d| r.url.contains(d.as_str())));

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Good");
        assert_eq!(results[1].title, "Also Good");
    }

    #[test]
    fn allowed_domains_builds_site_clause() {
        let allowed = vec!["rust-lang.org".to_string(), "docs.rs".to_string()];
        let query = "async trait";
        let site_clause = allowed
            .iter()
            .map(|d| format!("site:{d}"))
            .collect::<Vec<_>>()
            .join(" OR ");
        let effective = format!("{query} ({site_clause})");
        assert_eq!(
            effective,
            "async trait (site:rust-lang.org OR site:docs.rs)"
        );
    }

    #[test]
    fn is_read_only() {
        let tool = WebSearchTool;
        assert!(tool.is_read_only());
    }
}
