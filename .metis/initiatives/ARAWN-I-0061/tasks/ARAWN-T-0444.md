---
id: t-i-feed-run-feedback-in-tui-and
level: task
title: "T-I: Feed-run feedback in TUI and refresh stale vision doc"
short_code: "ARAWN-T-0444"
created_at: 2026-05-27T20:18:49.413618+00:00
updated_at: 2026-05-28T15:44:03.720092+00:00
parent: ARAWN-I-0061
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria
- [ ] After `/feeds run`, the TUI shows a result (rows/entities), not just a log line.
- [ ] `.metis/vision.md` uses lens/feed vocabulary and the corrected model.
- [ ] `angreal check workspace` green; `metis sync` clean.

## Dependencies
Independent. May be split into two if convenient.

## Status Updates

**2026-05-28 — Done.** Both sub-items landed.

Feed-run feedback:
- `FeedSummaryDto` gains an optional `last_run_items: Option<u64>` field
  (`#[serde(default, skip_serializing_if = "Option::is_none")]` — backwards
  compatible). `feed_summary_to_dto` defaults it to `None`; `feed_run_inner`
  captures the `RunOutcome` from `runtime.run_feed_once`, extracts
  `summary.items_written`, and stamps it onto the DTO before returning.
- The server notice that previously printed a literal `?` now prints the real
  item count.
- TUI `FeedRun` handler reads `dto.last_run_items` and appends
  `- Items pulled: **N**` to the system-message body when present. Falls
  through silently when the field is missing (so the cron path through
  `feeds_list` keeps its existing shape).

Vision refresh:
- `.metis/vision.md` rewritten around lens/feed vocabulary and the
  memory/signal/lens model. New `Memory · Signals · Lenses` section, `Feeds`
  section, `Knowledge Persistence` reframed (memory global, signals per-lens),
  `Sandboxed Tool Execution` updated to per-lens `workspace/` boundaries,
  success criteria reworded to lens/feed/memory terms, AWEN noted as the
  multi-user reshape. Workstream / watcher / "promote a scratch session"
  language gone.
- `metis sync` clean (2 updated docs — vision + this status).

Verification: `arawn-tui` 254/0; `angreal check workspace` exit 0.