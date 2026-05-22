use ratatui::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::theme;

pub(super) fn render_toast_bar(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let Some(toast) = app.toast_queue.front() else {
        return;
    };
    let fg = match toast.level {
        crate::toast::ToastLevel::Info => theme::SUBTEXT0,
        crate::toast::ToastLevel::Warn => theme::YELLOW,
        crate::toast::ToastLevel::Error => theme::RED,
    };
    let style = Style::default().fg(fg).add_modifier(Modifier::BOLD);
    let budget = (area.width as usize).saturating_sub(2);
    let text = crate::width::truncate_display(&toast.message, budget);
    let line = Line::from(vec![Span::raw(" "), Span::styled(text, style)]);
    let para = Paragraph::new(vec![line]);
    frame.render_widget(para, area);
}

pub(super) fn render_oauth_heartbeat(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let Some((svc, started)) = &app.oauth_in_flight else {
        return;
    };
    let elapsed = started.elapsed().as_secs();
    // Pulse the bullet between yellow and overlay every other second
    // using the existing spinner_frame so no new tick state is needed.
    let pulse_on = (app.spinner_frame / 5).is_multiple_of(2);
    let bullet_color = if pulse_on {
        theme::GENERATING
    } else {
        theme::OVERLAY0
    };
    let dim = Style::default().fg(theme::OVERLAY1);
    let line = Line::from(vec![
        Span::raw(" "),
        Span::styled("•", Style::default().fg(bullet_color)),
        Span::styled(
            format!(" waiting for {svc} OAuth in browser… {elapsed}s · Esc to cancel"),
            dim,
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

