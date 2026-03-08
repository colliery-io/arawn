//! Tool output pane rendering.

use super::theme;
use crate::app::{App, ToolExecution};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Render the tool output pane (split view at bottom of screen).
pub fn render_tool_pane(app: &App, frame: &mut Frame, area: Rect) {
    let is_focused = app.focus.is(crate::focus::FocusTarget::ToolPane);

    // Build title with tool selector
    let title = build_title(app);

    let border_style = if is_focused {
        theme::border_focused()
    } else {
        theme::border()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Render tool output or placeholder
    if let Some(tool) = get_selected_tool(app) {
        render_tool_output(tool, app.tool_scroll, frame, inner);
    } else if app.tools.is_empty() {
        render_no_tools(frame, inner);
    } else {
        render_no_selection(frame, inner);
    }
}

/// Build the title line with tool selector.
fn build_title(app: &App) -> Line<'static> {
    if app.tools.is_empty() {
        return Line::from(Span::styled(" tools ", theme::header()));
    }

    let mut spans = vec![Span::raw(" ")];

    for (i, tool) in app.tools.iter().enumerate() {
        let is_selected = app.selected_tool_index == Some(i);

        // Status indicator
        let status = if tool.running {
            Span::styled("◐", Style::default().fg(theme::WARN))
        } else if tool.success == Some(true) {
            Span::styled("✓", Style::default().fg(theme::OK))
        } else {
            Span::styled("✗", Style::default().fg(theme::ERR))
        };

        // Tool name (highlighted if selected)
        let name_style = if is_selected {
            theme::selected()
        } else {
            theme::list_item()
        };

        let name = Span::styled(tool.name.to_string(), name_style);

        spans.push(status);
        spans.push(Span::raw(" "));
        spans.push(name);

        if i < app.tools.len() - 1 {
            spans.push(Span::styled(" │ ", theme::separator()));
        }
    }

    spans.push(Span::raw(" "));

    Line::from(spans)
}

/// Get the currently selected tool.
fn get_selected_tool(app: &App) -> Option<&ToolExecution> {
    app.selected_tool_index.and_then(|idx| app.tools.get(idx))
}

/// Render the output of a tool.
fn render_tool_output(tool: &ToolExecution, scroll: usize, frame: &mut Frame, area: Rect) {
    if tool.output.is_empty() {
        if tool.running {
            let content = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Running...",
                    Style::default().fg(theme::WARN),
                )),
            ]);
            frame.render_widget(content, area);
        } else {
            let content = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("  (no output)", theme::empty_state())),
            ]);
            frame.render_widget(content, area);
        }
        return;
    }

    // Build lines from output
    let lines: Vec<Line> = tool
        .output
        .lines()
        .map(|line| Line::from(Span::raw(line.to_string())))
        .collect();

    let content_height = lines.len();
    let view_height = area.height as usize;

    // Clamp scroll to valid range
    let max_scroll = content_height.saturating_sub(view_height);
    let actual_scroll = scroll.min(max_scroll);

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((actual_scroll as u16, 0));

    frame.render_widget(paragraph, area);
}

/// Render placeholder when no tools exist.
fn render_no_tools(frame: &mut Frame, area: Rect) {
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  No tools executed yet",
            theme::empty_state(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Tool output will appear here when the assistant uses tools.",
            theme::empty_state(),
        )),
    ]);
    frame.render_widget(content, area);
}

/// Render placeholder when no tool is selected.
fn render_no_selection(frame: &mut Frame, area: Rect) {
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Use ←/→ to select a tool",
            theme::empty_state(),
        )),
    ]);
    frame.render_widget(content, area);
}

/// Render help footer for tool pane.
pub fn render_tool_pane_footer(frame: &mut Frame, area: Rect) {
    let help = Line::from(vec![
        Span::styled("←→", theme::key_hint()),
        Span::styled(" tools ", theme::key_desc()),
        Span::styled("↑↓", theme::key_hint()),
        Span::styled(" scroll ", theme::key_desc()),
        Span::styled("^O", theme::key_hint()),
        Span::styled(" editor ", theme::key_desc()),
        Span::styled("Esc", theme::key_hint()),
        Span::styled(" close", theme::key_desc()),
    ]);

    let footer = Paragraph::new(help);
    frame.render_widget(footer, area);
}
