//! Chat view rendering with streaming support.

use super::theme;
use crate::app::{App, ChatMessage, ToolExecution};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

/// Streaming cursor indicator.
const STREAMING_CURSOR: &str = "▌";

/// Render the chat view with all messages.
pub fn render_chat(app: &mut App, frame: &mut Frame, area: Rect) {
    // If no messages, show welcome screen
    if app.messages.is_empty() {
        render_welcome(frame, area);
        return;
    }

    let mut lines = Vec::new();

    for (i, msg) in app.messages.iter().enumerate() {
        // Add spacing between messages (except before first)
        if i > 0 {
            lines.push(Line::from(""));
        }

        if msg.is_user {
            // User message with > prefix
            render_user_message(&mut lines, msg);
        } else {
            // Assistant message
            render_assistant_message(&mut lines, msg, area.width as usize);

            // Render tool executions after the current assistant message.
            // Tools are stored globally in app.tools and cleared when a new
            // user message is sent (see App::send_message). This means tools
            // always represent the current response cycle.
            //
            // Note: During rapid streaming updates, tools may briefly appear
            // unordered as ToolStart/ToolOutput/ToolEnd events arrive. This is
            // a visual timing artifact, not data corruption. Each tool maintains
            // its own state correctly via tool_id correlation.
            if msg.streaming || (i == app.messages.len() - 1 && !app.tools.is_empty()) {
                render_tools(&mut lines, &app.tools);
            }
        }
    }

    // Calculate scroll position
    let content_height = lines.len();
    let view_height = area.height as usize;

    let max_scroll = content_height.saturating_sub(view_height);
    let scroll_offset = if app.chat_auto_scroll {
        // Auto-scroll: show the bottom of the content
        max_scroll
    } else {
        // Manual scroll: clamp to valid range and write back to prevent overshoot
        let clamped = app.chat_scroll.min(max_scroll);
        app.chat_scroll = clamped;
        clamped
    };

    let chat = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset as u16, 0));

    frame.render_widget(chat, area);

    // Render scroll position indicator when not at the bottom
    if max_scroll > 0 && scroll_offset < max_scroll {
        let percent = (scroll_offset * 100) / max_scroll;
        let indicator = format!(" {}% ↓ ", percent);
        let indicator_width = indicator.len() as u16;
        let indicator_area = Rect::new(
            area.x + area.width.saturating_sub(indicator_width + 1),
            area.y,
            indicator_width,
            1,
        );
        let indicator_widget = Paragraph::new(Span::styled(indicator, theme::scroll_indicator()));
        frame.render_widget(indicator_widget, indicator_area);
    }
}

/// Render user message with > prefix.
fn render_user_message(lines: &mut Vec<Line<'static>>, msg: &ChatMessage) {
    let prefix = Span::styled("> ", theme::user_prefix());
    let content = Span::styled(msg.content.clone(), theme::user_text());
    lines.push(Line::from(vec![prefix, content]));
}

/// Render assistant message with word wrapping and streaming cursor.
fn render_assistant_message(lines: &mut Vec<Line<'static>>, msg: &ChatMessage, _width: usize) {
    let content = if msg.streaming {
        format!("{}{}", msg.content, STREAMING_CURSOR)
    } else {
        msg.content.clone()
    };

    // Split content by newlines and add each as a line
    for line_text in content.lines() {
        let style = if msg.streaming {
            theme::streaming_text()
        } else {
            theme::assistant_text()
        };
        lines.push(Line::from(Span::styled(line_text.to_string(), style)));
    }

    // Handle trailing newline or empty content
    if (content.ends_with('\n') || content.is_empty()) && msg.streaming {
        lines.push(Line::from(Span::styled(
            STREAMING_CURSOR.to_string(),
            theme::streaming_text(),
        )));
    }
}

/// Dotted separator character for tool display.
const TOOL_SEPARATOR: &str = "┄";

/// Render tool executions between messages.
fn render_tools(lines: &mut Vec<Line<'static>>, tools: &[ToolExecution]) {
    if tools.is_empty() {
        return;
    }

    lines.push(Line::from(""));

    for tool in tools {
        // Top separator
        lines.push(Line::from(Span::styled(
            TOOL_SEPARATOR.repeat(48),
            theme::separator(),
        )));

        // Build one-liner: [name] args... status duration
        let status_indicator = if tool.running {
            Span::styled("◐", Style::default().fg(theme::WARN))
        } else if tool.success == Some(true) {
            Span::styled("✓", Style::default().fg(theme::OK))
        } else {
            Span::styled("✗", Style::default().fg(theme::ERR))
        };

        let tool_name = Span::styled(format!("[{}]", tool.name), theme::tool_name());

        // Format args or output preview (truncated)
        let preview = if !tool.args.is_empty() {
            truncate_str(&tool.args, 30)
        } else if !tool.output.is_empty() {
            let first_line = tool.output.lines().next().unwrap_or("");
            truncate_str(first_line, 30)
        } else {
            String::new()
        };

        // Format duration
        let duration_str = if tool.running {
            String::new()
        } else if let Some(ms) = tool.duration_ms {
            format_duration(ms)
        } else {
            String::new()
        };

        // Build the line with proper spacing
        let mut spans = vec![tool_name, Span::raw(" ")];
        if !preview.is_empty() {
            spans.push(Span::styled(preview, theme::tool_preview()));
        }
        // Add spacing before status
        spans.push(Span::raw(" "));
        spans.push(status_indicator);
        if !duration_str.is_empty() {
            spans.push(Span::styled(
                format!(" {}", duration_str),
                theme::tool_duration(),
            ));
        }

        lines.push(Line::from(spans));

        // Bottom separator
        lines.push(Line::from(Span::styled(
            TOOL_SEPARATOR.repeat(48),
            theme::separator(),
        )));
    }
}

/// Truncate a string to max length, adding "..." if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Format duration in human-readable form.
fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let secs = ms / 1000;
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        format!("{}m{}s", mins, remaining_secs)
    }
}

/// Render the welcome screen when there are no messages.
fn render_welcome(frame: &mut Frame, area: Rect) {
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled("  Welcome to Arawn TUI", theme::header())),
        Line::from(""),
        Line::from("  Type a message and press Enter to send."),
        Line::from(""),
        Line::from(Span::styled("  Keyboard shortcuts:", theme::subheader())),
        Line::from(vec![
            Span::styled("    Ctrl+K  ", theme::key_hint()),
            Span::styled("Command palette", theme::key_desc()),
        ]),
        Line::from(vec![
            Span::styled("    Ctrl+S  ", theme::key_hint()),
            Span::styled("Sessions", theme::key_desc()),
        ]),
        Line::from(vec![
            Span::styled("    Ctrl+W  ", theme::key_hint()),
            Span::styled("Workstreams", theme::key_desc()),
        ]),
        Line::from(vec![
            Span::styled("    Ctrl+E  ", theme::key_hint()),
            Span::styled("Tool output pane", theme::key_desc()),
        ]),
        Line::from(vec![
            Span::styled("    Ctrl+Q  ", theme::key_hint()),
            Span::styled("Quit", theme::key_desc()),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Chat navigation:", theme::subheader())),
        Line::from(vec![
            Span::styled("    ↑/↓     ", theme::key_hint()),
            Span::styled("Scroll chat history", theme::key_desc()),
        ]),
        Line::from(vec![
            Span::styled("    PgUp    ", theme::key_hint()),
            Span::styled("Scroll up one page", theme::key_desc()),
        ]),
        Line::from(vec![
            Span::styled("    PgDn    ", theme::key_hint()),
            Span::styled("Scroll down one page", theme::key_desc()),
        ]),
        Line::from(vec![
            Span::styled("    Home    ", theme::key_hint()),
            Span::styled("Scroll to top", theme::key_desc()),
        ]),
        Line::from(vec![
            Span::styled("    End     ", theme::key_hint()),
            Span::styled("Scroll to bottom (auto-scroll)", theme::key_desc()),
        ]),
    ])
    .wrap(Wrap { trim: false });

    frame.render_widget(content, area);
}
