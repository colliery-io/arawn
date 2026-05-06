---
id: phase-5-empty-loading-states-idle
level: task
title: "Phase 5 — Empty + loading states (idle hero, OAuth heartbeat, thinner cursor)"
short_code: "ARAWN-T-0212"
created_at: 2026-05-06T11:22:08.494371+00:00
updated_at: 2026-05-06T11:22:08.494371+00:00
parent: ARAWN-I-0036
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0036
---

# Phase 5 — Empty + loading states (idle hero, OAuth heartbeat, thinner cursor)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0036]]

## Objective

Treat empty / loading / mid-flow states as **designed moments** rather than accidental gaps. Today: first-launch is a blank chat with `Type your message...`; OAuth flow vanishes into a 5-minute black hole; the streaming cursor `█` is heavy and dominant.

Adds:
1. Idle hero — subtle `arawn` wordmark + key-binding hints when chat is empty.
2. OAuth heartbeat — single dim line above status bar pulsing every 30s while a `/connect <svc>` flow is in flight, with elapsed time. Disappears on success/timeout.
3. Thinner streaming cursor — `▌` (or `▍`) instead of full block `█`, in `accent` (mauve).

This is I-0036 Phase 5 — visual polish on the moments-when-arawn-is-not-actively-displaying-content. Smallest of the five phases (~80 LOC).

## Type / Priority

- Feature + visual polish
- P2 — affects first impressions strongly but doesn't block anything else.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

### Idle hero (empty chat)

- [ ] When `app.messages.is_empty() && !app.is_generating && app.streaming_text.is_empty()`, render a centered subtle hero block in the chat area:
  ```
                    ╭─────────────╮
                    │    arawn    │
                    ╰─────────────╯
                    
        Type / for commands · Tab to toggle sidebar
                /connect <service> · ↑ recall
  ```
- [ ] Hero rendered in `subtext0` (dim text); border in `surface1`. No accent.
- [ ] Hero disappears as soon as the first message arrives (user submit OR welcome-on-empty system message from I-0035 Phase 1, whichever lands first).
- [ ] **Note:** I-0035 Phase 1 will add a welcome system message that pushes a single message into `app.messages` on first launch. Coordinate so the welcome message replaces the hero (not stacked above it). I-0035 Phase 1 lands AFTER this task per sequencing — so this task ships the hero, and I-0035 Phase 1 swaps it for a welcome system message.

### OAuth heartbeat

- [ ] `App` gains state `oauth_in_flight: Option<{ service: String, started: Instant }>`. Set when `/connect <svc>` dispatches its `start_oauth_flow` RPC; cleared on the resulting `[integration] connected` system message OR after 5-minute timeout.
- [ ] While `oauth_in_flight.is_some()`, render a single dim line above the status bar:
  ```
   • waiting for {service} OAuth in browser… {elapsed}s · Esc to cancel
  ```
  Pulse the bullet (Yellow ↔ DarkGray) every second so the user knows the app isn't frozen.
- [ ] **Esc to cancel** while OAuth is in-flight: clear `oauth_in_flight` locally; the callback server will time out on its own (5 min). Future improvement: a server RPC to drop the listener immediately, but that's a separate task — see I-0033 followups.

### Thinner streaming cursor

- [ ] In `render.rs:496-502` (the streaming-text render block), replace `Span::styled("█", Style::default().fg(Color::Blue))` with `Span::styled("▌", Style::default().fg(theme::accent))`.
- [ ] Snapshot rebaseline.

## Implementation Notes

- **Hero placement:** use `Layout` to vertically center within the chat area. ratatui's `Layout::Direction::Vertical` with `Constraint::Length` rows surrounding a `Constraint::Min(0)` works. Or compute `(area.height - hero_height) / 2` and offset.
- **Hero is decorative — don't tab-stop into it.** Focus model unchanged; Tab still cycles chat ↔ sidebar.
- **OAuth heartbeat layout:** add a 1-row `Constraint::Length(1)` above the status bar in the main layout when `oauth_in_flight.is_some()`. Otherwise zero rows (no layout shift in normal use).
- **Pulse animation:** use the existing `app.spinner_frame` modulo for color flip — no new state needed.
- **Hero is opt-out:** if a user prefers the blank chat, they can dismiss with Esc (treats hero as a focusable card; Esc clears it for the session). Or just leave it always-on and accept; user feedback dictates which.

## Out of Scope (defer)

- Welcome system message on empty state — that's I-0035 Phase 1 (workstream-conditioned identity correction).
- ASCII art / branding flourish — keep the hero minimal; we're not stylizing arawn as a brand here.
- OAuth flow cancellation server-side — covered by I-0033's followup item about the callback-listener leak (5-minute hangover after failed connect).
- Mid-generation idle states (e.g. "the agent is on a long tool call") — that surface is OK today; defer.

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

*To be added during implementation*