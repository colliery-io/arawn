//! Render assertion helpers for TUI tests.
//!
//! These functions inspect a `ratatui::backend::TestBackend` buffer
//! to verify that expected text is (or isn't) visible on screen.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

/// Extract all visible text from a terminal buffer, row by row.
pub fn buffer_to_string(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut lines = Vec::new();
    for y in area.y..area.y + area.height {
        let mut line = String::new();
        for x in area.x..area.x + area.width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        lines.push(line.trim_end().to_string());
    }
    lines.join("\n")
}

/// Check if the terminal buffer contains the given text substring.
pub fn buffer_contains_text(buffer: &Buffer, text: &str) -> bool {
    let rendered = buffer_to_string(buffer);
    rendered.contains(text)
}

/// Assert that the terminal buffer contains the given text.
///
/// Panics with a buffer dump on failure for easy debugging.
#[allow(dead_code)]
pub fn assert_rendered(terminal: &Terminal<TestBackend>, text: &str) {
    let buffer = terminal.backend().buffer().clone();
    if !buffer_contains_text(&buffer, text) {
        let rendered = buffer_to_string(&buffer);
        panic!(
            "Expected to find '{}' in rendered buffer.\n\nBuffer contents:\n{}",
            text, rendered
        );
    }
}

/// Assert that the terminal buffer does NOT contain the given text.
#[allow(dead_code)]
pub fn assert_not_rendered(terminal: &Terminal<TestBackend>, text: &str) {
    let buffer = terminal.backend().buffer().clone();
    if buffer_contains_text(&buffer, text) {
        let rendered = buffer_to_string(&buffer);
        panic!(
            "Expected NOT to find '{}' in rendered buffer.\n\nBuffer contents:\n{}",
            text, rendered
        );
    }
}
