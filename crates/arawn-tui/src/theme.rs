//! Centralized TUI theme — colors and styles used across render.rs and markdown.rs.
//!
//! Change colors here to restyle the entire TUI in one place.

use ratatui::style::{Color, Modifier, Style};

// ── Semantic colors ──────────────────────────────────────────────────────────

/// User message prefix ("You:")
pub const USER: Color = Color::Green;

/// Assistant message prefix ("Arawn:")
pub const ASSISTANT: Color = Color::Blue;

/// System message prefix
pub const SYSTEM: Color = Color::Magenta;

/// Error text and indicators
pub const ERROR: Color = Color::Red;

/// Tool name in tool calls
pub const TOOL_NAME: Color = Color::Cyan;

/// Generating / in-progress indicator
pub const GENERATING: Color = Color::Yellow;

/// Success indicator (✓)
pub const SUCCESS: Color = Color::Green;

// ── Chrome / structural elements ─────────────────────────────────────────────

/// Box borders around tool calls/results (┌│└)
pub const CHROME: Color = Color::Rgb(100, 100, 115);

/// Separator line between chat and input
pub const SEPARATOR: Color = Color::DarkGray;

/// Status bar background. The matching value is what render.rs has been
/// using as a literal (`Rgb(30, 30, 40)`) — the constant previously
/// said `DarkGray`, which lied about the actual rendered color.
pub const STATUS_BAR_BG: Color = Color::Rgb(30, 30, 40);

/// Status bar text
pub const STATUS_BAR_FG: Color = Color::White;

/// Sidebar border (unfocused)
pub const BORDER_INACTIVE: Color = Color::DarkGray;

/// Sidebar border (focused)
pub const BORDER_ACTIVE: Color = Color::Cyan;

/// Sidebar tab strip background
pub const SIDEBAR_TAB_BG: Color = Color::Rgb(25, 25, 30);

// ── Tool result text ─────────────────────────────────────────────────────────

/// Tool result content text
pub const RESULT_TEXT: Color = Color::Rgb(150, 150, 165);

/// Tool result labels ("▸ shell result")
pub const RESULT_LABEL: Color = Color::Rgb(130, 130, 145);

/// Tool input summary text (args after tool name)
pub const TOOL_SUMMARY: Color = Color::Rgb(140, 140, 155);

/// Truncation hints ("… 15 more")
pub const RESULT_HINT: Color = Color::Rgb(120, 120, 135);

// ── Input area ───────────────────────────────────────────────────────────────

/// Input prompt "> "
pub const INPUT_PROMPT: Color = Color::Green;

/// Placeholder text ("Type your message...")
pub const PLACEHOLDER: Color = Color::DarkGray;

// ── Code blocks ──────────────────────────────────────────────────────────────

/// Code block background
pub const CODE_BG: Color = Color::Rgb(30, 30, 40);

/// Code block text (fallback when no syntax highlighting)
pub const CODE_FG: Color = Color::Rgb(180, 180, 180);

/// Inline code text
pub const INLINE_CODE_FG: Color = Color::Rgb(220, 170, 110);

/// Inline code background
pub const INLINE_CODE_BG: Color = Color::Rgb(40, 40, 50);

/// Code block language label
pub const CODE_LANG: Color = Color::DarkGray;

// ── Markdown headings ────────────────────────────────────────────────────────

pub const HEADING_1: Color = Color::Cyan;
pub const HEADING_2: Color = Color::Blue;
pub const HEADING_3: Color = Color::Magenta;
pub const HEADING_4: Color = Color::White;

// ── Markdown misc ────────────────────────────────────────────────────────────

/// Horizontal rules
pub const RULE: Color = Color::DarkGray;

/// List bullet/number prefix
pub const LIST_BULLET: Color = Color::DarkGray;

/// Block quote text
pub const BLOCK_QUOTE: Color = Color::DarkGray;

/// Link text
pub const LINK: Color = Color::Cyan;

/// Link URL shown after link text
pub const LINK_URL: Color = Color::DarkGray;

/// Table chrome (│ ├ ┼ ┤)
pub const TABLE_CHROME: Color = Color::DarkGray;

// ── Composite styles (color + modifier combos used frequently) ───────────────

pub const fn bold(color: Color) -> Style {
    Style::new().fg(color).add_modifier(Modifier::BOLD)
}

pub const fn italic(color: Color) -> Style {
    Style::new().fg(color).add_modifier(Modifier::ITALIC)
}
