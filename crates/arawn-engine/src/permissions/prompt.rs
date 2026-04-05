use std::io::{self, BufRead, IsTerminal, Write};

use async_trait::async_trait;

use super::checker::{ModalPrompt, ModalRequest};

/// CLI-based modal prompt. Prints options to stderr, reads selection from stdin.
/// Falls back to None (cancel) when stdin is not a TTY (non-interactive mode).
pub struct CliModalPrompt;

impl Default for CliModalPrompt {
    fn default() -> Self {
        Self::new()
    }
}

impl CliModalPrompt {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ModalPrompt for CliModalPrompt {
    async fn prompt(&self, request: ModalRequest) -> Option<usize> {
        if !io::stdin().is_terminal() {
            eprintln!("[modal] {} — auto-cancelled (non-interactive)", request.title);
            return None;
        }

        eprintln!();
        eprintln!("─── {} ───", request.title);
        if let Some(ref subtitle) = request.subtitle {
            eprintln!("  {subtitle}");
        }
        eprintln!();
        for (i, opt) in request.options.iter().enumerate() {
            eprint!("  {}. {}", i + 1, opt.label);
            if let Some(ref desc) = opt.description {
                eprint!("  ({desc})");
            }
            eprintln!();
        }
        eprintln!();
        eprint!("  Select [1-{}] or 'c' to cancel: ", request.options.len());
        io::stderr().flush().ok();

        let mut input = String::new();
        if io::stdin().lock().read_line(&mut input).is_err() {
            return None;
        }

        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("c") {
            return None;
        }
        trimmed.parse::<usize>().ok().and_then(|n| {
            if n >= 1 && n <= request.options.len() {
                Some(n - 1)
            } else {
                None
            }
        })
    }
}

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
        let mock = MockModalPrompt::with_responses(
            vec![Some(0), Some(2), None],
            Some(1),
        );
        assert_eq!(mock.prompt(test_request()).await, Some(0));
        assert_eq!(mock.prompt(test_request()).await, Some(2));
        assert_eq!(mock.prompt(test_request()).await, None);
        // Queue exhausted — falls back to default
        assert_eq!(mock.prompt(test_request()).await, Some(1));
    }
}
