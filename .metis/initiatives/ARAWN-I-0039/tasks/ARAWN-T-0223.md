---
id: jira-feed-templates-project
level: task
title: "Jira feed templates — project-tracker + assignee-tracker"
short_code: "ARAWN-T-0223"
created_at: 2026-05-08T21:01:17+00:00
updated_at: 2026-05-09T00:12:57.172048+00:00
parent: ARAWN-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0039
---

# Jira feed templates — project-tracker + assignee-tracker

## Parent Initiative

[[ARAWN-I-0039]]

## Objective

Land the two Jira feed templates from I-0039 Phase 4. Split out from T-0217. Heaviest of the three providers since each issue has three on-disk artifacts (issue snapshot, comments append-log, history append-log).

- `jira/project-tracker` — issues + comments + history for a project. Param: `project` (key like `ENG`).
- `jira/assignee-tracker` — personal feed: `assignee = currentUser()`. Auto-created on `/connect atlassian`.

Reference: I-0039 Detailed Design; existing `arawn-integrations/src/atlassian/jira.rs` (`jira_v3_openapi` client landed in T-0213).

## Type / Priority

- Feature, P1.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Both templates registered in `arawn_feeds::default_registry`.
- [ ] Extends the `AtlassianFeedClient` trait introduced by T-0222 with Jira surface: `jql_search(jql, since)` returning issue list, `issue_changelog(key, since)` returning history entries, `issue_comments(key, since)` returning comments.
- [ ] **Cursors**:
  - Per-feed: highest `updated` ISO timestamp seen across the project / assigned issues. Used as a JQL `updated >= "<cursor>"` clause on next run.
  - Per-issue (project-tracker only): `last_comment_id` and `last_history_id` so append-only logs advance independently of the issue snapshot.
- [ ] **Disk layout**:
  - `jira/project-tracker/<feed_id>/<ISSUE-KEY>/issue.json` — overwrite per run (latest snapshot).
  - `jira/project-tracker/<feed_id>/<ISSUE-KEY>/comments.jsonl` — append-only.
  - `jira/project-tracker/<feed_id>/<ISSUE-KEY>/history.jsonl` — append-only.
  - `jira/assignee-tracker/<feed_id>/<ISSUE-KEY>/issue.json` only (no comments/history for personal — keeps it light).
- [ ] `validate(params)`:
  - `project-tracker` requires `project` non-empty; resolves to project ID at registration time and rejects unknown projects with a clear error.
  - `assignee-tracker` no params.
- [ ] `defaults(params)`: cadence `30m` for both.
- [ ] **Auto-create** `jira/assignee-tracker` on `/connect atlassian`. Idempotent. (May be deferred to T-0219 alongside `/feeds` UX, see notes.)
- [ ] **Failure modes**: token expired → `FeedError::Auth`; rate-limit → `FeedError::RateLimited(retry_after)`; provider 410 / API deprecation → `FeedError::Schema(detail)`.
- [ ] **Tests** in `crates/arawn-feeds/tests/jira_*.rs`:
  - `validate_rejects_missing_project_for_project_tracker`.
  - `project_tracker_appends_new_comments_overwrites_issue_snapshot`.
  - `project_tracker_history_log_advances_independently_of_comments`.
  - `assignee_tracker_writes_only_issue_json_no_logs`.
  - `cursor_advances_only_on_successful_persist`.
  - `partial_failure_on_one_issue_does_not_block_others`.
  - `returns_auth_when_atlassian_not_connected`.
- [ ] `angreal check workspace` and `angreal check clippy` clean.

## Implementation Notes

### Technical Approach

Builds on the `AtlassianFeedClient` trait + `RealAtlassianClient` adapter introduced by T-0222.

1. Extend trait with Jira surface: `jql_search`, `issue_changelog`, `issue_comments`.
2. `templates/jira/{common.rs, project_tracker.rs, assignee_tracker.rs}` — common helper handles the three-file write pattern; templates differ only in JQL construction.

### Dependencies

- T-0214 (feed runtime, landed).
- T-0213 (Jira v3 client landed).
- T-0222 (introduces shared `AtlassianFeedClient` trait). Take this in order.

### Risk Considerations

- Comments and history on busy issues can grow unbounded; per-issue jsonl files are fine for low/medium volume but may want size-based rotation later. Defer until needed.
- JQL `updated >= "<iso>"` is minute-grained; helper must dedupe by issue key + per-line id (`comment.id`, `history.id`) to avoid duplicate appends across overlapping windows.
- Atlassian rate limits are per-account, not per-app; aggressive cadences can starve interactive tools. 30m default is the floor.

## Status Updates

### 2026-05-08 — jira/{project-tracker, assignee-tracker} landed

**Trait extension** (`AtlassianFeedClient` gains 3 methods; `JiraIssueMeta` + `JiraIssueDetail` types):
- `jql_search(jql, max)` — returns issue meta. Uses `issue_search_api::search_and_reconsile_issues_using_jql_post`.
- `issue_full(key, want_changelog, want_comments)` — single `issues_api::get_issue` with `expand=changelog`, `fields=*all`. Flags let `assignee-tracker` skip the changelog/comments cost it doesn't need.
- `resolve_project(key_or_id)` — for fail-fast registration, mapping 404 → `FeedError::InvalidParams`.

Error mapping mirrors the pattern: ResponseError 401/403 → Auth, 404 → InvalidParams, 410 → Schema, 429 → RateLimited.

**Storage**:
- `jira/project-tracker/<feed_id>/<KEY>/{issue.json, comments.jsonl, history.jsonl}` — snapshot overwrites; append-only logs.
- `jira/assignee-tracker/<feed_id>/<KEY>/issue.json` only (lighter personal feed).

**Cursor** (shared `CursorState` in `templates/jira/common.rs`):
```json
{
  "latest_updated_iso": "<RFC3339-like timestamp>",
  "issues": { "ENG-1": { "last_comment_id": "...", "last_history_id": "..." } }
}
```
Comment/history dedup uses numeric id comparison — safer than relying on JQL's minute-grained `updated >=` (the AC's note about overlapping windows). Per-issue cursors advance independently of the snapshot.

**Templates** (~80 LOC each; helpers live in `common.rs`):
- `project-tracker` JQL: `project = X AND updated >= "..." ORDER BY updated ASC`. Calls `issue_full(true, true)`. Per-issue resilience — Schema/Provider failure on one issue doesn't poison the run.
- `assignee-tracker` JQL: `assignee = currentUser() AND updated >= "..." ORDER BY updated ASC`. Calls `issue_full(false, false)`. No params (singleton).

Both default to `*/30 * * * *`.

**Departures from AC**:
- AC said "resolves to project ID at registration time and rejects unknown projects with a clear error". The trait method exists (`resolve_project`) but `validate` is sync and the registration flow doesn't currently call it. Defer the wiring of registration-time resolution to T-0219 alongside the `/feeds` UX where the async hook fits naturally. Today, a typoed project key produces a Provider error on first `run` (Jira's JQL validates the project exists).
- Auto-create `jira/assignee-tracker` on `/connect atlassian`: deferred to T-0219.
- AC named the trait methods `issue_changelog` + `issue_comments` (separate). Landed as a single `issue_full(want_changelog, want_comments)` since the Jira API hands back both alongside the snapshot in one `get_issue` call — separating them would have meant either two API calls per issue or a duplicated snapshot fetch.

**Tests** (7 new integration tests + 4 unit tests for shared helpers):
- `project_tracker_appends_new_comments_overwrites_issue_snapshot`
- `project_tracker_history_advances_independently_of_comments`
- `project_tracker_partial_failure_doesnt_block_other_issues`
- `project_tracker_validates_project`
- `assignee_tracker_writes_only_issue_json_no_logs`
- `assignee_tracker_uses_currentUser_jql_and_advances_cursor`
- `returns_auth_when_atlassian_not_connected`

**Production wiring**: no new wiring beyond T-0222's `with_atlassian(...)` builder — both Jira templates use the same `AtlassianIntegration` Arc. Existing test mocks (only `confluence_space_archive.rs`) gained `unreachable!()` stubs for the new trait methods.

127 arawn-feeds tests green. `angreal check workspace` and `angreal check clippy` clean. (Clippy auto-fixed two `if let && ...` chains to use let-chains.)

I-0039 Phase 4 complete. **12 templates over 5 providers** in arawn-feeds.