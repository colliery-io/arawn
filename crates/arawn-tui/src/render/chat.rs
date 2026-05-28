use ratatui::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::{SPINNER_FRAMES, compact_tool_summary};
use crate::app::{App, ChatRole};
use crate::theme;

pub(super) fn render_chat(app: &mut App, frame: &mut Frame, area: ratatui::layout::Rect) {
    // Empty chat, no streaming, not generating. Two cases:
    // - At least one ceremony tablet exists → render the cached brief
    //   markdown top-aligned in the chat area (I-0035 Phase 2). The
    //   brief shares the assistant-message render path so it inherits
    //   the same Catppuccin Mocha styling as everything else.
    // - No tablets yet → fall through to the T-0331 welcome hero
    //   (centered wordmark + key-binding hints).
    if app.messages.is_empty() && !app.is_generating && app.streaming_text.is_empty() {
        if app.should_show_brief_in_empty_chat() {
            render_empty_chat_brief(app, frame, area);
        } else {
            render_idle_hero(frame, area, app.model_name.is_empty());
        }
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let num_messages = app.messages.len();

    // Pre-compute per-message flags to avoid borrow conflicts in the loop.
    // For each tool call: does the next message have a result? Is it an error?
    let tool_call_flags: Vec<(bool, bool)> = (0..num_messages)
        .map(|i| {
            let next_is_result = i + 1 < num_messages
                && matches!(app.messages[i + 1].role, ChatRole::ToolResult { .. });
            let next_is_error = i + 1 < num_messages
                && matches!(
                    app.messages[i + 1].role,
                    ChatRole::ToolResult { is_error: true, .. }
                );
            (next_is_result, next_is_error)
        })
        .collect();

    // ARAWN-I-0061: signal provenance footer chip. For each Assistant message,
    // collect the distinct source lenses from any `signal_*` tool results that
    // landed in the same turn (between this assistant message and the prior
    // user/system message). Pre-computed here to dodge the borrow conflict
    // against `iter_mut()` below.
    let signal_sources_per_message: Vec<Vec<String>> = (0..num_messages)
        .map(|i| {
            if matches!(app.messages[i].role, ChatRole::Assistant) {
                collect_signal_sources_for_turn(&app.messages, i)
            } else {
                Vec::new()
            }
        })
        .collect();

    let chat_width = area.width as usize;

    // Snapshot whether the last message is a completed-turn marker
    // candidate (assistant text or tool result). Captured before the
    // mutable iteration below so we can use it after the loop without
    // triggering a borrow conflict.
    let last_is_completed_turn = matches!(
        app.messages.last().map(|m| &m.role),
        Some(ChatRole::Assistant) | Some(ChatRole::ToolResult { .. })
    );

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
                            .fg(theme::USER)
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
                let gutter_style = Style::default().fg(theme::CHROME);
                let content_width = chat_width.saturating_sub(4);
                for md_line in msg.rendered_lines(content_width) {
                    let mut prefixed = vec![Span::styled("│ ", gutter_style)];
                    prefixed.extend(md_line.spans.clone());
                    lines.push(Line::from(prefixed));
                }

                // ARAWN-I-0061: sources footer. Surfaces the cross-lens
                // provenance of `signal_*` reads consumed by this turn so the
                // user sees that an answer drew from `work · personal · …`
                // without expanding the raw tool card.
                let sources = &signal_sources_per_message[msg_idx];
                if !sources.is_empty() {
                    let dim = Style::default().fg(theme::OVERLAY0);
                    let mut footer: Vec<Span<'static>> = vec![
                        Span::styled("│ ", gutter_style),
                        Span::styled("◆ sources: ", dim),
                    ];
                    for (i, name) in sources.iter().enumerate() {
                        if i > 0 {
                            footer.push(Span::styled(" · ", dim));
                        }
                        footer.push(Span::styled(
                            name.clone(),
                            Style::default().fg(theme::TOOL_NAME),
                        ));
                    }
                    lines.push(Line::from(footer));
                }
            }
            ChatRole::ToolCall { name } => {
                let chrome = Style::default().fg(theme::CHROME);

                let (next_is_result, next_is_error) = tool_call_flags[msg_idx];
                let is_running = !next_is_result && app.is_generating;

                // Collapsed by default; expand when the user has toggled
                // either this call or its paired result, or when the
                // paired result is an error (errors always show body).
                let pair_toggled = app.expanded_tool_results.contains(&msg_idx)
                    || (next_is_result && app.expanded_tool_results.contains(&(msg_idx + 1)));
                let expand_card = next_is_error || pair_toggled;

                let summary_raw = compact_tool_summary(&msg.content);

                if !expand_card {
                    // Single-line collapsed render
                    let mut spans: Vec<Span<'static>> = Vec::with_capacity(8);
                    spans.push(Span::styled("  ⏵ ", chrome));
                    spans.push(Span::styled(
                        name.clone(),
                        Style::default()
                            .fg(theme::TOOL_NAME)
                            .add_modifier(Modifier::BOLD),
                    ));
                    if !summary_raw.is_empty() {
                        let trimmed = crate::width::truncate_display(&summary_raw, 60);
                        spans.push(Span::styled(" · ", chrome));
                        spans.push(Span::styled(
                            trimmed,
                            Style::default().fg(theme::TOOL_SUMMARY),
                        ));
                    }
                    if is_running {
                        let elapsed = msg.created_at.elapsed().as_secs_f64();
                        let frame_char =
                            SPINNER_FRAMES[app.spinner_frame as usize % SPINNER_FRAMES.len()];
                        spans.push(Span::styled(" · ", chrome));
                        spans.push(Span::styled(
                            format!("running {elapsed:.1}s "),
                            Style::default().fg(theme::OVERLAY0),
                        ));
                        spans.push(Span::styled(
                            frame_char.to_string(),
                            Style::default().fg(theme::GENERATING),
                        ));
                    } else if next_is_result {
                        let elapsed = msg.created_at.elapsed().as_secs_f64();
                        spans.push(Span::styled(" · ", chrome));
                        spans.push(Span::styled(
                            format!("{elapsed:.1}s"),
                            Style::default().fg(theme::OVERLAY0),
                        ));
                    }
                    lines.push(Line::from(spans));
                } else {
                    let icon = if is_running {
                        let frame_char =
                            SPINNER_FRAMES[app.spinner_frame as usize % SPINNER_FRAMES.len()];
                        Span::styled(
                            format!("{frame_char} "),
                            Style::default().fg(theme::GENERATING),
                        )
                    } else if next_is_error {
                        Span::styled("✗ ", Style::default().fg(theme::ERROR))
                    } else if next_is_result {
                        Span::styled("✓ ", Style::default().fg(theme::SUCCESS))
                    } else {
                        Span::styled("⏳ ", Style::default().fg(theme::GENERATING))
                    };

                    let tool_name = Span::styled(
                        name.clone(),
                        Style::default()
                            .fg(theme::TOOL_NAME)
                            .add_modifier(Modifier::BOLD),
                    );
                    let summary_span = if summary_raw.is_empty() {
                        Span::raw("")
                    } else {
                        Span::styled(
                            format!("  {summary_raw}"),
                            Style::default().fg(theme::TOOL_SUMMARY),
                        )
                    };

                    let elapsed_span = if is_running {
                        let elapsed = msg.created_at.elapsed().as_secs_f64();
                        Span::styled(
                            format!("  {elapsed:.1}s"),
                            Style::default().fg(theme::OVERLAY0),
                        )
                    } else {
                        Span::raw("")
                    };

                    use unicode_width::UnicodeWidthStr;
                    let box_width = chat_width.saturating_sub(2).min(82);
                    let header_spans = [&icon, &tool_name, &summary_span, &elapsed_span];
                    let header_display_width: usize = header_spans
                        .iter()
                        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
                        .sum();
                    let fill_len = box_width.saturating_sub(header_display_width + 4);
                    let fill = "─".repeat(fill_len);

                    lines.push(Line::from(vec![
                        Span::styled("  ┌ ", chrome),
                        icon,
                        tool_name,
                        summary_span,
                        elapsed_span,
                        Span::styled(format!(" {fill}┐"), chrome),
                    ]));
                }
            }
            ChatRole::ToolResult { name, is_error } => {
                let chrome = Style::default().fg(theme::CHROME);
                let is_expanded = app.expanded_tool_results.contains(&msg_idx)
                    || (msg_idx > 0 && app.expanded_tool_results.contains(&(msg_idx - 1)));

                if *is_error {
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", chrome),
                        Span::styled("✗ ", Style::default().fg(theme::ERROR)),
                        Span::styled(
                            format!("{name} error"),
                            Style::default()
                                .fg(theme::ERROR)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    for err_line in msg.content.lines().take(10) {
                        lines.push(Line::from(vec![
                            Span::styled("  │  ", chrome),
                            Span::styled(err_line.to_string(), Style::default().fg(theme::ERROR)),
                        ]));
                    }
                    // Match the top border's box_width so top and bottom
                    // align even with unicode tool names. `└` + N dashes +
                    // `┘` consumes 2 corner chars, so dashes = box_width - 2.
                    let box_width = chat_width.saturating_sub(2).min(82);
                    let bottom_len = box_width.saturating_sub(2);
                    lines.push(Line::from(Span::styled(
                        format!("  └{}┘", "─".repeat(bottom_len)),
                        chrome,
                    )));
                } else if is_expanded {
                    let toggle_hint = Span::styled(
                        " (Ctrl+E to collapse)",
                        Style::default()
                            .fg(theme::CHROME)
                            .add_modifier(Modifier::ITALIC),
                    );
                    let result_text = Style::default().fg(theme::RESULT_TEXT);
                    let label_style = Style::default().fg(theme::RESULT_LABEL);
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
                    // Match the top border's box_width so top and bottom
                    // align even with unicode tool names. `└` + N dashes +
                    // `┘` consumes 2 corner chars, so dashes = box_width - 2.
                    let box_width = chat_width.saturating_sub(2).min(82);
                    let bottom_len = box_width.saturating_sub(2);
                    lines.push(Line::from(Span::styled(
                        format!("  └{}┘", "─".repeat(bottom_len)),
                        chrome,
                    )));
                } else {
                    let total_lines = msg.content.lines().count();
                    let total_chars = msg.content.chars().count();
                    let max_preview_lines = 5;
                    // Per-line char budget: chat width minus the "  │  "
                    // gutter (5 chars) and a small safety margin, so the
                    // preview never overflows into wrap territory.
                    let line_budget = chat_width.saturating_sub(7).max(20);
                    let result_text = Style::default().fg(theme::RESULT_TEXT);
                    let label_style = Style::default().fg(theme::RESULT_LABEL);

                    // Two ways the preview can be incomplete: too many
                    // lines, OR any line is wider than `line_budget`
                    // (single-line JSON dumps used to leak through here).
                    let any_line_too_wide = msg
                        .content
                        .lines()
                        .any(|l| crate::width::display_width(l) > line_budget);
                    let truncated_lines = total_lines > max_preview_lines;
                    let truncated = truncated_lines || any_line_too_wide;

                    let toggle_hint = if truncated {
                        let label = if truncated_lines {
                            format!(" ({total_lines} lines — Ctrl+E to expand)")
                        } else {
                            format!(" ({total_chars} chars — Ctrl+E to expand)")
                        };
                        Span::styled(
                            label,
                            Style::default()
                                .fg(theme::CHROME)
                                .add_modifier(Modifier::ITALIC),
                        )
                    } else {
                        Span::raw("")
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  ▸ ", label_style),
                        Span::styled(format!("{name} result"), label_style),
                        toggle_hint,
                    ]));
                    for result_line in msg.content.lines().take(max_preview_lines) {
                        let display = crate::width::truncate_display(result_line, line_budget);
                        lines.push(Line::from(vec![
                            Span::raw("    "),
                            Span::styled(display, result_text),
                        ]));
                    }
                    if truncated_lines {
                        lines.push(Line::from(vec![
                            Span::raw("    "),
                            Span::styled(
                                format!(
                                    "… {remaining} more",
                                    remaining = total_lines - max_preview_lines
                                ),
                                Style::default()
                                    .fg(theme::RESULT_HINT)
                                    .add_modifier(Modifier::ITALIC),
                            ),
                        ]));
                    }
                }
            }
            ChatRole::System => {
                let gutter_style = Style::default().fg(theme::SYSTEM);
                lines.push(Line::from(Span::styled(
                    "│ system:",
                    gutter_style.add_modifier(Modifier::ITALIC),
                )));
                let content_width = chat_width.saturating_sub(4);
                for md_line in msg.rendered_lines(content_width) {
                    let mut prefixed = vec![Span::styled("│ ", gutter_style)];
                    prefixed.extend(md_line.spans.clone());
                    lines.push(Line::from(prefixed));
                }
            }
        }
    }

    // Streaming text (in progress)
    if !app.streaming_text.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(theme::CHROME)),
            Span::raw(app.streaming_text.clone()),
            Span::styled("▌", Style::default().fg(theme::BORDER_ACTIVE)),
        ]));
    } else if app.is_generating {
        // Waiting for first token — show thinking indicator with elapsed time
        let frame_char = SPINNER_FRAMES[app.spinner_frame as usize % SPINNER_FRAMES.len()];
        let elapsed = app
            .generation_started
            .map(|t| format!(" {:.1}s", t.elapsed().as_secs_f64()))
            .unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled(format!("{frame_char} "), Style::default().fg(theme::BLUE)),
            Span::styled(
                format!("thinking...{elapsed}"),
                Style::default()
                    .fg(theme::OVERLAY0)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    } else if last_is_completed_turn {
        // End-of-response marker. Subtle dim horizontal rule beneath the
        // last completed turn so the user has a clear "agent is done,
        // your turn" signal — otherwise long output runs into the input
        // area with no visual delimiter.
        let rule_width = chat_width.saturating_sub(2).max(3);
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(rule_width)),
            Style::default().fg(theme::SEPARATOR),
        )));
    }

    // Pre-wrap to the chat area's exact width. We own the wrap so the
    // visual-line count is exact, and so the chat renderer can slice the
    // visible window directly — no `Paragraph::wrap` + estimated-scroll
    // mismatch (which used to clip the bottom of long messages).
    let content_width = (area.width as usize).max(1);
    let wrapped = crate::wrap::wrap_lines(lines, content_width);
    let visible_height = area.height as usize;
    let total = wrapped.len();
    let auto_scroll_pos = total.saturating_sub(visible_height);

    // Clamp scroll_offset to valid range — can't scroll past the top.
    if app.scroll_offset > auto_scroll_pos {
        app.scroll_offset = auto_scroll_pos;
    }
    let start = if app.scroll_offset == 0 {
        auto_scroll_pos
    } else {
        auto_scroll_pos.saturating_sub(app.scroll_offset)
    };
    let end = (start + visible_height).min(total);
    let view: Vec<Line<'static>> = wrapped[start..end].to_vec();

    // Flat blit — no Wrap, no scroll. The pre-wrapped slice is already the
    // exact visible content for this Rect.
    let chat = Paragraph::new(view);
    frame.render_widget(chat, area);
}

pub(super) fn render_separator(frame: &mut Frame, area: ratatui::layout::Rect) {
    let line = "─".repeat(area.width as usize);
    let sep = Paragraph::new(line).style(Style::default().fg(theme::SEPARATOR));
    frame.render_widget(sep, area);
}

/// I-0035 Phase 2 (T-0354): render the cached brief markdown in the
/// empty-chat area. Inherits the same `markdown_to_lines_with_width`
/// pipeline as assistant messages so the brief reads visually
/// identically whether shown via `/brief` or auto-rendered here.
pub(super) fn render_empty_chat_brief(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let Some(md) = app.brief_markdown.as_deref() else {
        return;
    };
    // 2-cell left/right margin so the brief breathes inside the chat
    // pane and lines wrap at the right column.
    let content_width = area.width.saturating_sub(4) as usize;
    if content_width < 10 {
        return;
    }
    let lines = crate::markdown::markdown_to_lines_with_width(md, content_width);
    let inner = ratatui::layout::Rect::new(
        area.x + 2,
        area.y,
        area.width.saturating_sub(4),
        area.height,
    );
    let para = ratatui::widgets::Paragraph::new(lines).alignment(ratatui::layout::Alignment::Left);
    frame.render_widget(para, inner);
}

pub(super) fn render_idle_hero(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    no_model: bool,
) {
    let chrome = Style::default().fg(theme::CHROME);
    let dim = Style::default().fg(theme::SUBTEXT0);
    let hint = Style::default().fg(theme::OVERLAY1);
    let warn = Style::default().fg(theme::YELLOW);

    let mut hero_lines: Vec<Line> = vec![
        Line::from(Span::styled("╭─────────────╮", chrome)),
        Line::from(vec![
            Span::styled("│    ", chrome),
            Span::styled("arawn", dim.add_modifier(Modifier::BOLD)),
            Span::styled("    │", chrome),
        ]),
        Line::from(Span::styled("╰─────────────╯", chrome)),
        Line::from(""),
        Line::from(Span::styled(
            "Welcome — your personal agentic assistant.",
            dim,
        )),
        Line::from(Span::styled(
            "I watch, check, summarize, and nudge across your tools.",
            dim,
        )),
        Line::from(""),
    ];

    // ARAWN-I-0061: actionable first-run guidance when no LLM provider is
    // configured — otherwise this screen would lead the user into a typing
    // session that silently fails.
    if no_model {
        hero_lines.extend([
            Line::from(Span::styled("No LLM provider configured.", warn)),
            Line::from(Span::styled("Run `arawn doctor` to diagnose.", hint)),
            Line::from(Span::styled(
                "Edit `~/.arawn/arawn.toml` to set one.",
                hint,
            )),
            Line::from(""),
        ]);
    }

    hero_lines.extend([
        Line::from(Span::styled(
            "Type / for commands · Tab to toggle sidebar",
            hint,
        )),
        Line::from(Span::styled("/connect <service> · ↑ recall", hint)),
    ]);

    let hero_height = hero_lines.len() as u16;
    let hero_width = 56u16;
    if area.width < hero_width || area.height < hero_height {
        return;
    }
    let x = area.x + (area.width.saturating_sub(hero_width)) / 2;
    let y = area.y + (area.height.saturating_sub(hero_height)) / 2;
    let rect = ratatui::layout::Rect::new(x, y, hero_width, hero_height);
    let para = Paragraph::new(hero_lines).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(para, rect);
}

/// ARAWN-I-0061: collect the distinct source lenses from `signal_*` tool
/// results in the turn ending at `assistant_idx`. Walks backward from the
/// assistant message until the previous User/System message, parses each
/// matching `ToolResult` payload as JSON, and harvests every `"lens"` field's
/// string value. Sorted + deduped for stable rendering.
pub(super) fn collect_signal_sources_for_turn(
    messages: &[crate::app::ChatMessage],
    assistant_idx: usize,
) -> Vec<String> {
    use std::collections::BTreeSet;

    let mut set: BTreeSet<String> = BTreeSet::new();
    for i in (0..assistant_idx).rev() {
        match &messages[i].role {
            ChatRole::User | ChatRole::System => break,
            ChatRole::ToolResult {
                name,
                is_error: false,
            } if name.starts_with("signal_") => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&messages[i].content) {
                    harvest_lens_strings(&v, &mut set);
                }
            }
            _ => {}
        }
    }
    set.into_iter().collect()
}

fn harvest_lens_strings(v: &serde_json::Value, out: &mut std::collections::BTreeSet<String>) {
    match v {
        serde_json::Value::Object(map) => {
            for (k, val) in map {
                if k == "lens"
                    && let serde_json::Value::String(s) = val
                    && !s.is_empty()
                {
                    out.insert(s.clone());
                }
                harvest_lens_strings(val, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                harvest_lens_strings(item, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{ChatMessage, ChatRole};

    fn msg(role: ChatRole, content: &str) -> ChatMessage {
        ChatMessage::new(role, content)
    }

    #[test]
    fn chip_collects_lenses_from_signal_search_result() {
        let messages = vec![
            msg(ChatRole::User, "what happened?"),
            msg(
                ChatRole::ToolCall {
                    name: "signal_search".into(),
                },
                r#"{"query":"x"}"#,
            ),
            msg(
                ChatRole::ToolResult {
                    name: "signal_search".into(),
                    is_error: false,
                },
                r#"{"results":[{"lens":"work","title":"a"},{"lens":"personal","title":"b"},{"lens":"work","title":"c"}]}"#,
            ),
            msg(ChatRole::Assistant, "done"),
        ];
        let sources = collect_signal_sources_for_turn(&messages, 3);
        assert_eq!(sources, vec!["personal".to_string(), "work".to_string()]);
    }

    #[test]
    fn chip_empty_when_no_signal_tool_in_turn() {
        let messages = vec![
            msg(ChatRole::User, "hi"),
            msg(
                ChatRole::ToolCall {
                    name: "feed_search".into(),
                },
                r#"{}"#,
            ),
            msg(
                ChatRole::ToolResult {
                    name: "feed_search".into(),
                    is_error: false,
                },
                r#"{"results":[{"lens":"work","title":"x"}]}"#,
            ),
            msg(ChatRole::Assistant, "answer"),
        ];
        // feed_search is not signal_*; chip should be empty even though the
        // payload contains a `lens` field.
        let sources = collect_signal_sources_for_turn(&messages, 3);
        assert!(sources.is_empty());
    }

    #[test]
    fn chip_stops_at_previous_user_message() {
        // A signal_* result from a *prior* turn must not bleed into this
        // turn's chip.
        let messages = vec![
            msg(
                ChatRole::ToolResult {
                    name: "signal_search".into(),
                    is_error: false,
                },
                r#"{"results":[{"lens":"old"}]}"#,
            ),
            msg(ChatRole::User, "new question"),
            msg(ChatRole::Assistant, "answer with no signal calls"),
        ];
        let sources = collect_signal_sources_for_turn(&messages, 2);
        assert!(sources.is_empty());
    }

    #[test]
    fn chip_ignores_error_results() {
        let messages = vec![
            msg(ChatRole::User, "what?"),
            msg(
                ChatRole::ToolCall {
                    name: "signal_search".into(),
                },
                r#"{}"#,
            ),
            msg(
                ChatRole::ToolResult {
                    name: "signal_search".into(),
                    is_error: true,
                },
                r#"{"results":[{"lens":"work"}]}"#,
            ),
            msg(ChatRole::Assistant, "no answer"),
        ];
        let sources = collect_signal_sources_for_turn(&messages, 3);
        assert!(sources.is_empty());
    }
}
