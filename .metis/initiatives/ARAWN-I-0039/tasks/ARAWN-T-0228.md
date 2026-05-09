---
id: slack-channel-archive-duplicates
level: task
title: "Slack channel-archive duplicates parent + boundary reply on every run"
short_code: "ARAWN-T-0228"
created_at: 2026-05-09T00:00:00+00:00
updated_at: 2026-05-09T00:00:00+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: ARAWN-I-0039
---

# Slack channel-archive duplicates parent + boundary reply on every run

## Parent Initiative

[[ARAWN-I-0039]]

## Severity

P1 — found during T-0218 live UAT. Compounded by T-0226's cron_recovery loop, the slack/channel-archive feed for `#domino-data-labs` accumulated **65 MB** of thread-file data containing **only 3 unique message timestamps repeated ~298 times each**. Even with cron_recovery disabled, every cron firing re-appends the parent + boundary reply to each thread file.

## Reproduction

1. Register `slack/channel-archive` against any channel that has a thread with replies.
2. Let it run twice.
3. `cat threads/<parent_ts>.jsonl | jq -r .ts | sort | uniq -c` — observe duplicate ts entries for the parent and the most recent reply.

## Root cause

Two bugs in `crates/arawn-feeds/src/templates/slack/common.rs::archive_channel_with_threads`:

### Bug 1 — pass-2 parent skip is first-call-only

```rust
// Old code:
if ts == parent_ts && prior.is_none() {
    continue;
}
```

The `prior.is_none()` guard means the parent is only skipped on the first thread fetch (when no cursor exists). On every subsequent run the cursor is `Some(...)`, so the parent (which `conversations.replies` always returns) is appended again. One duplicate parent line per cron firing per thread.

### Bug 2 — Slack's `oldest` parameter is *inclusive*

Both `conversations.history` and `conversations.replies` treat `oldest` as inclusive (despite some Slack docs being ambiguous). Our cursor stores `max(ts seen)`, then passes that as `oldest` on the next call. Slack returns the boundary message + everything newer. We had no in-template dedup against the prior cursor value, so the boundary message was re-appended.

This affected both top-level day files (boundary message duplicates) and thread reply files (boundary reply duplicates).

## Disk-side fallout

For the `domino-data-labs` UAT feed (compounded with the cron_recovery 37x amplifier from T-0226):

```
$ wc -l threads/1772030014.664699.jsonl
1191
$ jq -r .ts threads/1772030014.664699.jsonl | sort | uniq -c | sort -rn | head -3
 595 1772030014.664699
 298 1772030086.344859
 298 1772030068.684629

unique: 3 messages.   duplicated: 1188 lines.
```

Total thread-file size: 65MB for ~295 unique reply-thread messages. Should have been about 200KB.

## Fix landed

Two-line change in `archive_channel_with_threads`:

```rust
// Pass 1 — dedup boundary against history's oldest_ts:
if let Some(floor) = history_floor.as_deref()
    && ts <= floor {
        continue;
}

// Pass 2 — always skip parent (drop the prior.is_none() clause),
// AND dedup boundary against the per-thread cursor:
if ts == parent_ts {
    continue;
}
if let Some(floor) = prior.as_deref()
    && ts <= floor {
        continue;
}
```

## Acceptance Criteria

- [x] Pass-1 history loop skips messages whose `ts <= history_floor` (the prior `latest_ts` cursor).
- [x] Pass-2 thread-replies loop unconditionally skips the parent ts.
- [x] Pass-2 thread-replies loop skips messages whose `ts <= prior` (the per-thread cursor).
- [x] All 11 existing `slack_channel_archive` tests still pass — confirms the dedup logic doesn't break the seed-and-advance path.
- [ ] Add a regression test that runs the helper twice with a stable mock returning parent + replies (i.e. simulating Slack's inclusive `oldest`) and asserts no duplicate ts on disk after the second run. (Filed as a v2 test gap — landed the fix during UAT to stop the bleeding.)
- [x] On-disk dedup for the affected `domino-data-labs` feed dir during UAT — script de-duplicates thread JSONL files in place by ts.

## Status Updates

### 2026-05-09 — fixed during UAT

Two-line fix landed in `templates/slack/common.rs`. All slack channel-archive tests still pass. Existing on-disk data deduplicated via in-place rewrite (preserved chronological order, kept first occurrence of each ts).
