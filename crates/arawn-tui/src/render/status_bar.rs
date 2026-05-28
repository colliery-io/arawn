use ratatui::Frame;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::SPINNER_FRAMES;
use crate::app::App;
use crate::theme;

pub(super) fn render_status_bar(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let bar_style = Style::default()
        .fg(theme::STATUS_BAR_FG)
        .bg(theme::STATUS_BAR_BG);
    let dim = Style::default()
        .fg(theme::OVERLAY0)
        .bg(theme::STATUS_BAR_BG);
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

    // ARAWN-I-0061: no write-target indicator. Memory is global and signals are
    // read across every lens, so there is no "current lens" to show here — lenses
    // are standing extractors, not a place you switch into.

    // Permission mode (T-0347: renamed to /autonomy vocabulary —
    // ask | edits | full | plan).
    if app.permission_mode != "ask" {
        spans.push(Span::styled(" │ ", dim));
        let (label, color) = match app.permission_mode.as_str() {
            "full" => ("FULL", theme::ERROR),
            "edits" => ("EDITS", theme::YELLOW),
            "plan" => ("PLAN", theme::SAPPHIRE),
            _ => ("ASK", theme::TEXT),
        };
        spans.push(Span::styled(
            label.to_string(),
            Style::default()
                .fg(color)
                .bg(crate::theme::STATUS_BAR_BG)
                .add_modifier(ratatui::style::Modifier::BOLD),
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
            Style::default()
                .fg(theme::GENERATING)
                .bg(theme::STATUS_BAR_BG),
        ));
        // Elapsed time
        if let Some(started) = app.generation_started {
            let elapsed = started.elapsed().as_secs();
            if elapsed >= 2 {
                spans.push(Span::styled(
                    format!(" {elapsed}s"),
                    Style::default()
                        .fg(theme::OVERLAY0)
                        .bg(theme::STATUS_BAR_BG),
                ));
            }
        }
    } else {
        spans.push(Span::styled(
            " Ready",
            Style::default().fg(theme::SUCCESS).bg(theme::STATUS_BAR_BG),
        ));
    }

    let status =
        Paragraph::new(Line::from(spans)).style(Style::default().bg(crate::theme::STATUS_BAR_BG));
    frame.render_widget(status, area);
}

/// Format a token count for display: 1234 → "1.2k", 12345 → "12.3k", 500 → "500"
pub(super) fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
