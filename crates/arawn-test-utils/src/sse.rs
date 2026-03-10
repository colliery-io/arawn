//! SSE (Server-Sent Events) test helpers.
//!
//! Utilities for collecting and inspecting SSE event streams
//! from the `/api/v1/chat/stream` endpoint.

use serde_json::Value;

/// A parsed SSE event.
#[derive(Debug, Clone)]
pub struct SseEvent {
    /// The event type (e.g., "session", "text", "tool_start", "done").
    pub event: String,
    /// The parsed JSON data payload.
    pub data: Value,
}

impl SseEvent {
    /// Check if this is a specific event type.
    pub fn is(&self, event_type: &str) -> bool {
        self.event == event_type
    }

    /// Get a string field from the data payload.
    pub fn get_str(&self, field: &str) -> Option<&str> {
        self.data.get(field).and_then(|v| v.as_str())
    }

    /// Get a bool field from the data payload.
    pub fn get_bool(&self, field: &str) -> Option<bool> {
        self.data.get(field).and_then(|v| v.as_bool())
    }
}

/// Collect all SSE events from a streaming response.
///
/// Parses the response body as SSE format and returns structured events.
pub async fn collect_sse_events(resp: reqwest::Response) -> Vec<SseEvent> {
    let text = resp.text().await.unwrap_or_default();
    parse_sse_text(&text)
}

/// Parse SSE text into events (useful for testing without HTTP).
pub fn parse_sse_text(text: &str) -> Vec<SseEvent> {
    let mut events = Vec::new();
    let mut current_event = String::new();
    let mut current_data = String::new();

    for line in text.lines() {
        if let Some(ev) = line.strip_prefix("event: ") {
            current_event = ev.to_string();
        } else if let Some(d) = line.strip_prefix("data: ") {
            current_data = d.to_string();
        } else if line.is_empty() && !current_event.is_empty() {
            if let Ok(data) = serde_json::from_str::<Value>(&current_data) {
                events.push(SseEvent {
                    event: current_event.clone(),
                    data,
                });
            }
            current_event.clear();
            current_data.clear();
        }
    }

    events
}

/// Reconstruct full text content from SSE text events.
pub fn reconstruct_text(events: &[SseEvent]) -> String {
    events
        .iter()
        .filter(|e| e.is("text"))
        .filter_map(|e| e.get_str("content"))
        .collect::<Vec<_>>()
        .join("")
}

/// Find all events of a specific type.
pub fn events_of_type<'a>(events: &'a [SseEvent], event_type: &str) -> Vec<&'a SseEvent> {
    events.iter().filter(|e| e.is(event_type)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_text() {
        let text = r#"event: session
data: {"session_id":"abc-123"}

event: text
data: {"content":"Hello"}

event: done
data: {"iterations":1}

"#;
        let events = parse_sse_text(text);
        assert_eq!(events.len(), 3);
        assert!(events[0].is("session"));
        assert_eq!(events[0].get_str("session_id"), Some("abc-123"));
        assert!(events[1].is("text"));
        assert_eq!(events[1].get_str("content"), Some("Hello"));
        assert!(events[2].is("done"));
    }

    #[test]
    fn test_reconstruct_text() {
        let events = vec![
            SseEvent {
                event: "session".to_string(),
                data: serde_json::json!({"session_id": "x"}),
            },
            SseEvent {
                event: "text".to_string(),
                data: serde_json::json!({"content": "Hello "}),
            },
            SseEvent {
                event: "text".to_string(),
                data: serde_json::json!({"content": "world!"}),
            },
            SseEvent {
                event: "done".to_string(),
                data: serde_json::json!({"iterations": 1}),
            },
        ];
        assert_eq!(reconstruct_text(&events), "Hello world!");
    }

    #[test]
    fn test_events_of_type() {
        let events = vec![
            SseEvent {
                event: "text".to_string(),
                data: serde_json::json!({"content": "a"}),
            },
            SseEvent {
                event: "tool_start".to_string(),
                data: serde_json::json!({"id": "t1"}),
            },
            SseEvent {
                event: "text".to_string(),
                data: serde_json::json!({"content": "b"}),
            },
        ];
        assert_eq!(events_of_type(&events, "text").len(), 2);
        assert_eq!(events_of_type(&events, "tool_start").len(), 1);
        assert_eq!(events_of_type(&events, "done").len(), 0);
    }
}
