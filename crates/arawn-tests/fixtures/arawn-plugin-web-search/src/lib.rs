use arawn_tool_plugin::{plugin_impl, ArawnTool, ToolExecuteOutput, __fidius_ArawnTool};

pub struct WebSearchTool;

#[plugin_impl(ArawnTool, crate = "arawn_tool_plugin::fidius")]
impl ArawnTool for WebSearchTool {
    fn name(&self) -> String {
        "web_search".to_string()
    }

    fn description(&self) -> String {
        "Search the web using DuckDuckGo and return results with titles, URLs, and snippets."
            .to_string()
    }

    fn parameters_schema(&self) -> String {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 5)"
                }
            },
            "required": ["query"]
        })
        .to_string()
    }

    fn execute(&self, _context_json: String, params_json: String) -> ToolExecuteOutput {
        let params: serde_json::Value = match serde_json::from_str(&params_json) {
            Ok(v) => v,
            Err(e) => return ToolExecuteOutput::error(format!("invalid params: {e}")),
        };

        let query = match params.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return ToolExecuteOutput::error("missing 'query' parameter"),
        };

        let num_results = params
            .get("num_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build();

        let client = match client {
            Ok(c) => c,
            Err(e) => return ToolExecuteOutput::error(format!("client error: {e}")),
        };

        // Use DuckDuckGo HTML search (no API key needed)
        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencod(query)
        );

        let response = match client
            .get(&url)
            .header("User-Agent", "Arawn/0.1")
            .send()
        {
            Ok(r) => r,
            Err(e) => return ToolExecuteOutput::error(format!("search error: {e}")),
        };

        let body = match response.text() {
            Ok(t) => t,
            Err(e) => return ToolExecuteOutput::error(format!("read error: {e}")),
        };

        let results = parse_ddg_results(&body, num_results);

        if results.is_empty() {
            return ToolExecuteOutput::success("No results found.");
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

        ToolExecuteOutput::success(output.trim())
    }
}

struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

fn parse_ddg_results(html: &str, max: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // DuckDuckGo HTML results are in <a class="result__a"> tags
    // with snippets in <a class="result__snippet">
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
            // DuckDuckGo wraps URLs in redirects — extract the actual URL
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
            // Stop at query string boundaries
            break;
        } else {
            result.push(c);
        }
    }
    result
}

arawn_tool_plugin::fidius::fidius_plugin_registry!();
