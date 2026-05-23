use ratatui::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::{App, Focus, SidebarSection};
use crate::theme;

pub(super) fn render_sidebar_tab(frame: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(theme::BORDER_INACTIVE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mid = inner.height / 2;
    let bg_style = Style::default().bg(theme::SIDEBAR_TAB_BG);
    let mut lines: Vec<Line> = Vec::new();
    for i in 0..inner.height {
        if i == mid {
            lines.push(Line::from(Span::styled(
                " ▸",
                Style::default()
                    .fg(theme::BORDER_ACTIVE)
                    .bg(theme::SIDEBAR_TAB_BG),
            )));
        } else {
            lines.push(Line::from(Span::styled("  ", bg_style)));
        }
    }
    let widget = Paragraph::new(lines).style(bg_style);
    frame.render_widget(widget, inner);
}

pub(super) fn render_sidebar(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let border_color = if app.focus == Focus::Sidebar {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    // I-0035 Phase 3 T-A: sidebar is Workstreams-only. The previous
    // Sessions sub-section was removed; sessions are accessible via
    // `/session list`.
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
    frame.render_widget(ws_list, area);
}

