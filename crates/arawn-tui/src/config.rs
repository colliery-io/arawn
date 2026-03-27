//! TUI configuration.

/// Configuration for the TUI application.
#[derive(Debug, Clone)]
pub struct TuiConfig {
    /// Server URL to connect to (e.g., "http://127.0.0.1:3000").
    pub server_url: String,
    /// Display name for the current context.
    pub context_name: Option<String>,
    /// Workstream to use.
    pub workstream: Option<String>,
}

impl TuiConfig {
    /// Create a new TUI config pointing at the given server.
    pub fn new(server_url: &str) -> Self {
        Self {
            server_url: server_url.to_string(),
            context_name: None,
            workstream: None,
        }
    }

    /// Derive the WebSocket URL from the server URL.
    pub fn ws_url(&self) -> String {
        let base = self.server_url.trim_end_matches('/');
        let ws_base = if base.starts_with("https://") {
            base.replacen("https://", "wss://", 1)
        } else {
            base.replacen("http://", "ws://", 1)
        };
        format!("{}/ws", ws_base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_url_from_http() {
        let cfg = TuiConfig::new("http://127.0.0.1:3000");
        assert_eq!(cfg.ws_url(), "ws://127.0.0.1:3000/ws");
    }

    #[test]
    fn ws_url_from_https() {
        let cfg = TuiConfig::new("https://example.com");
        assert_eq!(cfg.ws_url(), "wss://example.com/ws");
    }

    #[test]
    fn ws_url_strips_trailing_slash() {
        let cfg = TuiConfig::new("http://localhost:3000/");
        assert_eq!(cfg.ws_url(), "ws://localhost:3000/ws");
    }
}
