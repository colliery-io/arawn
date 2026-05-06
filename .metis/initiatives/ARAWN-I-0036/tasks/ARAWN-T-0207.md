---
id: tui-quality-bugs-utf-8-panic
level: task
title: "TUI quality bugs — UTF-8 panic, cancel doesn't cancel, branch loses tool_results, modal direct-select, chrome drift"
short_code: "ARAWN-T-0207"
created_at: 2026-05-06T10:44:35.601289+00:00
updated_at: 2026-05-06T11:05:33.367717+00:00
parent: ARAWN-I-0036
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0036
---

# TUI quality bugs — UTF-8 panic, cancel doesn't cancel, branch loses tool_results, modal direct-select, chrome drift

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0036]]

## Objective

Bundle of seven concrete bugs surfaced by the parallel design review (visual + interaction agents). All real quality regressions, not design tastes — each has a specific file:line and a clear "what's wrong" + "what should happen". Bundled into one task because they're all small (S effort each) and live in the same crate. Total estimated: ~1 hour of focused work.

Filed under I-0036 (Visual coherence pass) because four of the seven are visual/chrome bugs and the rest are quality cleanups in the same `arawn-tui` crate. Doesn't block I-0036's design phases — these can land independently.

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

## Type / Priority

- Bug bundle (7 items)
- P1 — UTF-8 panic is a hard crash; cancel-doesn't-cancel and branch-loses-tool-results are user-visible quality issues; the rest are visible-but-not-broken.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Bug 1 — UTF-8 panic in `truncate_for_display` (P0 within the bundle)
- **File:** `crates/arawn-tui/src/render.rs:745-751`
- **What:** `&s[..max]` slices a String by **bytes**, not chars. If a tool input contains a multi-byte char (emoji, accent) crossing byte `max`, this panics with `byte index N is not a char boundary`.
- **Fix:** use `s.chars().take(max).collect::<String>()` (matching the pattern used by `truncate_to` in the same file).
- **Test:** add a snapshot or unit test passing a string with `🔥` at byte 60 with `max = 60`.

### Bug 2 — List bullet color leaks into item text
- **File:** `crates/arawn-tui/src/markdown.rs` around line 289 (the LIST_BULLET span).
- **What:** Snapshot `styled_snapshot_rich_markdown` shows entire list items rendered in `DarkGray` (bullet color) instead of just the bullet glyph. Either the bullet style is being applied to the text spans, or the renderer's `current_style` accumulator carries the bullet style forward.
- **Fix:** Trace through `current_style` handling around list items. Bullet glyph gets `LIST_BULLET` style; subsequent text spans must use the default content style. Likely a missing style-pop after the bullet emit.
- **Test:** assert the styled snapshot for a list item has the text portion in default fg, only bullet in DarkGray.

### Bug 3 — Tool-call header dashes drift on unicode
- **File:** `crates/arawn-tui/src/render.rs:387` (top border) and `:419` (bottom border).
- **What:** Top uses `chat_width.saturating_sub(header.len() + 3)` where `header.len()` is **byte length**, not display columns. Bottom uses `chat_width.saturating_sub(4).min(80)` — completely decoupled. Result: top and bottom of the same box are different widths whenever tool names contain unicode or `chat_width > 84`.
- **Fix:** Compute box width once with `unicode_width::UnicodeWidthStr::width(...)`, share the value between top and bottom border rendering. (`unicode-width` crate is added in I-0036 Phase 4 — pull it in early for this bug.)
- **Test:** snapshot showing aligned top + bottom for a tool with an emoji or CJK char in the name.

### Bug 4 — `Cancel` action flips `is_generating=false` locally but doesn't actually cancel
- **Files:** `crates/arawn-tui/src/app.rs:540` (Cancel handler) and `crates/arawn-tui/src/event_loop.rs` (event-loop cancel path).
- **What:** When the user hits Esc during generation, the spinner disappears and `streaming_text` gets flushed with `(cancelled)` appended — but no cancel RPC is sent to the server. The model keeps generating; tokens land in `streaming_text` (now invisible because `is_generating=false`); when `Complete` arrives, a duplicate response is pushed.
- **Fix:** `Cancel` should:
  1. Send a `cancel_generation` RPC (already exists per `arawn-service::Service::cancel`).
  2. Mark the current session/turn as cancelled — ignore subsequent `StreamingText` / `ToolCall` / `Complete` events for that turn until the next user `Submit`.
  3. Then flip local `is_generating=false`.
- **Test:** integration test where a fake LLM streams 3 tokens, user cancels after 2, fake LLM emits the 3rd + Complete — assert the cancelled message stays cancelled (no duplicate Assistant message appears).

### Bug 5 — Branch flow drops `tool_result` messages on refresh
- **File:** `crates/arawn-tui/src/event_loop.rs:240-244` (history-branch modal close path).
- **What:** After truncating the session, the local `app.messages` is rebuilt from the truncated `SessionDetail`. The `match role` block has `_ => continue`, which silently skips `tool_result` and `summary` rows. The user's chat now shows only User+Assistant messages but the server kept the tool history — visual divergence from server state.
- **Fix:** Use `App::load_session_messages` (already exists) which handles tool_use/tool_result/summary correctly. Or extend the inline mapping to cover those roles.
- **Test:** branch a session that contains a tool call, assert the refreshed `app.messages` includes the `ToolCall` + `ToolResult` entries, not just User/Assistant.

### Bug 6 — Modal number-key direct-select is a TODO, currently misbehaves
- **File:** `crates/arawn-tui/src/event.rs:94-98`.
- **What:** Number keys (1-9) in modal mode currently emit `ModalConfirm` — confirming whatever option is currently focused, regardless of which number was pressed. A user pressing `2` to pick option 2 may instead confirm option 1 if it's focused.
- **Fix:** Add `Action::ModalSelectIndex(usize)` variant. In `event.rs`, map `KeyCode::Char('1'..='9')` to `ModalSelectIndex(n - 1)`. In `app.rs`, the action sets `modal.focused_index = idx` and then triggers `confirm()`.
- **Test:** unit test: set up a modal with 3 options, dispatch `ModalSelectIndex(2)`, assert option 2 was confirmed (oneshot received `Some(2)`).

### Bug 7 — `STATUS_BAR_BG` constant lies
- **File:** `crates/arawn-tui/src/theme.rs` (constant) vs `crates/arawn-tui/src/render.rs:107-192` (call site).
- **What:** `theme::STATUS_BAR_BG = Color::DarkGray` but `render.rs` actually uses `Color::Rgb(30, 30, 40)` directly. Snapshots confirm the `Rgb(30,30,40)` (a near-black blue), not DarkGray. Pick one — the Rgb is the better choice (richer, distinct from sidebar tab `Rgb(25,25,30)`).
- **Fix:** Update the constant to `Color::Rgb(30, 30, 40)` AND wire the call site to read from the constant. (This is the I-0036 Phase 1 wiring pattern — early-land an example.)
- **Test:** snapshot stays the same; verify constant matches rendered value.

## Implementation Notes

- All seven are S effort. Bundle is ~1 hour of focused work.
- Order to do them in (dependency-light first):
  1. Bug 7 (constant) — trivially mechanical, sets the wiring example.
  2. Bug 1 (UTF-8 panic) — pure swap, add 1 test.
  3. Bug 6 (modal direct-select) — small action+handler addition.
  4. Bug 5 (branch tool_result drop) — refactor to use existing helper.
  5. Bug 2 (list bullet style leak) — trace through `current_style` handling, may take longer to find than fix.
  6. Bug 3 (chrome drift) — pulls `unicode-width` dep, mechanical width replacement.
  7. Bug 4 (cancel doesn't cancel) — touches both client and server-call surface; careful but small.
- Each bug should be its own commit so the bisect-friendliness of the history is preserved. Test added per bug where feasible.

## Out of Scope (defer)

- The deeper interaction-flow issues from the design review (single-Esc invisibility, history modal fragility, `/connect` race) — those are design decisions, not bugs. They go in I-0035 / I-0036 task decomposition.
- Visual coherence work (palette wiring, tool-call collapsed rendering, etc.) — that's I-0036 proper.

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

## Status Updates

### 2026-05-06 — Six of seven bugs landed; bug 2 deferred for investigation

Six bugs fixed and committed independently for bisect-friendliness:

| # | Bug | Commit |
|---|---|---|
| 7 | `STATUS_BAR_BG` constant lies — wired to actual `Rgb(30,30,40)` | `7b12143` |
| 1 | UTF-8 panic in `truncate_for_display` (byte-slice → char-slice) | `68215cb` |
| 6 | Modal number-key direct-select (new `Action::ModalSelectIndex`) | `cb11770` |
| 5 | Branch flow drops `tool_result` (use `App::load_session_messages`) | `a8468d6` |
| 3 | Tool-call box top/bottom width drift (`unicode-width`, shared `box_width`) | `4e40cd4` |
| 4 | Cancel actually cancels (RPC + drop stale stream events for cancelled session) | `f1d00e0` |

**Test count:** 129 → 135 (six new tests added, one per bug where feasible).
**Clippy:** 0 warnings.
**Snapshots:** two re-baselined for bug 3's box-width change (`snapshot_chat_with_conversation`, `styled_snapshot_conversation`).

### Bug 2 deferred

The list-bullet-color-leak issue was deliberately skipped after a confidence check. The reviewer's evidence is the styled snapshot showing list item text in `DarkGray`, but I couldn't confirm whether that's a real code bug (style accumulator carrying the bullet style forward) or a snapshot-rendering artifact (terminal default fg serialized as `DarkGray` by the snapshot framework). Worth investigating in its own task; bundling a maybe-bug with verified bugs would have polluted the commit history.

Filed for follow-up: trace `markdown.rs:289` and the surrounding `current_style` handling, write a test that exercises a list item with explicit non-`DarkGray` text content, and verify whether the bug is real before committing a fix.

### Status

- ✅ 6 bugs landed
- ⏸ 1 bug deferred (#2 — list bullet color)
- All commits independently bisectable
- Ready to mark completed; the deferred bug should be filed as its own follow-up task under I-0036 when picked up.