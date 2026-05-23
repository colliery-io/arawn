---
id: projectionstore-fts-search-doesn-t
level: task
title: "ProjectionStore::fts_search doesn't ensure schema — fails on unwritten feed types"
short_code: "ARAWN-T-0371"
created_at: 2026-05-20T12:50:00+00:00
updated_at: 2026-05-20T13:55:31.381751+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#search"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# fts_search must ensure schema, like the other read paths

## Objective

`ProjectionStore::fts_search` queries `{feed_type}_fts MATCH …`
but doesn't call `ensure_feed_type_tables` first. So a feed
type that has never had a write (or any call to `count` /
`missing_source_ids` / `vector_search` — all of which DO ensure)
has no FTS table, and the query fails with:

```
fts (jira_history): storage: prepare fts: no such table: jira_history_fts
```

Surfaced during the I-0052 UAT post-T-0370 re-run: the agent
searched `feed_search({query: "RFC-0042 sign-off Alice"})` with
no `feed_types` filter, which fans out to every known feed
type including `jira_history`. The fixture didn't seed jira
data, so the table was never created.

## Root cause

`crates/arawn-projections/src/store.rs::fts_search` is missing
the `schema::ensure_feed_type_tables(&conn, feed_type)?` call
that every other read method (`vector_search`, `count`,
`missing_source_ids`, `count_for_feed`, `get_row`) already
makes.

## Fix

One-line: add `schema::ensure_feed_type_tables(&conn, feed_type)?;`
inside `fts_search` after the lock acquire, matching the
pattern used by `vector_search`. `ensure_feed_type_tables`
issues `CREATE … IF NOT EXISTS` so it's idempotent and safe to
call on every search. The brief write-lock to create an empty
table on first search is acceptable (post-T-0366 we no longer
hold long write locks).

## Acceptance criteria

- [x] `fts_search` calls `ensure_feed_type_tables` before its
  query (matching `vector_search` and the other read paths).
- [x] Regression test `search_unwritten_feed_type_returns_empty_not_error`
  searches a never-written feed type (`jira_history` — the
  exact UAT failure mode) and asserts empty result without
  error.
- [x] `angreal test unit` green. `angreal check workspace` green.

## Status Updates — 2026-05-20

Landed in `4d1afc3`. One-line fix in `ProjectionStore::fts_search`
matching `vector_search`'s pattern. 11/11 fts_escape tests pass
(+1 regression).

Surfaced during ARAWN-I-0052 UAT post-T-0370 re-run
(2026-05-20).