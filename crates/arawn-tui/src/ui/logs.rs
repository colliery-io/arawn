//! Logs panel rendering.

use super::theme;
use crate::logs::LogBuffer;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Render the logs panel.
pub fn render_logs_panel(log_buffer: &LogBuffer, scroll: usize, frame: &mut Frame, area: Rect) {
    let entries = log_buffer.entries();

    let block = Block::default()
        .title(format!(" logs ({}) ", entries.len()))
        .borders(Borders::ALL)
        .border_style(theme::border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if entries.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No log entries yet...",
            theme::empty_state(),
        )));
        frame.render_widget(empty, inner);
        return;
    }

    // Build log lines
    let mut lines: Vec<Line> = Vec::with_capacity(entries.len());
    for entry in &entries {
        // Format: [LEVEL] target: message
        let level_span = Span::styled(
            format!("[{}]", entry.level_prefix()),
            Style::default()
                .fg(entry.level_color())
                .add_modifier(Modifier::BOLD),
        );

        // Shorten target to last component for readability
        let target_short = entry.target.rsplit("::").next().unwrap_or(&entry.target);
        let target_span = Span::styled(format!(" {}: ", target_short), theme::list_item_dim());

        let message_span = Span::styled(&entry.message, Style::default());

        lines.push(Line::from(vec![level_span, target_span, message_span]));
    }

    // Calculate scroll bounds
    let visible_height = inner.height as usize;
    let total_lines = lines.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let actual_scroll = scroll.min(max_scroll);

    // Take visible slice
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(actual_scroll)
        .take(visible_height)
        .collect();

    let logs = Paragraph::new(visible_lines).wrap(Wrap { trim: false });

    frame.render_widget(logs, inner);
}

/// Render the logs footer with keyboard hints.
pub fn render_logs_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("  ↑↓", theme::key_hint()),
        Span::styled(" scroll", theme::key_desc()),
        Span::styled(" │ ", theme::separator()),
        Span::styled("^C", theme::key_hint()),
        Span::styled(" clear", theme::key_desc()),
        Span::styled(" │ ", theme::separator()),
        Span::styled("esc", theme::key_hint()),
        Span::styled(" close", theme::key_desc()),
    ]));
    frame.render_widget(footer, area);
}
