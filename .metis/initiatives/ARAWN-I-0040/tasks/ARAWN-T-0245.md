---
id: atlassian-projections-jira-issues
level: task
title: "Atlassian projections — jira issues/comments/history + confluence pages"
short_code: "ARAWN-T-0245"
created_at: 2026-05-12T03:28:18.482723+00:00
updated_at: 2026-05-12T03:28:18.482723+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0242]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Atlassian projections — jira issues/comments/history + confluence pages

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Implement the Atlassian projection family on top of T-0242's plumbing: `jira_issues`, `jira_comments`, `jira_history`, `confluence_pages`. Issues update frequently — this is the first projection family that exercises the UPDATE path seriously.

## Scope

- `jira_issues` table: id, feed_id, source_id (issue key), source_ts (updated), project_key, summary, status, assignee, reporter, priority, labels (JSON), components (JSON), body_text (description), resolution_at, created_at/updated_at, UNIQUE(feed_id, source_id).
- `jira_comments` table: id, feed_id, source_id (comment id), source_ts, issue_key, author, body_text, created_at/updated_at, UNIQUE(feed_id, source_id).
- `jira_history` table: id, feed_id, source_id (changelog event id), source_ts, issue_key, field, from_value, to_value, author, created_at/updated_at, UNIQUE(feed_id, source_id). NOT embedded (low semantic value); FTS over `field + from_value + to_value` only.
- `confluence_pages` table: id, feed_id, source_id (page id), source_ts (last-updated), space_key, parent_id, title, body_text (extracted markdown), version, author, created_at/updated_at, UNIQUE(feed_id, source_id).
- FTS5 over the natural text fields for each.
- Embedding over `summary + body_text` for issues / `body_text` for comments + pages.
- UPSERT for issues + pages (mutable). Refresh embedding on body_text hash change.
- Mirror-to-projection adapters in `arawn-feeds::templates::atlassian::*`.
- Backfill walks each feed's mirror.

## Acceptance Criteria

- [ ] Four Atlassian projection tables exist with appropriate FTS + embedding (history is FTS-only).
- [ ] Idempotent on re-run; UPDATE refreshes embedding when text changes.
- [ ] Backfill walks the existing mirror for each Atlassian feed.
- [ ] Tests cover insert + update + history-event path.
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

- This is the first task to exercise update semantics seriously. T-0242's plumbing must support UPSERT cleanly; if it doesn't, fix in T-0242 first or surface a follow-up.
- Jira history rows are append-only (one per field-change event) — keep them as historical record even when the issue is later deleted; rely on `feed_id + source_id` UNIQUE.
- Confluence pages' `body_text` already comes through `htmd` conversion in the mirror.

### Dependencies

- T-0242 (projection plumbing, including UPSERT path).

## Status Updates

*To be added during implementation*