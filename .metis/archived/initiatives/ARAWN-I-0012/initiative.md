---
id: tui-polish-unified-layout-markdown
level: initiative
title: "TUI polish — unified layout, markdown rendering, tool display, visual refinement"
short_code: "ARAWN-I-0012"
created_at: 2026-04-03T02:24:16.731821+00:00
updated_at: 2026-04-03T18:04:27.893180+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: tui-polish-unified-layout-markdown
---

# TUI polish — unified layout, markdown rendering, tool display, visual refinement Initiative

## Context

The arawn TUI works but feels rough. The biggest UX friction is the three-pane layout with tab-to-switch focus — chat and input should be a single unified flow. Tool calls render as plain text prefixes. No markdown rendering. The sidebar dominates when visible. Status bar is minimal.

This initiative is about making the TUI feel good to use, not about cloning Claude Code's ink/React approach. We're a ratatui app with different strengths.

## Goals & Non-Goals

**Goals:**
- Unified chat+input pane (no focus switching between chat and input)
- Markdown rendering in assistant responses (headings, code blocks, lists, bold/italic)
- Better tool call display (status indicators, compact inline format, collapsible results)
- Cleaner status bar (model, tokens, session info, generating indicator)
- Progress indicators (spinners for tool execution, streaming indicator)
- Permission prompt rendering (modal dialog when permission system lands)

**Non-Goals:**
- Theme system / multiple color schemes (keep it simple for now)
- Vim mode (separate task T-0065)
- Voice input
- Customizable keybindings
- Rewriting to ink/React — we stay on ratatui

## Architecture

All work stays within the existing ratatui architecture. Key modules:
- `crates/arawn-tui/src/app.rs` — main App struct, focus/panel management
- `crates/arawn-tui/src/ui/` — rendering functions for each panel
- `crates/arawn-tui/src/widgets/` — custom widget implementations

The unified layout change (Task 1) is the foundation — it restructures the panel system that everything else renders into. Markdown rendering and tool display are independent of each other but both render within the chat panel. Status bar and progress indicators are also independent.

## Detailed Design

### Task 1: Unified chat+input layout
Remove the separate Input panel and focus-switching. Chat area fills the screen with input pinned at the bottom (2-3 rows). User is always "in" the input — typing goes to input, scroll keys scroll chat. No Tab cycling between chat and input. Sidebar becomes a toggleable overlay or drawer triggered by a keybind, not a permanent panel.

**Current:** 3 panels (sidebar, chat, input) with Tab to switch focus
**Target:** Single pane — chat scrolls above, input pinned below, sidebar is a drawer

### Task 2: Markdown rendering
Render assistant responses as styled markdown using `termimad` or `pulldown-cmark` + manual ANSI styling:
- **Headings**: bold, color-differentiated by level
- **Code blocks**: dimmed background or indented, with language label
- **Inline code**: backtick-wrapped with distinct color
- **Bold/italic**: actual bold/italic terminal styles
- **Lists**: bullet/numbered with proper indentation
- **Links**: underlined with URL visible

### Task 3: Tool call display
Replace `[tool: name]` prefix with compact inline indicators:
- `⏺ shell` (blinking/yellow while running, green on success, red on error)
- Tool results: truncated by default, expandable on scroll-to
- Distinguish tool call (what was requested) from tool result (what came back)
- Errors: red indicator + error text

### Task 4: Status bar redesign
Move status bar to bottom (above input). Show:
- Model name
- Token count (input + output)
- Session info (workstream / session short-id)
- Generating spinner (animated when active)
- Cost if available

### Task 5: Progress indicators
- Animated spinner during tool execution (ratatui has `Spinner` or we use frame-based unicode)
- Streaming indicator: pulsing cursor or `...` animation while waiting for first token
- Tool execution: show tool name + elapsed time while running

### Task 6: Permission prompt modal (blocked on I-0009)
When permission system lands, render permission requests as centered modal:
- Tool name + input preview
- Options: Allow Once / Allow Always / Deny
- Arrow key selection, Enter to confirm
- Amber/warning border

### Decomposition
1. Unified layout (remove focus switching, pin input, sidebar drawer)
2. Markdown rendering in chat
3. Tool call status indicators
4. Status bar redesign
5. Progress spinners and streaming indicators
6. Permission prompt modal (after I-0009)

## Testing Strategy

Primarily manual/visual testing — TUI rendering is hard to unit test meaningfully. Each task should be verified by running the TUI and exercising the feature. Snapshot tests for widget rendering where practical.

## Alternatives Considered

- **Rewrite to ink/React like Claude Code**: Rejected — ratatui is working well, gives us native performance, and avoids a Node.js dependency. The polish gap is about missing features, not the wrong framework.
- **Full theme system first**: Rejected — premature. Get the layout and rendering right with a single clean theme, then consider customization later.
- **pulldown-cmark vs termimad for markdown**: Both viable. pulldown-cmark gives more control but more work. termimad is higher-level. Decision deferred to the markdown task — try termimad first, fall back to pulldown-cmark if it's too limiting.

## Implementation Plan

**Phase 1 (foundation):** Unified layout (T-0066) — already active. Everything else builds on this.
**Phase 2 (rendering):** Markdown rendering + tool call display — independent, can be parallel.
**Phase 3 (chrome):** Status bar + progress indicators — lighter tasks, polish layer.
**Phase 4 (blocked):** Permission prompt modal — depends on I-0009 landing first.

Dependency graph:
```
T-0066 (layout) ──┬──> markdown rendering
                   ├──> tool call display
                   ├──> status bar
                   └──> progress indicators
I-0009 ──────────────> permission modal
```