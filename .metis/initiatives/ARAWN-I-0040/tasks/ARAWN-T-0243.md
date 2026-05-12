---
id: slack-projections-slack-messages
level: task
title: "Slack projections — slack_messages + slack_thread_messages"
short_code: "ARAWN-T-0243"
created_at: 2026-05-12T03:28:16.488434+00:00
updated_at: 2026-05-12T03:28:16.488434+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0242]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Slack projections — slack_messages + slack_thread_messages

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Implement Slack projections — `slack_messages` and `slack_thread_messages` — on top of the projection plumbing landed in T-0242. Each Slack channel message and thread reply becomes a projection row, embedded + FTS-indexed.

## Scope

- `slack_messages` table: id, feed_id, source_id, source_ts, channel_id, channel_name, sender_id, sender_name, body_text, thread_ts (nullable; non-null means this is a thread parent), reactions (JSON), permalink, created_at/updated_at, UNIQUE(feed_id, source_id).
- `slack_thread_messages` table: same shape plus parent_thread_ts. Distinct from `slack_messages` so callers can filter "top-level only" vs "thread replies" cheaply.
- FTS5 over `sender_name + body_text` for each.
- Embedding over `body_text`.
- Mirror-to-projection mapping in `arawn-feeds::templates::slack::*` projection adapters.
- Backfill on first projection-db open.
- Unit tests + small fixture-feed integration test.

## Acceptance Criteria

- [ ] Both Slack tables exist with FTS + embedding, populated on `slack-messages` / `slack-thread-messages` feed runs.
- [ ] Idempotent on re-run (UNIQUE on `(feed_id, source_id)`).
- [ ] Backfill walks the existing mirror.
- [ ] Tests verify projection row + FTS searchability after feed run.
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

- Follow T-0242's `Projection` enum pattern; add two variants.
- Thread replies are NOT promoted to top-level rows — they live exclusively in `slack_thread_messages`. Top-level thread parents appear in both tables (canonical in `slack_messages`, with reply count denormalized).
- Reactions emoji can be stored as JSON; not embedded.

### Dependencies

- T-0242 (projection plumbing).



## Status Updates **[REQUIRED]**

*To be added during implementation*