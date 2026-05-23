//! Modal-prompt implementations of `ModalPrompt`.
//!
//! Production wires `ChannelModalPrompt` (in `arawn/src/channel_prompt.rs`)
//! that bridges the WS event loop. Tests use `MockModalPrompt`. A
//! `CliModalPrompt` used to live here for a stdio-driven flow but was
//! removed in the YAGNI pass — nothing constructed it.

use async_trait::async_trait;

use super::checker::{ModalPrompt, ModalRequest};

/// Mock modal prompt for tests. Returns responses from a queue, or a default.
pub struct MockModalPrompt {
    responses: std::sync::Mutex<std::collections::VecDeque<Option<usize>>>,
    default: Option<usize>,
}

impl MockModalPrompt {
    /// Create a mock that always returns the given index.
    pub fn always(index: Option<usize>) -> Self {
        Self {
            responses: std::sync::Mutex::new(std::collections::VecDeque::new()),
            default: index,
        }
    }

    /// Create a mock with queued responses.
    pub fn with_responses(responses: Vec<Option<usize>>, default: Option<usize>) -> Self {
        Self {
            responses: std::sync::Mutex::new(responses.into()),
            default,
        }
    }
}

#[async_trait]
impl ModalPrompt for MockModalPrompt {
    async fn prompt(&self, _request: ModalRequest) -> Option<usize> {
        let mut queue = self.responses.lock().unwrap();
        queue.pop_front().unwrap_or(self.default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::checker::ModalOption;

    fn test_request() -> ModalRequest {
        ModalRequest {
            title: "Test".into(),
            subtitle: None,
            options: vec![
                ModalOption::new("Option A"),
                ModalOption::new("Option B"),
                ModalOption::new("Option C"),
            ],
        }
    }

    #[tokio::test]
    async fn mock_always_returns_index() {
        let mock = MockModalPrompt::always(Some(1));
        assert_eq!(mock.prompt(test_request()).await, Some(1));
        assert_eq!(mock.prompt(test_request()).await, Some(1));
    }

    #[tokio::test]
    async fn mock_always_cancel() {
        let mock = MockModalPrompt::always(None);
        assert_eq!(mock.prompt(test_request()).await, None);
    }

    #[tokio::test]
    async fn mock_queued_responses() {
        let mock = MockModalPrompt::with_responses(vec![Some(0), Some(2), None], Some(1));
        assert_eq!(mock.prompt(test_request()).await, Some(0));
        assert_eq!(mock.prompt(test_request()).await, Some(2));
        assert_eq!(mock.prompt(test_request()).await, None);
        // Queue exhausted — falls back to default
        assert_eq!(mock.prompt(test_request()).await, Some(1));
    }
}
