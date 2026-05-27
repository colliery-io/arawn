---
id: t-i-feed-run-feedback-in-tui-and
level: task
title: "T-I: Feed-run feedback in TUI and refresh stale vision doc"
short_code: "ARAWN-T-0444"
created_at: 2026-05-27T20:18:49.413618+00:00
updated_at: 2026-05-27T20:18:49.413618+00:00
parent: ARAWN-I-0061
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0061
---

# T-I: Feed-run feedback in TUI and refresh stale vision doc

## Parent Initiative
[[ARAWN-I-0061]]

## Objective
Two loose ends from the review: (1) after `/feeds run <id>` the "pulled N rows /
M entities" signal lives only in the server log, invisible in the client; (2)
`.metis/vision.md` still uses retired "workstream/watcher" vocabulary.

## Scope
- **Feed-run feedback**: surface a result toast/message in the TUI after
  `/feeds run` (rows processed / entities extracted). Source the counts from the
  run path (`extraction.run feed=… rows=N`) through to a client toast
  (`crates/arawn-tui/src/event_loop/` feeds handling + `toast`).
- **Vision refresh**: update `.metis/vision.md` workstream→lens, watcher→feed, and
  align the memory/signal/lens framing with [[ARAWN-I-0061]]. Use `metis sync`
  after editing (don't hand-edit around the MCP).

## Acceptance Criteria
- [ ] After `/feeds run`, the TUI shows a result (rows/entities), not just a log line.
- [ ] `.metis/vision.md` uses lens/feed vocabulary and the corrected model.
- [ ] `angreal check workspace` green; `metis sync` clean.

## Dependencies
Independent. May be split into two if convenient.

## Status Updates
*To be added during implementation*
