---
id: backfill-loop-walk-existing
level: task
title: "Backfill loop — walk existing projection rows on new binding"
short_code: "ARAWN-T-0253"
created_at: 2026-05-13T01:28:14.585042+00:00
updated_at: 2026-05-13T01:28:14.585042+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0251]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Backfill loop — walk existing projection rows on new binding

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

When a workstream binds a new feed via `/workstream bind <name> <feed_id>`, the existing projection rows for that feed need to be walked through the extractor so they land in the workstream's KB. Without this, only items arriving *after* the bind get extracted — the workstream looks empty until the next feed cycle even though the projection rows are already on disk.

Implements a spawn-loop backfill that walks `WHERE source_ts > cursor` in batches, calling the same `ExtractorRunner::run_for_workstream` the steady-state dispatch hook uses. Re-uses the cursor table from T-0251 so a partial backfill resumes cleanly.

## Scope

### Trigger

`/workstream bind <name> <feed_id>` calls a new `ExtractorRunner::trigger_backfill(workstream, feed_id)` after persisting the binding. The trigger:

1. Resolves `feed_id` → `feed_type` via the feed registry (gmail-inbox-archive → gmail_messages, slack-channel-* → slack_messages/slack_thread_messages, etc.).
2. Spawns a tokio task that runs the backfill loop and logs progress.
3. Returns immediately — the slash command doesn't block.

The runner records `workstream_extractor_runs` state (or extends `feeds::meta`) to make backfill resumable after a crash.

### Loop

```text
loop {
    let stats = runner.run_for_workstream(workstream, feed_type).await?;
    if stats.processed == 0 { break; }   // caught up
    metrics.advance(stats);
}
```

Each iteration processes `BATCH_SIZE` rows (default 50), uses the cursor for paging. Same execution path as the dispatch-hook trigger from T-0251; backfill is just "run the same thing repeatedly until no new rows."

### Caps and resumption

- Wall-clock cap per backfill burst: 10 min default. After the cap, persist the cursor and let the next scheduled trigger pick up.
- Crash safety: cursor is the source of truth. A restart resumes from the persisted position automatically — no separate "backfill in progress" flag needed.

### What's deferred

- Backfill on workstream *creation* (not just bind) — for v1 a freshly-created workstream that's never bound has nothing to walk. The bind action is the trigger.
- Throttling per workstream — could be added later if any workstream's backfill starves others. v1: tokio's scheduler is fine for ~15 workstreams.

## Acceptance Criteria

- [ ] `ExtractorRunner::trigger_backfill(workstream, feed_id)` exists; called by `WorkstreamBindTool`.
- [ ] Spawn-loop processes existing projection rows in batches until caught up.
- [ ] Cursor advances monotonically; partial backfill resumes from where it left off after a restart.
- [ ] Wall-clock cap honored; next-scheduled trigger picks up the rest.
- [ ] Unit tests cover: bind triggers backfill, cursor resumption, wall-clock cap.
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

### Tokio task lifecycle

The spawn-loop runs as a `tokio::spawn`'d task with no explicit join. If the process exits mid-backfill, the cursor in sqlite remembers the position. No bespoke "in flight" tracking needed because the cursor is durable.

### Concurrency

Multiple binds for the same workstream in quick succession should not race. Cheapest defense: a `Mutex<HashSet<(workstream, feed_type)>>` on the runner that gates parallel backfills. Second bind sees the in-flight task and is a no-op.

### Dependencies

- T-0251 (ExtractorRunner, cursor store).
- `arawn-engine::tools::workstream::WorkstreamBindTool` (trigger site).
- `arawn-feeds::registry` (feed_id → feed_type resolution).

### Risk considerations

- **Stale cursor after manual edit.** If somebody hand-edits the cursor table or restores from backup, the backfill could re-process rows that are already in the workstream. `store_fact`'s dedup makes this safe in terms of correctness; just wastes compute. Acceptable.
- **Long backfill on first binding.** If the user binds a gmail feed with 50k existing messages, that's potentially hours of LLM calls. Surface progress via tracing logs; revisit if it bites.

## Status Updates

*To be added during implementation*