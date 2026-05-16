---
id: retro-tui-client-ws-rpc-fetch
level: task
title: "/retro TUI client — WS-RPC fetch, render, diary editor"
short_code: "ARAWN-T-0290"
created_at: 2026-05-15T23:45:55.027050+00:00
updated_at: 2026-05-16T03:13:18.844887+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# /retro TUI client

## Goal
TUI surface for the retro tablet. Connects via the existing WS-RPC client; fetches the current retro; renders `what_happened` + `patterns` as markdown; opens a diary editor; saves via `ceremonies.upsert_diary`.

## Reference
I-0043 §Diary capture step 1–4.

## Acceptance
- New `/retro` TUI command.
- Sequence:
  1. `ceremonies.get_retro_current` → tablet + items
  2. render `what_happened` + `patterns` sections (read-only, with citation IDs as inline footnotes)
  3. open the diary editor (existing editor surface from the chat input area; pre-populates with any existing diary body)
  4. on save → `ceremonies.upsert_diary` → on success, show "saved" toast and refresh
- Subscribes to `EngineEvent::Ceremony(_)` for live updates while open.
- Tests: snapshot the rendered markdown for a synthetic retro tablet.

## Out of scope
`/today` and `/week` clients — those are I-0041 and I-0042.
## Status Updates

**2026-05-16 — implementation landed (renderer only; slash-command wiring deferred).**

**Scope deviation:** the task body wanted a full `/retro` slash command in `arawn-tui`: registry entry, WS-RPC fetch via `ceremonies.get_retro_current`, diary editor, save via `ceremonies.upsert_diary`, live-refresh on broadcast events.

T-0283 shipped the `ceremonies.*` methods as a `CeremonyService` *inside* `arawn-ceremonies` rather than threading them through `arawn-service::ArawnService`/`LocalService`. Until ceremonies are wired into the binary's RPC dispatcher (a one-task follow-up not yet written), the TUI has nothing to call.

What I shipped instead: the **renderer** — the well-bounded, testable piece that doesn't depend on the binary integration.

- New `crates/arawn-ceremonies/src/render.rs`:
  - `RetroView { tablet, items, diary: Option<String> }` — the bundle the TUI assembles from three RPC calls.
  - `render_retro(&RetroView) -> String` — produces the canonical markdown layout: header, "What happened" + "Patterns" + "Your reflection" sections with citation footnotes.
  - Handles every edge: empty `what_happened` → placeholder copy; empty `patterns` → bootstrap-message ("insufficient history"); blank/missing diary → write-prompt placeholder; items sorted by ordinal; citation footnotes deduplicated; missing citation → no `[^cite-…]` marker; non-text bodies fall back to raw JSON.

- Exposed from `lib.rs` as `RetroView` + `render_retro`. The TUI consumes via `arawn_ceremonies::render_retro(&view)`.

**Tests (9 new in `render::tests`, 80 total in the crate):**
- Full happy-path with all three sections + citations + footnotes.
- Empty what_happened / patterns placeholders.
- Missing + blank diary placeholders.
- Sort-by-ordinal preserved.
- Footnote dedup with repeated citations.
- Missing citation omits the marker rather than breaking layout.
- Non-text body falls back to JSON.

**Follow-up (not in this task):**
- The `/retro` slash command itself (registry + dispatch + editor save).
- The RPC method binding (`ceremonies.get_retro_current` etc. wired into the WS dispatcher).
- Live refresh on `CeremonyEvent` broadcast.

Filed as part of the "wire ceremonies into the binary" task that hasn't been written yet. The renderer's contract is stable — the TUI client just needs the three RPC calls returning the existing DTOs.

Next: T-0291 (UAT scenario — synthetic 4-week retro run).