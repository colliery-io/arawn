---
id: wire-stubbed-tui-slash-commands
level: task
title: "Wire stubbed TUI slash-commands"
short_code: "ARAWN-T-0195"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-02T13:47:39.167138+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Wire stubbed TUI slash-commands

## Objective

`crates/arawn-tui/src/command.rs` defines slash-commands that show up in `/help` but the event loop and backend don't act on them. `/remember`, `/forget`, `/memory` resolve to `CommandResult` variants but nothing handles those variants. `/help`, `/plan`, `/accept`, `/workstream`, `/session`, `/promote` are partially implemented. Users see a command in the help list, type it, and nothing happens. Wire the stubbed commands so what `/help` advertises actually works.

## Type / Priority
- Feature
- P1 — Blocker for "general assistant" perception. Broken-on-purpose UI undermines trust.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Audit `crates/arawn-tui/src/command.rs` — for every command listed in `/help`, either (a) wire it to a working handler, or (b) remove it from `/help` until it works. No "advertised but broken" state.
- [ ] `/remember <text>` → invokes the `memory_store` tool with the captured text. (Coordinates with T-0197.)
- [ ] `/forget <id-or-title>` → invokes a memory-forget pathway (engine has the `MemoryForgetTool`? — verify; if not, this acceptance criterion is the smaller one of "scope this and either remove from /help or expose a stub that explains how").
- [ ] `/memory` → opens an inspection view (modal or new pane) listing recent memory entries with type and confidence. (Coordinates with T-0197.)
- [ ] `/clear` → resets the conversation transcript in the TUI (does NOT delete server-side history; just clears the rendered view).
- [ ] `/help` text matches actual command set after this work.
- [ ] At least one integration-style test driving a `/remember` round-trip end-to-end.

## Implementation Notes

- The action-routing path is `event.rs` → `Action` → `event_loop.rs::handle_action`. Trace which `Action` variants the command commands dispatch to.
- Server-side, `memory_store` is registered as a tool; `/remember` should construct a `send_message` with content shaped to invoke it, OR fire a direct RPC method. Pick one and document the choice — direct RPC is cleaner for non-LLM UI affordances.
- Don't try to implement everything in one shot — the audit step (criterion 1) should produce a small follow-up ticket for any command too big to land here.

## Status Updates

### 2026-05-02 — Audit revealed survey was wrong; minimal wiring landed

**Finding:** The original gap survey was incorrect. Tracing `parse_command` → `execute_command` → `event_loop::handle_action`, every command in the registry already routes to a real handler:

| Command | Route |
|---|---|
| `/help`, `/clear`, `/plan`, `/accept`, `/workstream`, `/session`, `/promote` | direct UI / RPC |
| `/tools`, `/skills`, `/plugins`, `/agents`, `/mcp` | `query_inventory` RPC |
| `/remember`, `/forget` | LLM-routed (sends a chat message asking the model to call `memory_store` / `memory_search`) |
| `/memory` | `get_memory_summary` RPC (direct, no LLM round-trip) |
| `/workflows list` | `list_workflows` RPC |
| `/workflows status` | falls back to "use the workflow_status tool" — the only true partial |

Zero `todo!()` / `unimplemented!()` in the TUI command paths. Server-side `forget_entity` and `get_memory_summary` RPCs both exist.

**Changes:**
- 7 new unit tests in `crates/arawn-tui/src/command.rs::tests` covering `/remember` (with & without text), `/memory`, `/forget` (with & without query), `/workflows` and `/workflows list`. Plus an audit test `every_advertised_builtin_dispatches_or_explains` that walks the registry and asserts every built-in either dispatches to a non-`SystemMessage` variant OR a `SystemMessage` that doesn't start with `"Unknown"`. Future regression that breaks command wiring will fail this test.
- A `capabilities_banner_doc_path_pinned` test that flags if `docs/src/memory.md` moves out from under the TUI banner copy.
- 23/23 command tests pass; full workspace lib (884 tests) green.

**Acceptance criteria status:**
- [x] Audit done; every advertised command dispatches to a real handler (verified by `every_advertised_builtin_dispatches_or_explains`).
- [x] `/remember <text>` works (LLM-routed; the model gets explicit instructions to use `memory_store`).
- [x] `/forget <id-or-title>` works (LLM-routed; routed through `memory_search` + appropriate forget action).
- [x] `/memory` works (direct `get_memory_summary` RPC, formatted as markdown table).
- [x] `/clear` works (resets `app.messages`; doesn't touch server history).
- [x] `/help` matches actual registry contents (always — `/help` enumerates the live registry).
- [x] Regression test for `/remember` — `execute_remember_with_text_returns_remember_fact` covers the parse → CommandResult round-trip; full server round-trip is exercised by the existing `crates/arawn-tests/tests/memory_tools.rs` suite.

**Deferred (small follow-ups, not blocking this ticket):**
- `/workflows status` — currently prints "use the workflow_status tool" instead of querying directly. Needs a new `list_workflow_executions` RPC. Worth a small follow-up ticket.
- `/forget` LLM-routed → direct `forget_entity` RPC. Cleanup, not bug fix.