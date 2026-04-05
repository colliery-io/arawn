---
id: unified-chat-input-layout-remove
level: task
title: "Unified chat+input layout — remove focus switching, pin input at bottom"
short_code: "ARAWN-T-0066"
created_at: 2026-04-03T02:26:55.135493+00:00
updated_at: 2026-04-03T10:41:13.816946+00:00
parent: ARAWN-I-0012
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0012
---

# Unified chat+input layout — remove focus switching, pin input at bottom

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0012]]

## Objective

Merge chat and input into a single seamless pane. The user is always typing in the input — no Tab cycling between chat and input focus. Chat scrolls above, input pinned at bottom, no borders between them. **Sidebar is untouched** — keep existing behavior exactly as-is.

## Scope

ONLY the main chat+input area. Do not change:
- Sidebar (3-char strip, 20% expand, focus behavior) — keep as-is
- Tab cycling to/from sidebar — keep as-is
- Status bar position — keep as-is

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Chat and input are one continuous vertical area — no separate bordered panels
- [ ] Input is always active when in main view (not sidebar) — no Tab needed to start typing
- [ ] Focus cycle simplified: Tab goes Main ↔ Sidebar (two states, not three)
- [ ] In Main mode: typing → input, Up/Down scroll chat, Enter submits
- [ ] No visible border between chat area and input — seamless text flow with a thin separator
- [ ] Streaming text and input coexist visually in the same pane
- [ ] Snapshot tests updated for new layout

## Implementation Notes

### Layout changes (render.rs)
**Current:** Chat has a bordered `Block`, Input has a separate bordered `Block`, Tab switches focus between them.

**Target:** Single area — chat Paragraph fills space above, thin separator line (─), input area (2-3 rows) pinned at bottom. No Block borders on either. The "Chat" title and border disappear. Input loses its border — just a `> ` prompt prefix.

### App state changes (app.rs)
- `Focus` enum: remove `Chat` variant. Keep `Input` (renamed `Main`) and `Sidebar`.
- When focus is `Main`: all typing → input, Up/Down scroll chat, Enter submits.
- Remove all `self.focus == Focus::Chat` branches — chat scrolling works from Main focus.
- Remove all `self.focus == Focus::Input` checks on typing — always accept input in Main.

### Event changes (event.rs)
- Tab: Main → Sidebar → Main (two-state toggle)
- In Main: char → TypeChar, Up/Down → ScrollUp/ScrollDown, Enter → Submit
- In Sidebar: Up/Down → SidebarUp/SidebarDown, Enter → SidebarSelect (same as today)
- Remove the Chat focus key handling entirely

## Status Updates

- Removed `Focus::Chat`, renamed `Focus::Input` → `Focus::Main`
- Tab now two-state toggle: Main ↔ Sidebar
- Chat area: removed bordered Block, renders directly as Paragraph
- Input: borderless single line with `> ` prompt, horizontal scroll for long input
- Thin `───` separator between chat and input
- Layout: status(1) + chat(flex) + separator(1) + input(1) = more chat space
- Up/Down scroll chat from Main focus (no separate Chat focus needed)
- Updated event.rs: merged map_input_key/map_chat_key into map_main_key
- Updated event_loop.rs: all Focus::Input/Chat refs → Focus::Main
- All 56 TUI tests passing, 14 snapshots updated