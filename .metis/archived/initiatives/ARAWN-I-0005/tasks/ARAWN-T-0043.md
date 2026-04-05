---
id: render-pipeline-layout-status-bar
level: task
title: "Render pipeline — layout, status bar, chat area, input bar"
short_code: "ARAWN-T-0043"
created_at: 2026-04-01T11:46:41.674685+00:00
updated_at: 2026-04-01T12:26:04.553221+00:00
parent: ARAWN-I-0005
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0005
---

# Render pipeline — layout, status bar, chat area, input bar

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0005]]

## Objective

Implement the three-panel layout with real ratatui widgets: status bar (top), sidebar + chat (middle, horizontal split), input bar (bottom). Each panel renders from App state. All rendering is headless-testable via TestBackend.

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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Layout: `Vertical [StatusBar(1), Middle(flex), InputBar(3)]` with Middle = `Horizontal [Sidebar(20%), Chat(80%)]`
- [ ] `widgets/status.rs`: renders "Arawn | Workstream: {name} | Session: {id_short}" from App state
- [ ] `widgets/chat.rs`: renders messages as scrollable list. User messages prefixed "You:", assistant prefixed "Arawn:", tool calls as `[tool: name] result`. Streaming text shown with cursor indicator.
- [ ] `widgets/input.rs`: renders input buffer with cursor position. Border changes color based on focus. Placeholder "Type your message..." when empty. Grayed out when `is_generating`.
- [ ] `widgets/sidebar.rs`: placeholder for now — renders "Workstreams" and "Sessions" headers with lists from App state. Selected item highlighted. Active focus border.
- [ ] Focus-dependent border highlighting: active panel gets colored border, others get dim border
- [ ] Headless test: render with 80x24 TestBackend → status bar text present on row 0
- [ ] Headless test: render with messages → chat area contains message text
- [ ] Headless test: render with input text → input bar contains the text
- [ ] Headless test: layout adapts to different terminal sizes (40x12 minimum)

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

## Implementation Notes

- `render.rs` becomes the real layout function, `widgets/` directory for each panel
- Use `ratatui::layout::Layout` with `Direction::Vertical` and `Direction::Horizontal`
- Chat: `List` of `ListItem`s with styled spans for role prefixes
- Input: `Paragraph` with cursor via `frame.set_cursor_position()`
- Status bar: `Paragraph` with centered spans
- Sidebar: `List` with `ListState` for selection tracking
- All widgets take `&App` — no side effects
- Depends on: T-0042 (App state, Action, render skeleton)

## Status Updates
- **2026-04-01**: Complete. All widgets implemented as functions in render.rs (kept flat instead of separate files — simpler for now): render_status_bar (workstream name + session ID + generating indicator), render_sidebar (workstream list with selection + session list with dates), render_chat (messages with styled role prefixes, tool calls, streaming text with █ cursor, word wrap, scroll), render_input (placeholder text, generating state, cursor positioning). 6 headless tests: status bar content, messages render, input text, streaming cursor, small/large terminal sizes. 16 TUI tests total, clippy clean.