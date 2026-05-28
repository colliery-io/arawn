use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::theme;

pub(super) fn render_dashboard_pane(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let border_color = theme::BORDER_INACTIVE;
    let block = Block::default()
        .title(" Today ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Vertical split: brief section gets up to BRIEF_HEIGHT rows;
    // action items section takes the rest. Falls back to a single
    // section when the pane is too short to be useful.
    const BRIEF_HEIGHT: u16 = 12;
    if inner.height >= BRIEF_HEIGHT + 4 {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(BRIEF_HEIGHT), Constraint::Min(2)])
            .split(inner);
        render_dashboard_brief(app, frame, split[0]);
        render_dashboard_actions(app, frame, split[1]);
    } else {
        render_dashboard_brief(app, frame, inner);
    }
}

/// I-0035 Phase 3 T-B (T-0357) — compact brief summary in the
/// top portion of the dashboard pane. Shows date header + up to
/// `MAX_CAL` calendar bullets, with an inline `⚠ conflict` marker
/// when two events overlap.
pub(super) fn render_dashboard_brief(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    if area.width < 6 || area.height < 1 {
        return;
    }
    let strong = Style::default()
        .fg(theme::TEXT)
        .add_modifier(Modifier::BOLD);
    let muted = Style::default().fg(theme::SUBTEXT0);
    let warn = Style::default().fg(theme::YELLOW);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::styled("Brief".to_string(), strong)));

    let Some(view) = app.daily_view.as_ref() else {
        lines.push(Line::from(Span::styled(
            "(no brief yet — try /brief or /today)".to_string(),
            muted,
        )));
        let para = Paragraph::new(lines);
        frame.render_widget(para, area);
        return;
    };

    // Date line: tablet's period_key (`YYYY-MM-DD`) + weekday from
    // chrono if parseable, else just the period_key.
    let date_line = format_brief_date_line(&view.tablet.period_key);
    lines.push(Line::from(Span::styled(date_line, muted)));
    lines.push(Line::from(""));

    let calendar_items: Vec<&arawn_ceremonies::service::ItemDto> = view
        .items
        .iter()
        .filter(|i| i.section_key == "calendar")
        .collect();
    if calendar_items.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no calendar events today)".to_string(),
            muted,
        )));
    } else {
        const MAX_CAL: usize = 4;
        // Inner text budget = area.width - 2 (bullet "• ").
        let text_budget = (area.width as usize).saturating_sub(2);
        for item in calendar_items.iter().take(MAX_CAL) {
            let row = format_calendar_row(item, text_budget);
            lines.push(Line::from(vec![Span::raw("• "), Span::raw(row)]));
        }
        if calendar_items.len() > MAX_CAL {
            lines.push(Line::from(Span::styled(
                format!("… +{} more", calendar_items.len() - MAX_CAL),
                muted,
            )));
        }
        if let Some(conflict_line) = detect_conflict(&calendar_items) {
            lines.push(Line::from(Span::styled(conflict_line, warn)));
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

/// I-0035 Phase 3 T-C (T-0358) — read-only action items list in
/// the bottom portion of the dashboard pane. Renders the daily
/// tablet's `attention` section as checkbox rows, with a
/// `─ Carried over ─` separator + carried-over rolling todos
/// below when both are present. Truncates with `…` to fit pane
/// width; overflows with `… +K more` footer.
pub(super) fn render_dashboard_actions(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    if area.width < 6 || area.height < 1 {
        return;
    }
    let strong = Style::default()
        .fg(theme::TEXT)
        .add_modifier(Modifier::BOLD);
    let muted = Style::default().fg(theme::SUBTEXT0);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::styled("Action items".to_string(), strong)));

    let Some(view) = app.daily_view.as_ref() else {
        lines.push(Line::from(Span::styled(
            "(no action items)".to_string(),
            muted,
        )));
        frame.render_widget(Paragraph::new(lines), area);
        return;
    };

    let attention: Vec<&arawn_ceremonies::service::ItemDto> = view
        .items
        .iter()
        .filter(|i| i.section_key == "attention")
        .collect();
    let carried: Vec<&arawn_ceremonies::service::ItemDto> = view
        .items
        .iter()
        .filter(|i| i.section_key == "todos")
        .collect();

    if attention.is_empty() && carried.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no action items)".to_string(),
            muted,
        )));
        frame.render_widget(Paragraph::new(lines), area);
        return;
    }

    // Text budget: width - 2 (checkbox glyph + padding).
    let text_budget = (area.width as usize).saturating_sub(2);
    // Row budget: total area height - 1 (header). When we have
    // attention items AND carried-over todos, reserve 1 row for
    // the separator.
    let used_so_far = 1usize; // header
    let has_separator = !attention.is_empty() && !carried.is_empty();
    let separator_cost = if has_separator { 1 } else { 0 };
    let total_rows = area.height as usize;
    let body_budget = total_rows
        .saturating_sub(used_so_far)
        .saturating_sub(separator_cost);

    // Allocate budget across the two sections. Attention items get
    // priority — they're the most actionable. Carried-over get
    // whatever's left after attention's rows + overflow footer.
    let attention_alloc = if carried.is_empty() {
        body_budget
    } else {
        // Leave at least 2 rows for the carried section (1 item +
        // optional overflow footer). Squeeze attention to fit.
        body_budget.saturating_sub(2).max(1)
    };

    push_action_rows(&mut lines, &attention, attention_alloc, text_budget, muted);

    if has_separator {
        lines.push(Line::from(Span::styled(
            "─ Carried over ─".to_string(),
            muted,
        )));
        let carried_alloc = total_rows.saturating_sub(lines.len()).max(1);
        push_action_rows(&mut lines, &carried, carried_alloc, text_budget, muted);
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

/// Push action rows (checkbox + truncated body) into `lines`,
/// honoring a row budget. When `items.len() > budget`, render the
/// first `budget - 1` items plus a `… +K more` footer line.
pub(super) fn push_action_rows(
    lines: &mut Vec<Line<'static>>,
    items: &[&arawn_ceremonies::service::ItemDto],
    budget: usize,
    text_budget: usize,
    muted_style: Style,
) {
    if budget == 0 || items.is_empty() {
        return;
    }
    if items.len() <= budget {
        for item in items {
            lines.push(Line::from(vec![
                Span::raw("☐ "),
                Span::raw(format_action_row(item, text_budget)),
            ]));
        }
    } else {
        let show = budget.saturating_sub(1).max(1);
        let hidden = items.len() - show;
        for item in items.iter().take(show) {
            lines.push(Line::from(vec![
                Span::raw("☐ "),
                Span::raw(format_action_row(item, text_budget)),
            ]));
        }
        lines.push(Line::from(Span::styled(
            format!("… +{hidden} more"),
            muted_style,
        )));
    }
}

pub(super) fn format_action_row(
    item: &arawn_ceremonies::service::ItemDto,
    budget: usize,
) -> String {
    let title = item
        .body
        .get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            item.body
                .get("summary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            item.body
                .get("subject")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| serde_json::to_string(&item.body).unwrap_or_default());
    crate::width::truncate_display(&title, budget)
}

/// Format the brief's date line: `YYYY-MM-DD · Wed` when the
/// period_key parses; otherwise the raw period_key.
pub(super) fn format_brief_date_line(period_key: &str) -> String {
    use chrono::Datelike;
    chrono::NaiveDate::parse_from_str(period_key, "%Y-%m-%d")
        .map(|d| {
            let wd = match d.weekday() {
                chrono::Weekday::Mon => "Mon",
                chrono::Weekday::Tue => "Tue",
                chrono::Weekday::Wed => "Wed",
                chrono::Weekday::Thu => "Thu",
                chrono::Weekday::Fri => "Fri",
                chrono::Weekday::Sat => "Sat",
                chrono::Weekday::Sun => "Sun",
            };
            format!("{period_key} · {wd}")
        })
        .unwrap_or_else(|_| period_key.to_string())
}

/// Render one calendar row: `HH:MM <title>` when body has a
/// `start_ts`, else `<title>` falling back to body.text.
pub(super) fn format_calendar_row(
    item: &arawn_ceremonies::service::ItemDto,
    budget: usize,
) -> String {
    let title = item
        .body
        .get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            item.body
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        });
    let time = item
        .body
        .get("start_ts")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.format("%H:%M").to_string());
    let raw = match time {
        Some(t) => format!("{t} {title}"),
        None => title,
    };
    crate::width::truncate_display(&raw, budget)
}

/// Pure conflict detection over calendar items. Looks at consecutive
/// items in tablet order; returns a one-line warning like
/// "⚠ conflict 14:00–15:00" for the first overlap found. Returns
/// None when no parseable timestamps overlap.
pub(super) fn detect_conflict(items: &[&arawn_ceremonies::service::ItemDto]) -> Option<String> {
    let mut events: Vec<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> = items
        .iter()
        .filter_map(|i| {
            let start = i
                .body
                .get("start_ts")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())?
                .with_timezone(&chrono::Utc);
            let end = i
                .body
                .get("end_ts")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))
                // No end_ts → assume a 30-min default so we can still
                // detect overlap with a later event.
                .unwrap_or_else(|| start + chrono::Duration::minutes(30));
            Some((start, end))
        })
        .collect();
    events.sort_by_key(|e| e.0);
    for pair in events.windows(2) {
        if pair[1].0 < pair[0].1 {
            return Some(format!(
                "⚠ conflict {}–{}",
                pair[1].0.format("%H:%M"),
                pair[0].1.format("%H:%M")
            ));
        }
    }
    None
}
