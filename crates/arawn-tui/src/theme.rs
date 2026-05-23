//! Centralized TUI theme — Catppuccin Mocha palette + semantic aliases.
//!
//! Two layers:
//!
//! - **Palette layer**: Catppuccin Mocha hexes baked in as `Color::Rgb(...)`
//!   constants (https://catppuccin.com/palette/). Defined once here. No
//!   runtime dep — these are compile-time constants.
//!
//! - **Semantic layer**: named aliases (`USER`, `ASSISTANT`, `CHROME`,
//!   `BORDER_FOCUSED`, etc.) that map to palette values. Call sites import
//!   the semantic names — never the palette directly. This lets later
//!   phases reassign meanings without touching the palette.
//!
//! Per ARAWN-I-0036, `accent` (mauve) is reserved exclusively for
//! "focused / active interactive" — the only legitimate uses are
//! `BORDER_ACTIVE` and the input-focus indicator. Don't alias mauve into
//! tool names, headings, links, etc.

use ratatui::style::{Color, Modifier, Style};

// ─────────────────────────────────────────────────────────────────────────
// Palette — Catppuccin Mocha
// ─────────────────────────────────────────────────────────────────────────

// Surface scale (darkest → lightest)
pub const CRUST: Color = Color::Rgb(0x11, 0x11, 0x1B);
pub const MANTLE: Color = Color::Rgb(0x18, 0x18, 0x25);
pub const BASE: Color = Color::Rgb(0x1E, 0x1E, 0x2E);
pub const SURFACE0: Color = Color::Rgb(0x31, 0x32, 0x44);
pub const SURFACE1: Color = Color::Rgb(0x45, 0x47, 0x5A);
pub const SURFACE2: Color = Color::Rgb(0x58, 0x5B, 0x70);

// Overlay scale (mid grays)
pub const OVERLAY0: Color = Color::Rgb(0x6C, 0x70, 0x86);
pub const OVERLAY1: Color = Color::Rgb(0x7F, 0x84, 0x9C);
pub const OVERLAY2: Color = Color::Rgb(0x93, 0x99, 0xB2);

// Text scale (lightest grays / fg)
pub const SUBTEXT0: Color = Color::Rgb(0xA6, 0xAD, 0xC8);
pub const SUBTEXT1: Color = Color::Rgb(0xBA, 0xC2, 0xDE);
pub const TEXT: Color = Color::Rgb(0xCD, 0xD6, 0xF4);

// Accent / status colors
pub const LAVENDER: Color = Color::Rgb(0xB4, 0xBE, 0xFE);
pub const BLUE: Color = Color::Rgb(0x89, 0xB4, 0xFA);
pub const SAPPHIRE: Color = Color::Rgb(0x74, 0xC7, 0xEC);
pub const SKY: Color = Color::Rgb(0x89, 0xDC, 0xEB);
pub const TEAL: Color = Color::Rgb(0x94, 0xE2, 0xD5);
pub const GREEN: Color = Color::Rgb(0xA6, 0xE3, 0xA1);
pub const YELLOW: Color = Color::Rgb(0xF9, 0xE2, 0xAF);
pub const PEACH: Color = Color::Rgb(0xFA, 0xB3, 0x87);
pub const MAROON: Color = Color::Rgb(0xEB, 0xA0, 0xAC);
pub const RED: Color = Color::Rgb(0xF3, 0x8B, 0xA8);
pub const MAUVE: Color = Color::Rgb(0xCB, 0xA6, 0xF7);
pub const PINK: Color = Color::Rgb(0xF5, 0xC2, 0xE7);
pub const FLAMINGO: Color = Color::Rgb(0xF2, 0xCD, 0xCD);
pub const ROSEWATER: Color = Color::Rgb(0xF5, 0xE0, 0xDC);

// ─────────────────────────────────────────────────────────────────────────
// Semantic layer — call sites import these, never palette directly
// ─────────────────────────────────────────────────────────────────────────

/// User message prefix ("❯ ")
pub const USER: Color = GREEN;

/// Assistant message body — the agent's prose, default reading color
pub const ASSISTANT: Color = TEXT;

/// System / internal note prefix
pub const SYSTEM: Color = MAUVE;

/// Errors and danger indicators
pub const ERROR: Color = RED;

/// Tool name in tool calls — interactive but not focused
pub const TOOL_NAME: Color = LAVENDER;

/// In-progress / generating indicator (spinner, "thinking…")
pub const GENERATING: Color = YELLOW;

/// Success indicator (✓)
pub const SUCCESS: Color = GREEN;

// ── Chrome / structural ─────────────────────────────────────────────────

/// Box borders around tool calls/results (┌│└)
pub const CHROME: Color = SURFACE1;

/// Separator line between chat and input
pub const SEPARATOR: Color = SURFACE0;

/// Status bar background — Catppuccin Mantle (one shade darker than base)
pub const STATUS_BAR_BG: Color = MANTLE;

/// Status bar foreground (default text color on the bar)
pub const STATUS_BAR_FG: Color = TEXT;

/// Sidebar border when not focused
pub const BORDER_INACTIVE: Color = SURFACE1;

/// Sidebar border when focused — accent. **The one place mauve goes.**
pub const BORDER_ACTIVE: Color = MAUVE;

/// Sidebar tab strip background (collapsed sidebar) — Catppuccin Crust
pub const SIDEBAR_TAB_BG: Color = CRUST;

// ── Tool result text ────────────────────────────────────────────────────

/// Tool result content text
pub const RESULT_TEXT: Color = SUBTEXT0;

/// Tool result labels ("▸ shell result")
pub const RESULT_LABEL: Color = OVERLAY2;

/// Tool input summary text (args after tool name)
pub const TOOL_SUMMARY: Color = OVERLAY2;

/// Truncation hints ("… 15 more")
pub const RESULT_HINT: Color = OVERLAY1;

// ── Input area ──────────────────────────────────────────────────────────

/// Input prompt "> "
pub const INPUT_PROMPT: Color = GREEN;

/// Placeholder text ("Type your message...")
pub const PLACEHOLDER: Color = OVERLAY0;

// ── Code blocks ─────────────────────────────────────────────────────────

/// Code block background
pub const CODE_BG: Color = MANTLE;

/// Code block text (fallback when no syntax highlighting)
pub const CODE_FG: Color = TEXT;

/// Inline code text — Catppuccin peach has a known "code" feel
pub const INLINE_CODE_FG: Color = PEACH;

/// Inline code background
pub const INLINE_CODE_BG: Color = SURFACE0;

/// Code block language label
pub const CODE_LANG: Color = OVERLAY0;

// ── Markdown headings ────────────────────────────────────────────────────
//
// Phase 1 keeps the existing four-color hierarchy but routed through
// Catppuccin colors that are NOT mauve (mauve is reserved for accent).
// Phase 3 (T-0210) replaces this with a pure monochrome ladder.

// Pure-monochrome ladder: H1 brightest, H4 dimmest. No accent.
pub const HEADING_1: Color = TEXT;
pub const HEADING_2: Color = SUBTEXT1;
pub const HEADING_3: Color = SUBTEXT0;
pub const HEADING_4: Color = OVERLAY2;

// ── Markdown misc ───────────────────────────────────────────────────────

/// Horizontal rules
pub const RULE: Color = SURFACE0;

/// List bullet/number prefix
pub const LIST_BULLET: Color = OVERLAY1;

/// Block quote text
pub const BLOCK_QUOTE: Color = SUBTEXT0;

/// Link text — interactive, but not "focused", so not mauve
pub const LINK: Color = BLUE;

/// Link URL shown after link text
pub const LINK_URL: Color = OVERLAY0;

/// Table chrome (│ ├ ┼ ┤)
pub const TABLE_CHROME: Color = SURFACE1;

// ── Composite styles (color + modifier combos used frequently) ───────────

pub const fn bold(color: Color) -> Style {
    Style::new().fg(color).add_modifier(Modifier::BOLD)
}

pub const fn italic(color: Color) -> Style {
    Style::new().fg(color).add_modifier(Modifier::ITALIC)
}
