use arawn_tool_plugin::{plugin_impl, ArawnTool, ToolExecuteOutput, __fidius_ArawnTool};

pub struct WebFetchTool;

#[plugin_impl(ArawnTool, crate = "arawn_tool_plugin::fidius")]
impl ArawnTool for WebFetchTool {
    fn name(&self) -> String {
        "web_fetch".to_string()
    }

    fn description(&self) -> String {
        "Fetch the contents of a URL and return as text. Strips HTML tags from HTML responses."
            .to_string()
    }

    fn parameters_schema(&self) -> String {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch"
                },
                "max_bytes": {
                    "type": "integer",
                    "description": "Maximum bytes to return (default: 102400)"
                }
            },
            "required": ["url"]
        })
        .to_string()
    }

    fn execute(&self, _context_json: String, params_json: String) -> ToolExecuteOutput {
        let params: serde_json::Value = match serde_json::from_str(&params_json) {
            Ok(v) => v,
            Err(e) => return ToolExecuteOutput::error(format!("invalid params: {e}")),
        };

        let url = match params.get("url").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return ToolExecuteOutput::error("missing 'url' parameter"),
        };

        let max_bytes = params
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(102_400) as usize;

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build();

        let client = match client {
            Ok(c) => c,
            Err(e) => return ToolExecuteOutput::error(format!("client error: {e}")),
        };

        let response = match client.get(url).send() {
            Ok(r) => r,
            Err(e) => return ToolExecuteOutput::error(format!("fetch error: {e}")),
        };

        let status = response.status();
        if !status.is_success() {
            return ToolExecuteOutput::error(format!("HTTP {status}"));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = match response.text() {
            Ok(t) => t,
            Err(e) => return ToolExecuteOutput::error(format!("read error: {e}")),
        };

        let mut text = if content_type.contains("html") {
            strip_html_tags(&body)
        } else {
            body
        };

        if text.len() > max_bytes {
            text.truncate(max_bytes);
            text.push_str("\n... (truncated)");
        }

        ToolExecuteOutput::success(text)
    }
}

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

arawn_tool_plugin::fidius::fidius_plugin_registry!();
