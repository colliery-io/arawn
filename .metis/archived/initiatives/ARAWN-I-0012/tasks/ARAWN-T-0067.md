i---
id: markdown-rendering-in-assistant
level: task
title: "Markdown rendering in assistant chat messages — headings, code blocks, lists, emphasis"
short_code: "ARAWN-T-0067"
created_at: 2026-04-03T02:38:27.573191+00:00
updated_at: 2026-04-03T10:44:28.477239+00:00
parent: ARAWN-I-0012
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0012
---

# Markdown rendering in assistant chat messages — headings, code blocks, lists, emphasis

## Parent Initiative

[[ARAWN-I-0012]]

## Objective

Render assistant chat messages as styled markdown instead of plain text. Users should see properly formatted headings, code blocks with syntax highlighting labels, lists, bold/italic, and links.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Headings (h1-h4) render bold with color differentiation by level
- [ ] Fenced code blocks render with dimmed background/indent and language label
- [ ] Inline code renders with distinct color (backtick-wrapped)
- [ ] Bold and italic render with actual terminal bold/italic styles
- [ ] Bullet and numbered lists render with proper indentation
- [ ] Links render underlined with URL visible
- [ ] Plain text paragraphs wrap correctly at terminal width
- [ ] Long code blocks don't break chat scrolling

## Implementation Notes

### Technical Approach
- Try `termimad` first for high-level markdown-to-terminal rendering
- Fall back to `pulldown-cmark` + manual ANSI styling if termimad is too limiting for our ratatui widget integration
- Markdown rendering happens in the chat message widget — each assistant `ContentBlock::Text` gets parsed and rendered as styled spans
- Need to integrate with ratatui's `Text`/`Line`/`Span` model — termimad may need an adapter layer

### Dependencies
- Depends on T-0066 (unified layout) being complete — rendering into the new chat panel

## Status Updates

- Decision: used `pulldown-cmark` (not termimad) — clean integration with ratatui Text/Line/Span model
- Created `crates/arawn-tui/src/markdown.rs` with `markdown_to_lines()` function
- Headings: bold + color by level (h1=Cyan, h2=Blue, h3=Magenta, h4=White)
- Code blocks: indented with dimmed bg (RGB 30,30,40), language label in italic
- Inline code: backtick-wrapped, amber text (RGB 220,170,110) on dark bg
- Bold/italic/strikethrough: actual terminal modifiers via style stack
- Lists: bullet (•) for unordered, numbered for ordered, nested indentation
- Links: cyan underlined text
- Block quotes: dimmed text
- Horizontal rules: ─── line
- Style stack for nested formatting (bold inside lists, etc.)
- Integrated into render_chat: Assistant messages get prefix on own line + markdown content
- 10 markdown tests + 66 total TUI tests all passing
- Added pulldown-cmark 0.12 to arawn-tui Cargo.toml