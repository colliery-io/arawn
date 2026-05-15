---
---
id: retro-tui-client-ws-rpc-fetch
level: task
title: "/retro TUI client — WS-RPC fetch, render, diary editor"
short_code: "ARAWN-T-0290"
created_at: 2026-05-15T23:45:55.027050+00:00
updated_at: 2026-05-15T23:45:55.027050+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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
