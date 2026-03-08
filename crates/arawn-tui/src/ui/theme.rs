//! Centralized color theme for the TUI.
//!
//! All colors and styles are defined here so the palette can be tuned
//! in one place. Uses `Color::Rgb` for precise control — avoids
//! bloom/glow artifacts common with full-bright ANSI colors on
//! renderers like Warp, Hyper, or retro/synthwave terminal themes.

use ratatui::style::{Color, Modifier, Style};

// ─────────────────────────────────────────────────────────────────────────────
// Base palette — muted RGB tones that stay readable without blooming
// ─────────────────────────────────────────────────────────────────────────────

/// Primary accent color (interactive elements, focused borders, user prefix).
/// Muted teal — visible but doesn't bloom under glow.
pub const ACCENT: Color = Color::Rgb(90, 190, 210);

/// Secondary accent (tool pane headers, panel-specific highlights).
/// Soft blue — distinct from accent without competing.
pub const ACCENT2: Color = Color::Rgb(110, 140, 200);

/// Tertiary accent (sidebar section labels, tags).
/// Muted lavender — gentle contrast for subheaders.
pub const ACCENT3: Color = Color::Rgb(170, 130, 190);

/// Status: success.
pub const OK: Color = Color::Rgb(100, 200, 120);

/// Status: warning.
pub const WARN: Color = Color::Rgb(220, 180, 80);

/// Status: error / danger.
pub const ERR: Color = Color::Rgb(220, 90, 90);

// ─────────────────────────────────────────────────────────────────────────────
// Text hierarchy — four distinct brightness levels
// ─────────────────────────────────────────────────────────────────────────────

/// Primary text — user messages, important content.
/// Soft white — high contrast without bloom.
pub const TEXT_PRIMARY: Color = Color::Rgb(210, 210, 220);

/// Normal text — assistant messages, list items, readable body.
/// Light gray — comfortable for reading.
pub const TEXT_NORMAL: Color = Color::Rgb(165, 165, 175);

/// Secondary text — labels, metadata, timestamps.
/// Medium gray — clearly subordinate.
pub const TEXT_SECONDARY: Color = Color::Rgb(110, 110, 125);

/// Muted text — hints, disabled items, truly de-emphasized.
/// Dark gray — visible but unobtrusive.
pub const TEXT_MUTED: Color = Color::Rgb(75, 75, 90);

// ─────────────────────────────────────────────────────────────────────────────
// Chrome — borders, separators, backgrounds
// ─────────────────────────────────────────────────────────────────────────────

/// Default border color (unfocused panels).
pub const BORDER: Color = Color::Rgb(70, 70, 85);

/// Focused border color.
pub const BORDER_FOCUSED: Color = Color::Rgb(80, 160, 180);

/// Separator lines between messages / tool cards.
pub const SEPARATOR: Color = Color::Rgb(60, 60, 75);

// ─────────────────────────────────────────────────────────────────────────────
// Semantic styles — pre-built for common patterns
// ─────────────────────────────────────────────────────────────────────────────

/// Section header style (panel titles, section labels).
/// No BOLD — accent color alone provides enough emphasis without bloom.
pub fn header() -> Style {
    Style::default().fg(ACCENT)
}

/// Subheader or category label.
pub fn subheader() -> Style {
    Style::default().fg(ACCENT3)
}

/// Selected / highlighted item in a list.
pub fn selected() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Normal list item.
pub fn list_item() -> Style {
    Style::default().fg(TEXT_NORMAL)
}

/// Dimmed / secondary list item.
pub fn list_item_dim() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

/// Keyboard shortcut label in help text.
pub fn key_hint() -> Style {
    Style::default().fg(TEXT_NORMAL)
}

/// Description text next to a key hint.
pub fn key_desc() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

/// User message prefix style (the `> `).
pub fn user_prefix() -> Style {
    Style::default().fg(ACCENT)
}

/// User message content.
pub fn user_text() -> Style {
    Style::default().fg(TEXT_PRIMARY)
}

/// Assistant message text.
pub fn assistant_text() -> Style {
    Style::default().fg(TEXT_NORMAL)
}

/// Streaming (in-progress) assistant text.
pub fn streaming_text() -> Style {
    Style::default().fg(TEXT_PRIMARY)
}

/// Tool name badge.
pub fn tool_name() -> Style {
    Style::default().fg(ACCENT2)
}

/// Tool arguments / preview text.
pub fn tool_preview() -> Style {
    Style::default().fg(TEXT_NORMAL)
}

/// Tool duration / timing info.
pub fn tool_duration() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

/// Status bar text.
pub fn status_bar() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

/// Search / filter prompt text.
pub fn search_prompt() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

/// Empty state / placeholder text.
pub fn empty_state() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

/// Scroll position indicator.
pub fn scroll_indicator() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

/// Border style for an unfocused panel.
pub fn border() -> Style {
    Style::default().fg(BORDER)
}

/// Border style for a focused panel.
pub fn border_focused() -> Style {
    Style::default().fg(BORDER_FOCUSED)
}

/// Separator line between items.
pub fn separator() -> Style {
    Style::default().fg(SEPARATOR)
}

/// Warning banner style.
pub fn warning_banner() -> Style {
    Style::default().fg(WARN).add_modifier(Modifier::BOLD)
}
