use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::{App, ChatRole, Focus, LayoutRegions, SidebarSection};

const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Render function. Draws to Frame and updates app.layout for mouse hit-testing.
pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // chat area
            Constraint::Length(1), // thin separator
            Constraint::Length(1), // input (single line, borderless)
            Constraint::Length(1), // status bar (bottom)
        ])
        .split(area);

    if app.focus == Focus::Sidebar {
        let middle = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(vertical[0]);

        render_sidebar(app, frame, middle[0]);
        render_chat(app, frame, middle[1]);

        let sidebar_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(middle[0]);

        app.layout = LayoutRegions {
            sidebar: Some(middle[0]),
            sidebar_ws: Some(sidebar_split[0]),
            sidebar_sessions: Some(sidebar_split[1]),
            sidebar_tab: None,
            chat: middle[1],
            input: vertical[2],
        };
    } else {
        let middle = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(vertical[0]);

        render_sidebar_tab(frame, middle[0]);
        render_chat(app, frame, middle[1]);

        app.layout = LayoutRegions {
            sidebar: None,
            sidebar_ws: None,
            sidebar_sessions: None,
            sidebar_tab: Some(middle[0]),
            chat: middle[1],
            input: vertical[2],
        };
    }

    render_separator(frame, vertical[1]);

    render_input(app, frame, vertical[2]);
    render_status_bar(app, frame, vertical[3]);

    // Autocomplete dropdown (renders above the input line)
    if let Some(ref ac) = app.autocomplete {
        render_autocomplete(ac, frame, vertical[2]);
    }

    // Modal overlay (renders on top of everything)
    if let Some(ref modal) = app.active_modal {
        crate::modal::render_modal(modal, frame);
    }
}

fn render_sidebar_tab(frame: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mid = inner.height / 2;
    let style = Style::default()
        .fg(Color::DarkGray)
        .bg(Color::Rgb(25, 25, 30));
    let mut lines: Vec<Line> = Vec::new();
    for i in 0..inner.height {
        if i == mid {
            lines.push(Line::from(Span::styled(" ▸", style.fg(Color::Cyan))));
        } else if i % 2 == 0 {
            lines.push(Line::from(Span::styled("  ", style)));
        } else {
            lines.push(Line::from(Span::styled(" ·", style)));
        }
    }
    let widget = Paragraph::new(lines).style(style);
    frame.render_widget(widget, inner);
}

fn render_status_bar(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let bar_style = Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 40));
    let dim = Style::default().fg(Color::DarkGray).bg(Color::Rgb(30, 30, 40));
    let mut spans = Vec::new();

    // Model name
    let model = if app.model_name.is_empty() {
        "no model"
    } else {
        &app.model_name
    };
    spans.push(Span::styled(format!(" {model}"), bar_style));

    // Token usage
    let (inp, out) = app.token_usage;
    if inp > 0 || out > 0 {
        spans.push(Span::styled(" │ ", dim));
        spans.push(Span::styled(format_tokens(inp), bar_style));
        spans.push(Span::styled(" / ", dim));
        spans.push(Span::styled(format_tokens(out), bar_style));
    }

    // Workstream
    spans.push(Span::styled(" │ ", dim));
    let ws_name = app
        .current_workstream
        .as_ref()
        .map(|ws| ws.name.as_str())
        .unwrap_or("no workstream");
    spans.push(Span::styled(ws_name.to_string(), bar_style));

    // Permission mode
    if app.permission_mode != "default" {
        spans.push(Span::styled(" │ ", dim));
        let (label, color) = match app.permission_mode.as_str() {
            "bypass" => ("BYPASS", Color::Red),
            "accept_edits" => ("ACCEPT EDITS", Color::Yellow),
            "plan" => ("PLAN", Color::Cyan),
            _ => ("DEFAULT", Color::White),
        };
        spans.push(Span::styled(
            label.to_string(),
            Style::default().fg(color).bg(Color::Rgb(30, 30, 40)).add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }

    // Session ID (8 chars)
    if let Some(ref session) = app.current_session {
        spans.push(Span::styled(" │ ", dim));
        let id_short = &session.id.to_string()[..8];
        spans.push(Span::styled(id_short.to_string(), bar_style));
    }

    // State indicator
    if app.is_generating {
        let frame_char = SPINNER_FRAMES[app.spinner_frame as usize % SPINNER_FRAMES.len()];
        let state_text = if let Some(ref tool) = app.active_tool {
            format!(" {frame_char} Running {tool}...")
        } else {
            format!(" {frame_char} Thinking...")
        };
        spans.push(Span::styled(
            state_text,
            Style::default().fg(Color::Yellow).bg(Color::Rgb(30, 30, 40)),
        ));
        // Elapsed time
        if let Some(started) = app.generation_started {
            let elapsed = started.elapsed().as_secs();
            if elapsed >= 2 {
                spans.push(Span::styled(
                    format!(" {elapsed}s"),
                    Style::default().fg(Color::DarkGray).bg(Color::Rgb(30, 30, 40)),
                ));
            }
        }
    } else {
        spans.push(Span::styled(
            " Ready",
            Style::default().fg(Color::Green).bg(Color::Rgb(30, 30, 40)),
        ));
    }

    let status = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Rgb(30, 30, 40)));
    frame.render_widget(status, area);
}

/// Format a token count for display: 1234 → "1.2k", 12345 → "12.3k", 500 → "500"
fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn render_sidebar(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let border_color = if app.focus == Focus::Sidebar {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    // Split sidebar into workstreams (top) and sessions (bottom)
    let sidebar_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Workstreams section
    let ws_items: Vec<ListItem> = app
        .workstreams
        .iter()
        .enumerate()
        .map(|(i, ws)| {
            let prefix = if app.focus == Focus::Sidebar
                && app.sidebar_section == SidebarSection::Workstreams
                && i == app.sidebar_ws_index
            {
                "▸ "
            } else {
                "  "
            };
            let style = if Some(&ws.id) == app.current_workstream.as_ref().map(|w| &w.id) {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{prefix}{}", ws.name)).style(style)
        })
        .collect();

    let ws_block = Block::default()
        .title(" Workstreams ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let ws_list = List::new(ws_items).block(ws_block);
    frame.render_widget(ws_list, sidebar_split[0]);

    // Sessions section
    let session_items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let prefix = if app.focus == Focus::Sidebar
                && app.sidebar_section == SidebarSection::Sessions
                && i == app.sidebar_session_index
            {
                "▸ "
            } else {
                "  "
            };
            let id_short = &s.id.to_string()[..8];
            let date = s.created_at.format("%m/%d %H:%M");
            let style = if Some(&s.id) == app.current_session.as_ref().map(|sess| &sess.id) {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{prefix}{id_short} {date}")).style(style)
        })
        .collect();

    let session_block = Block::default()
        .title(" Sessions ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let session_list = List::new(session_items).block(session_block);
    frame.render_widget(session_list, sidebar_split[1]);
}

fn render_chat(app: &mut App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let mut lines: Vec<Line> = Vec::new();
    let num_messages = app.messages.len();

    // Pre-compute per-message flags to avoid borrow conflicts in the loop.
    // For each tool call: does the next message have a result? Is it an error?
    let tool_call_flags: Vec<(bool, bool)> = (0..num_messages)
        .map(|i| {
            let next_is_result = i + 1 < num_messages
                && matches!(app.messages[i + 1].role, ChatRole::ToolResult { .. });
            let next_is_error = i + 1 < num_messages
                && matches!(app.messages[i + 1].role, ChatRole::ToolResult { is_error: true, .. });
            (next_is_result, next_is_error)
        })
        .collect();

    let chat_width = area.width as usize;

    for (msg_idx, msg) in app.messages.iter_mut().enumerate() {
        match &msg.role {
            ChatRole::User => {
                // Blank line before user messages (turn separator) unless first message
                if msg_idx > 0 {
                    lines.push(Line::from(""));
                }
                lines.push(Line::from(vec![
                    Span::styled(
                        "❯ ",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(&msg.content),
                ]));
            }
            ChatRole::Assistant => {
                // Skip empty assistant messages (e.g., tool-use-only turns)
                if msg.content.trim().is_empty() {
                    continue;
                }
                lines.push(Line::from(""));
                let content_width = chat_width.saturating_sub(2);
                for md_line in msg.rendered_lines(content_width) {
                    let mut indented = vec![Span::raw("  ")];
                    indented.extend(md_line.spans.clone());
                    lines.push(Line::from(indented));
                }
            }
            ChatRole::ToolCall { name } => {
                let chrome = Style::default().fg(Color::Rgb(100, 100, 115));

                let (next_is_result, next_is_error) = tool_call_flags[msg_idx];
                let is_running = !next_is_result && app.is_generating;

                let icon = if is_running {
                    let frame_char = SPINNER_FRAMES[app.spinner_frame as usize % SPINNER_FRAMES.len()];
                    Span::styled(
                        format!("{frame_char} "),
                        Style::default().fg(Color::Yellow),
                    )
                } else if next_is_error {
                    Span::styled("✗ ", Style::default().fg(Color::Red))
                } else if next_is_result {
                    Span::styled("✓ ", Style::default().fg(Color::Green))
                } else {
                    Span::styled("⏳ ", Style::default().fg(Color::Yellow))
                };

                let tool_name = Span::styled(
                    name.clone(),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                );
                let summary = compact_tool_summary(&msg.content);
                let summary_span = if summary.is_empty() {
                    Span::raw("")
                } else {
                    Span::styled(format!("  {summary}"), Style::default().fg(Color::Rgb(140, 140, 155)))
                };

                let elapsed_span = if is_running {
                    let elapsed = msg.created_at.elapsed().as_secs_f64();
                    Span::styled(
                        format!("  {elapsed:.1}s"),
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    Span::raw("")
                };

                // Build header content to measure its width for the fill dashes
                let header_text = format!(
                    "┌ {} {} {}",
                    if is_running { "⠹" } else if next_is_error { "✗" } else if next_is_result { "✓" } else { "⏳" },
                    name,
                    &summary.chars().take(40).collect::<String>(),
                );
                let fill_len = chat_width.saturating_sub(header_text.len() + 3); // 3 for "  " prefix + "┐"
                let fill = "─".repeat(fill_len.min(80));

                lines.push(Line::from(vec![
                    Span::styled("  ┌ ", chrome),
                    icon,
                    tool_name,
                    summary_span,
                    elapsed_span,
                    Span::styled(format!(" {fill}┐"), chrome),
                ]));
            }
            ChatRole::ToolResult { name, is_error } => {
                let chrome = Style::default().fg(Color::Rgb(100, 100, 115));
                let is_expanded = app.expanded_tool_results.contains(&msg_idx);

                if *is_error {
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", chrome),
                        Span::styled("✗ ", Style::default().fg(Color::Red)),
                        Span::styled(
                            format!("{name} error"),
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    for err_line in msg.content.lines().take(10) {
                        lines.push(Line::from(vec![
                            Span::styled("  │  ", chrome),
                            Span::styled(err_line.to_string(), Style::default().fg(Color::Red)),
                        ]));
                    }
                    let bottom_len = chat_width.saturating_sub(4);
                    lines.push(Line::from(Span::styled(
                        format!("  └{}┘", "─".repeat(bottom_len.min(80))),
                        chrome,
                    )));
                } else if is_expanded {
                    let toggle_hint = Span::styled(
                        " (Ctrl+E to collapse)",
                        Style::default().fg(Color::Rgb(100, 100, 115)).add_modifier(Modifier::ITALIC),
                    );
                    let result_text = Style::default().fg(Color::Rgb(150, 150, 165));
                    let label_style = Style::default().fg(Color::Rgb(130, 130, 145));
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", chrome),
                        Span::styled("▾ ", label_style),
                        Span::styled(format!("{name} result"), label_style),
                        toggle_hint,
                    ]));
                    for result_line in msg.content.lines() {
                        lines.push(Line::from(vec![
                            Span::styled("  │  ", chrome),
                            Span::styled(result_line.to_string(), result_text),
                        ]));
                    }
                    let bottom_len = chat_width.saturating_sub(4);
                    lines.push(Line::from(Span::styled(
                        format!("  └{}┘", "─".repeat(bottom_len.min(80))),
                        chrome,
                    )));
                } else {
                    let total_lines = msg.content.lines().count();
                    let max_preview = 5;
                    let result_text = Style::default().fg(Color::Rgb(150, 150, 165));
                    let label_style = Style::default().fg(Color::Rgb(130, 130, 145));
                    let toggle_hint = if total_lines > max_preview {
                        Span::styled(
                            format!(" ({total_lines} lines — Ctrl+E to expand)"),
                            Style::default().fg(Color::Rgb(100, 100, 115)).add_modifier(Modifier::ITALIC),
                        )
                    } else {
                        Span::raw("")
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", chrome),
                        Span::styled("▸ ", label_style),
                        Span::styled(format!("{name} result"), label_style),
                        toggle_hint,
                    ]));
                    for result_line in msg.content.lines().take(max_preview) {
                        lines.push(Line::from(vec![
                            Span::styled("  │  ", chrome),
                            Span::styled(result_line.to_string(), result_text),
                        ]));
                    }
                    if total_lines > max_preview {
                        lines.push(Line::from(vec![
                            Span::styled("  │  ", chrome),
                            Span::styled(
                                format!("… {remaining} more", remaining = total_lines - max_preview),
                                Style::default().fg(Color::Rgb(120, 120, 135)).add_modifier(Modifier::ITALIC),
                            ),
                        ]));
                    }
                    let bottom_len = chat_width.saturating_sub(4);
                    lines.push(Line::from(Span::styled(
                        format!("  └{}┘", "─".repeat(bottom_len.min(80))),
                        chrome,
                    )));
                }
            }
            ChatRole::System => {
                lines.push(Line::from(Span::styled(
                    "system:",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::ITALIC),
                )));
                let content_width = chat_width.saturating_sub(2);
                for md_line in msg.rendered_lines(content_width) {
                    let mut indented = vec![Span::raw("  ")];
                    indented.extend(md_line.spans.clone());
                    lines.push(Line::from(indented));
                }
            }
        }
    }

    // Streaming text (in progress)
    if !app.streaming_text.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::raw(app.streaming_text.clone()),
            Span::styled("█", Style::default().fg(Color::Blue)),
        ]));
    } else if app.is_generating {
        // Waiting for first token — show thinking indicator with elapsed time
        let frame_char = SPINNER_FRAMES[app.spinner_frame as usize % SPINNER_FRAMES.len()];
        let elapsed = app.generation_started
            .map(|t| format!(" {:.1}s", t.elapsed().as_secs_f64()))
            .unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled(
                format!("{frame_char} "),
                Style::default().fg(Color::Blue),
            ),
            Span::styled(
                format!("thinking...{elapsed}"),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
        ]));
    }

    // Calculate scroll: show bottom of content by default (auto-scroll).
    // scroll_offset = 0 means "at the bottom". User scrolling up increases it.
    // Account for line wrapping by estimating visual lines.
    let content_width = (area.width as usize).max(1);
    // Use the actual chat area height — the layout already excludes separator/input/status.
    let visible_height = area.height as usize;
    let visual_lines: usize = lines
        .iter()
        .map(|line| {
            // Use ratatui's Line::width() for accurate unicode display width
            let w = line.width();
            if w == 0 {
                1
            } else {
                w.div_ceil(content_width)
            }
        })
        .sum();
    let auto_scroll_pos = visual_lines.saturating_sub(visible_height);
    // Clamp scroll_offset to valid range — can't scroll past the top
    if app.scroll_offset > auto_scroll_pos {
        app.scroll_offset = auto_scroll_pos;
    }
    let scroll_pos = if app.scroll_offset == 0 {
        auto_scroll_pos
    } else {
        auto_scroll_pos - app.scroll_offset
    };

    let scroll_u16 = u16::try_from(scroll_pos).unwrap_or(u16::MAX);
    let chat = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll_u16, 0));

    frame.render_widget(chat, area);
}

fn render_separator(frame: &mut Frame, area: ratatui::layout::Rect) {
    let line = "─".repeat(area.width as usize);
    let sep = Paragraph::new(line).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(sep, area);
}

fn render_input(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let prompt = "> ";
    let prompt_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);

    if app.is_generating {
        let line = Line::from(vec![
            Span::styled(prompt, Style::default().fg(Color::DarkGray)),
            Span::styled("Generating...", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    } else if app.input_buffer.is_empty() {
        let line = Line::from(vec![
            Span::styled(prompt, prompt_style),
            Span::styled("Type your message...", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    } else {
        // Horizontal scroll: keep cursor visible within the available width
        let prompt_len = prompt.len();
        let available = (area.width as usize).saturating_sub(prompt_len);
        let cursor = app.cursor_pos;
        let buf = &app.input_buffer;

        // Compute the visible window start so the cursor is always on screen
        let scroll_offset = if cursor >= available {
            cursor - available + 1
        } else {
            0
        };
        let visible_end = (scroll_offset + available).min(buf.len());
        let visible = &buf[scroll_offset..visible_end];

        let line = Line::from(vec![Span::styled(prompt, prompt_style), Span::raw(visible)]);
        frame.render_widget(Paragraph::new(line), area);

        if app.focus == Focus::Main {
            let cursor_x = area.x + prompt_len as u16 + (cursor - scroll_offset) as u16;
            frame.set_cursor_position((cursor_x, area.y));
        }
        return;
    }

    if app.focus == Focus::Main && !app.is_generating {
        let x = area.x + prompt.len() as u16 + app.cursor_pos as u16;
        let y = area.y;
        frame.set_cursor_position((x, y));
    }
}

/// Render the autocomplete dropdown above the input line.
fn render_autocomplete(
    ac: &crate::command::AutocompleteState,
    frame: &mut Frame,
    input_area: ratatui::layout::Rect,
) {
    if ac.is_empty() {
        return;
    }

    let max_visible = 8.min(ac.suggestions.len());
    let dropdown_height = max_visible as u16 + 2; // +2 for border

    // Position: directly above the input line
    let dropdown_area = ratatui::layout::Rect {
        x: input_area.x + 2, // offset to align with text after "> "
        y: input_area.y.saturating_sub(dropdown_height),
        width: input_area.width.min(50).max(20),
        height: dropdown_height,
    };

    // Clear the area
    frame.render_widget(Clear, dropdown_area);

    let items: Vec<Line> = ac
        .suggestions
        .iter()
        .take(max_visible)
        .enumerate()
        .map(|(i, cmd)| {
            let is_selected = i == ac.selected;
            let prefix = if is_selected { "▸ " } else { "  " };
            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let desc_style = Style::default().fg(Color::DarkGray);

            Line::from(vec![
                Span::raw(prefix),
                Span::styled(format!("/{:<12}", cmd.name), name_style),
                Span::styled(
                    truncate_to(&cmd.description, dropdown_area.width as usize - 18),
                    desc_style,
                ),
            ])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .style(Style::default().bg(Color::Rgb(30, 30, 35)));

    let paragraph = Paragraph::new(items).block(block);
    frame.render_widget(paragraph, dropdown_area);
}

/// Truncate a string to fit within a display width, adding "…" if needed.
fn truncate_to(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else if max_chars > 1 {
        let truncated: String = s.chars().take(max_chars - 1).collect();
        format!("{truncated}…")
    } else {
        String::new()
    }
}

/// Extract a compact summary from tool call content for inline display.
fn compact_tool_summary(content: &str) -> String {
    if content.is_empty() {
        return String::new();
    }
    truncate_for_display(content, 60)
}

fn truncate_for_display(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, ChatMessage};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn buffer_to_string(terminal: &Terminal<TestBackend>, row: u16) -> String {
        let width = terminal.backend().buffer().area.width;
        (0..width)
            .map(|x| {
                terminal
                    .backend()
                    .buffer()
                    .cell((x, row))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect()
    }

    #[test]
    fn render_empty_app_has_status_bar() {
        let mut app = App::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let h = terminal.backend().buffer().area.height;
        let status_row = buffer_to_string(&terminal, h - 1);
        assert!(status_row.contains("no model") || status_row.contains("no workstream"));
    }

    #[test]
    fn render_with_messages_shows_content() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::User, "hello world"));
        app.messages
            .push(ChatMessage::new(ChatRole::Assistant, "hi there"));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        // Check that message content appears somewhere in the buffer
        let buf = terminal.backend().buffer();
        let full_buffer: String = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .map(|(x, y)| {
                buf.cell((x, y))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect();
        assert!(full_buffer.contains("hello world"));
        assert!(full_buffer.contains("hi there"));
    }

    #[test]
    fn render_with_input_text() {
        let mut app = App::new();
        app.input_buffer = "test input".into();
        app.cursor_pos = 10;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        // Input bar is second from bottom (above status bar)
        let input_row = buffer_to_string(&terminal, 22);
        assert!(
            input_row.contains("test input"),
            "expected 'test input' in bottom row, got: {input_row}"
        );
    }

    #[test]
    fn render_streaming_shows_cursor() {
        let mut app = App::new();
        app.is_generating = true;
        app.streaming_text = "partial response".into();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let buf = terminal.backend().buffer();
        let full_buffer: String = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .map(|(x, y)| {
                buf.cell((x, y))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect();
        assert!(full_buffer.contains("partial response"));
        assert!(full_buffer.contains("█")); // cursor indicator
    }

    #[test]
    fn render_small_terminal() {
        let mut app = App::new();
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();
    }

    #[test]
    fn render_large_terminal() {
        let mut app = App::new();
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();
    }

    // --- Helpers for region-specific assertions ---

    /// Extract text from a rectangular region of the buffer.
    fn region_text(terminal: &Terminal<TestBackend>, x: u16, y: u16, w: u16, h: u16) -> String {
        let buf = terminal.backend().buffer();
        let mut text = String::new();
        for row in y..y + h {
            for col in x..x + w {
                if let Some(cell) = buf.cell((col, row)) {
                    text.push_str(cell.symbol());
                }
            }
            text.push('\n');
        }
        text
    }

    /// Extract the chat area text. When sidebar is visible (focus == Sidebar),
    /// chat is right 80%; otherwise chat is full width.
    fn chat_region_for(terminal: &Terminal<TestBackend>, sidebar_visible: bool) -> String {
        let buf = terminal.backend().buffer();
        let w = buf.area.width;
        let h = buf.area.height;
        let (chat_x, chat_w) = if sidebar_visible {
            let sidebar_w = w / 5;
            (sidebar_w, w - sidebar_w)
        } else {
            (0, w)
        };
        let chat_y = 0;
        let chat_h = h.saturating_sub(3); // exclude separator (1) + input (1) + status bar (1)
        region_text(terminal, chat_x, chat_y, chat_w, chat_h)
    }

    /// Convenience: chat region for default app (sidebar hidden).
    fn chat_region(terminal: &Terminal<TestBackend>) -> String {
        chat_region_for(terminal, false)
    }

    /// Extract the sidebar text (left 20%, rows 1..height-3).
    /// Only meaningful when sidebar is visible (focus == Sidebar).
    fn sidebar_region(terminal: &Terminal<TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        let w = buf.area.width;
        let h = buf.area.height;
        let sidebar_w = w / 5;
        let sidebar_y = 0;
        let sidebar_h = h.saturating_sub(3);
        region_text(terminal, 0, sidebar_y, sidebar_w, sidebar_h)
    }

    /// Extract the input bar text (second from bottom row).
    fn input_region(terminal: &Terminal<TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        let w = buf.area.width;
        let h = buf.area.height;
        region_text(terminal, 0, h - 2, w, 1)
    }

    // --- Targeted component rendering tests ---

    #[test]
    fn chat_renders_user_message_with_prefix() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::User, "What files exist?"));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("❯") && chat.contains("What files exist?"),
            "chat should show '❯ What files exist?', got:\n{chat}"
        );
    }

    #[test]
    fn chat_renders_assistant_message_with_prefix() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::Assistant, "Here are the files."));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("Here are the files"),
            "chat should show assistant text, got:\n{chat}"
        );
    }

    #[test]
    fn chat_renders_tool_call_with_icon() {
        let mut app = App::new();
        app.messages.push(ChatMessage::new(
            ChatRole::ToolCall { name: "shell".into() },
            "ls -la",
        ));
        // Add a result so the tool call shows ✓ (completed)
        app.messages.push(ChatMessage::new(
            ChatRole::ToolResult { name: "shell".into(), is_error: false },
            "file1.rs",
        ));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("shell"),
            "chat should show tool name 'shell', got:\n{chat}"
        );
        assert!(
            chat.contains("✓"),
            "chat should show ✓ icon for completed tool, got:\n{chat}"
        );
    }

    #[test]
    fn chat_renders_tool_result_collapsed() {
        let mut app = App::new();
        app.messages.push(ChatMessage::new(
            ChatRole::ToolResult {
                name: "shell".into(),
                is_error: false,
            },
            "file1.rs\nfile2.rs\nfile3.rs",
        ));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("shell result"),
            "chat should show 'shell result', got:\n{chat}"
        );
        assert!(
            chat.contains("file1.rs"),
            "chat should show tool output, got:\n{chat}"
        );
        // Collapsed indicator
        assert!(
            chat.contains("▸"),
            "collapsed result should show ▸, got:\n{chat}"
        );
    }

    #[test]
    fn chat_renders_tool_error_result() {
        let mut app = App::new();
        app.messages.push(ChatMessage::new(
            ChatRole::ToolResult {
                name: "shell".into(),
                is_error: true,
            },
            "permission denied",
        ));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("✗") && chat.contains("shell") && chat.contains("error"),
            "chat should show '✗ shell error', got:\n{chat}"
        );
        assert!(
            chat.contains("permission denied"),
            "error content should be visible, got:\n{chat}"
        );
    }

    #[test]
    fn chat_renders_tool_result_truncated() {
        let mut app = App::new();
        let long_result = (0..20)
            .map(|i| format!("line_{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        app.messages.push(ChatMessage::new(
            ChatRole::ToolResult {
                name: "grep".into(),
                is_error: false,
            },
            long_result,
        ));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        // First 5 lines should be visible
        assert!(chat.contains("line_0"), "first line should be visible");
        assert!(chat.contains("line_4"), "5th line should be visible");
        // Should show truncation indicator
        assert!(
            chat.contains("more"),
            "should show truncation indicator, got:\n{chat}"
        );
    }

    #[test]
    fn chat_streaming_text_appears_in_chat_area() {
        let mut app = App::new();
        app.is_generating = true;
        app.streaming_text = "streaming response here".into();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("streaming response here"),
            "streaming text should be in chat area, got:\n{chat}"
        );
        assert!(
            chat.contains("█"),
            "streaming text should show cursor, got:\n{chat}"
        );
    }

    #[test]
    fn sidebar_renders_workstream_names() {
        use arawn_service::WorkstreamInfo;
        use chrono::Utc;
        use std::path::PathBuf;
        use uuid::Uuid;

        let mut app = App::new();
        app.focus = Focus::Sidebar;
        app.workstreams = vec![
            WorkstreamInfo {
                id: Uuid::new_v4(),
                name: "scratch".into(),
                root_dir: PathBuf::from("/tmp"),
                created_at: Utc::now(),
            },
            WorkstreamInfo {
                id: Uuid::new_v4(),
                name: "myproject".into(),
                root_dir: PathBuf::from("/tmp"),
                created_at: Utc::now(),
            },
        ];

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let sidebar = sidebar_region(&terminal);
        assert!(
            sidebar.contains("scratch"),
            "sidebar should show 'scratch', got:\n{sidebar}"
        );
        assert!(
            sidebar.contains("myproject"),
            "sidebar should show 'myproject', got:\n{sidebar}"
        );
    }

    #[test]
    fn sidebar_does_not_leak_into_chat() {
        use arawn_service::WorkstreamInfo;
        use chrono::Utc;
        use std::path::PathBuf;
        use uuid::Uuid;

        let mut app = App::new();
        app.focus = Focus::Sidebar;
        app.workstreams = vec![WorkstreamInfo {
            id: Uuid::new_v4(),
            name: "sb_data".into(),
            root_dir: PathBuf::from("/tmp"),
            created_at: Utc::now(),
        }];
        app.messages
            .push(ChatMessage::new(ChatRole::User, "ch_data"));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let sidebar = sidebar_region(&terminal);
        let chat = chat_region_for(&terminal, true);

        assert!(sidebar.contains("sb_data"));
        assert!(
            !sidebar.contains("ch_data"),
            "chat content should not appear in sidebar"
        );
        assert!(chat.contains("ch_data"));
        assert!(
            !chat.contains("sb_data"),
            "sidebar content should not appear in chat"
        );
    }

    #[test]
    fn input_shows_placeholder_when_empty() {
        let mut app = App::new(); // empty buffer, focus on input
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let input = input_region(&terminal);
        assert!(
            input.contains("Type your message"),
            "empty input should show placeholder, got:\n{input}"
        );
    }

    #[test]
    fn input_shows_generating_when_active() {
        let mut app = App::new();
        app.is_generating = true;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let input = input_region(&terminal);
        assert!(
            input.contains("Generating"),
            "input should show 'Generating...' during generation, got:\n{input}"
        );
    }

    #[test]
    fn status_bar_shows_generating_indicator() {
        let mut app = App::new();
        app.is_generating = true;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let h = terminal.backend().buffer().area.height;
        let status = buffer_to_string(&terminal, h - 1);
        assert!(
            status.contains("Thinking"),
            "status bar should show generating indicator, got:\n{status}"
        );
    }

    #[test]
    fn status_bar_shows_workstream_name() {
        use arawn_service::WorkstreamInfo;
        use chrono::Utc;
        use std::path::PathBuf;
        use uuid::Uuid;

        let mut app = App::new();
        app.current_workstream = Some(WorkstreamInfo {
            id: Uuid::new_v4(),
            name: "Home Maintenance".into(),
            root_dir: PathBuf::from("/tmp"),
            created_at: Utc::now(),
        });

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let h = terminal.backend().buffer().area.height;
        let status = buffer_to_string(&terminal, h - 1);
        assert!(
            status.contains("Home Maintenance"),
            "status bar should show workstream name, got:\n{status}"
        );
    }

    #[test]
    fn messages_do_not_appear_in_input_area() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::User, "unique_chat_text_xyz"));
        app.input_buffer = "unique_input_text_abc".into();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let input = input_region(&terminal);
        let chat = chat_region(&terminal);

        assert!(input.contains("unique_input_text_abc"));
        assert!(
            !input.contains("unique_chat_text_xyz"),
            "chat content should not leak into input"
        );
        assert!(chat.contains("unique_chat_text_xyz"));
        assert!(
            !chat.contains("unique_input_text_abc"),
            "input content should not leak into chat"
        );
    }

    // --- Scroll tests ---

    #[test]
    fn chat_auto_scrolls_to_bottom_with_many_messages() {
        let mut app = App::new();

        // Add enough messages to overflow a 20-row chat area
        // Each message = 2 lines (content + blank separator)
        for i in 0..30 {
            app.messages
                .push(ChatMessage::new(ChatRole::User, format!("MSG_NUM_{i:03}")));
        }

        // scroll_offset = 0 means "show bottom"
        assert_eq!(app.scroll_offset, 0);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);

        // Bottom messages should be visible
        assert!(
            chat.contains("MSG_NUM_029"),
            "last message should be visible (auto-scroll to bottom), got:\n{chat}"
        );

        // Top messages should NOT be visible (scrolled off the top)
        assert!(
            !chat.contains("MSG_NUM_000"),
            "first message should be scrolled off the top, got:\n{chat}"
        );
    }

    #[test]
    fn chat_scroll_up_reveals_older_messages() {
        let mut app = App::new();

        for i in 0..30 {
            app.messages
                .push(ChatMessage::new(ChatRole::User, format!("MSG_NUM_{i:03}")));
        }

        // Scroll up a lot — should reveal earlier messages
        app.scroll_offset = 50; // scroll way up

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);

        // First message should now be visible
        assert!(
            chat.contains("MSG_NUM_000"),
            "first message should be visible after scrolling up, got:\n{chat}"
        );

        // Last message should NOT be visible (scrolled below)
        assert!(
            !chat.contains("MSG_NUM_029"),
            "last message should be scrolled off bottom after scrolling up, got:\n{chat}"
        );
    }

    #[test]
    fn chat_few_messages_all_visible() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::User, "ONLY_MSG"));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("ONLY_MSG"),
            "single message should be visible, got:\n{chat}"
        );
    }

    #[test]
    fn last_message_visible_above_input() {
        // The REAL failure mode: scroll doesn't go far enough, so the last
        // message ends up on rows that the separator/input then paints over.
        // We can't detect "painted over" — so we read the FULL buffer and
        // check the marker appears ABOVE the separator row.
        let mut app = App::new();

        for i in 0..15 {
            app.messages
                .push(ChatMessage::new(ChatRole::User, format!("question_{i}")));
            app.messages.push(ChatMessage::new(
                ChatRole::Assistant,
                format!("answer_{i} with some extra text"),
            ));
        }
        app.messages.push(ChatMessage::new(
            ChatRole::Assistant,
            "FINAL_VISIBLE_ANSWER",
        ));
        assert_eq!(app.scroll_offset, 0);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let buf = terminal.backend().buffer();
        let h = buf.area.height;
        let w = buf.area.width;

        // Find which row contains the separator (───)
        let sep_row = (0..h).find(|&row| {
            let row_text: String = (0..w)
                .map(|x| buf.cell((x, row)).unwrap().symbol().to_string())
                .collect();
            row_text.contains("───")
        });
        let sep_row = sep_row.expect("should have a separator row");

        // Read all rows ABOVE the separator — this is what the user actually sees as chat
        let visible_chat: String = (0..sep_row)
            .map(|row| {
                let line: String = (0..w)
                    .map(|x| buf.cell((x, row)).unwrap().symbol().to_string())
                    .collect();
                line
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            visible_chat.contains("FINAL_VISIBLE_ANSWER"),
            "last message must appear in rows ABOVE the separator (row {sep_row}), got:\n{visible_chat}"
        );
    }

    #[test]
    fn last_tool_result_visible_above_input() {
        let mut app = App::new();

        for i in 0..10 {
            app.messages
                .push(ChatMessage::new(ChatRole::User, format!("q_{i}")));
            app.messages.push(ChatMessage::new(
                ChatRole::Assistant,
                format!("a_{i}"),
            ));
        }
        app.messages.push(ChatMessage::new(
            ChatRole::ToolCall { name: "shell".into() },
            "cargo test",
        ));
        app.messages.push(ChatMessage::new(
            ChatRole::ToolResult {
                name: "shell".into(),
                is_error: false,
            },
            "TOOL_OUTPUT_VISIBLE",
        ));
        app.messages.push(ChatMessage::new(
            ChatRole::Assistant,
            "AFTER_TOOL_VISIBLE",
        ));

        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let buf = terminal.backend().buffer();
        let h = buf.area.height;
        let w = buf.area.width;

        // Find the thin separator row (between chat and input) — skip box-drawing
        // inside tool cards by looking for a full-width separator row.
        let sep_row = (0..h)
            .rev()
            .find(|&row| {
                let row_text: String = (0..w)
                    .map(|x| buf.cell((x, row)).unwrap().symbol().to_string())
                    .collect();
                // The separator is a full line of ─ with no other box chars (│┌└┐┘)
                row_text.contains("───") && !row_text.contains('│') && !row_text.contains('┌') && !row_text.contains('└')
            })
            .expect("should have separator row");

        let visible_chat: String = (0..sep_row)
            .map(|row| {
                let line: String = (0..w)
                    .map(|x| buf.cell((x, row)).unwrap().symbol().to_string())
                    .collect();
                line
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            visible_chat.contains("AFTER_TOOL_VISIBLE"),
            "message after tool result must appear above separator (row {sep_row}), got:\n{visible_chat}"
        );
    }
}
