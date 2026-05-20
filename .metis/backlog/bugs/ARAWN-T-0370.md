---
id: feed-search-fts5-query-escaping
level: task
title: "feed_search: FTS5 query escaping — hyphens / colons / punctuation break match"
short_code: "ARAWN-T-0370"
created_at: 2026-05-20T11:55:38.676786+00:00
updated_at: 2026-05-20T13:51:51.313333+00:00
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

# feed_search: FTS5 query escaping

## Objective

The `feed_search` tool passes the user-supplied query string
straight into SQLite FTS5 `MATCH`. FTS5 has special grammar
(column-scoped queries via `column:term`, NEAR/AND/OR/NOT
operators) that turns natural identifiers like `RFC-0042` or
phrases like `sign-off` into syntax errors masquerading as
"no such column" failures.

Surfaced during the I-0052 UAT judge pass: the
`draft-with-confirmation` scenario asked the agent to find an
email about "RFC-0042 sign-off Alice". The agent issued 11
sequential `feed_search` calls. Every variation failed:

```
fts (gmail_messages): storage: no such column: 0042   ← RFC-0042
fts (gmail_messages): storage: no such column: off    ← sign-off
fts (gmail_messages): storage: no such column: 0042   ← RFC 0042 (still hyphenated through tokenizer)
fts (jira_history): storage: prepare fts: no such table: jira_history_fts
```

The agent gave up and asked the user to clarify instead of
producing a draft. UAT mechanical PASS (it didn't send a
message without confirmation) but judge FAIL
(completion=2/5, quality=1/5).

## Impact

- **Severity:** P2 — search is a core agent surface. Any
  natural identifier with a hyphen, colon, or other punctuation
  fails today. This is most agent searches for tickets,
  product names, dates, URLs.
- **Affected paths:** `feed_search` (the agent-facing tool),
  any caller of `ProjectionStore::fts_search`.
- **Recovery is impossible:** the agent has no way to know
  FTS5 grammar; even rephrasing fails because the offending
  characters live in the *content* the user named.

## Reproduction

```rust
// crates/arawn-projections/src/store.rs::fts_search
self.fts_search("gmail_messages", "RFC-0042", 10)
// → Err("no such column: 0042")
```

Smoke test on a populated fixture:

```bash
echo '{"tool":"feed_search","arguments":{"query":"RFC-0042"}}' \
  | arawn-debug-tool-call
# → "no such column: 0042"
```

## Root cause

`crates/arawn-projections/src/store.rs:214 fts_search`:

```rust
let sql = format!(
    "SELECT projection_id FROM {feed_type}_fts \
     WHERE {feed_type}_fts MATCH ?1 ORDER BY rank LIMIT ?2"
);
// ...
stmt.query_map(params![query, limit as i64], ...)
```

`query` is passed verbatim. FTS5 interprets:
- `term1:term2` → column-scoped query (term2 in column term1).
- `term1-term2` → tokenises as `term1` then `NOT term2` if
  `-` is at a token boundary, OR treats as a column lookup
  with `term1` as column name.
- Bare `:` or unbalanced quotes → syntax error.

Combined with our schema (column names are inferred from
projection fields), `"RFC-0042"` triggers a "no such column"
lookup against the FTS table.

## Fix

Quote-and-escape each whitespace-separated token before
passing to MATCH. Wrap each token in double quotes (FTS5
treats `"…"` content as a literal phrase — no operator
parsing inside). Inside a token, escape any embedded `"` by
doubling it (`""`) per FTS5 quoting rules.

```rust
fn escape_fts5(query: &str) -> String {
    query
        .split_whitespace()
        .map(|tok| {
            let escaped = tok.replace('"', "\"\"");
            format!("\"{escaped}\"")
        })
        .collect::<Vec<_>>()
        .join(" ")
}
```

`"RFC-0042 sign-off Alice"` → `"\"RFC-0042\" \"sign-off\" \"Alice\""`
which FTS5 parses as three phrase-literal AND-conjoined terms.
Implicit AND is the existing semantics we want.

This drops the ability for callers to use FTS5 operators
directly (e.g. `column:term`, `term OR other`). Those weren't
exposed as a feature; the agent doesn't know FTS5 grammar.

## Out of scope

- The companion error `no such table: jira_history_fts` is a
  separate bug — the `jira_history` projection never built its
  FTS index. File separately if encountered post-fix.

## Acceptance criteria

- [x] `ProjectionStore::fts_search` escapes the input query
  via the new public `escape_fts5` helper before passing to
  FTS5 MATCH. Empty / whitespace-only queries short-circuit
  to an empty result (no SQLite call).
- [x] Unit tests (5 on the escape helper):
  - Empty → empty.
  - Each token quote-wrapped.
  - Hyphenated identifiers neutralised
    (`RFC-0042` / `sign-off`).
  - Colon + parens neutralised.
  - Embedded `"` doubled per FTS5 quoting.
- [x] End-to-end FTS5 tests (5 against a real SQLite store):
  - Hyphenated identifier matches.
  - Hyphenated phrase matches.
  - Multi-token implicit-AND.
  - Colon doesn't trigger column lookup.
  - Empty query returns empty without error.
- [x] `angreal test unit` green. `angreal check workspace` green.
- [ ] UAT `draft-with-confirmation` re-run with hyphenated
  queries returning non-empty — deferred to the next full UAT
  pass (98 min) rather than running scenario-specific UAT just
  for this fix. The 10 inline tests exercise the exact
  bug-triggering tokens.

## Status Updates — 2026-05-20

Landed.

- `escape_fts5(query: &str) -> String` added to
  `crates/arawn-projections/src/store.rs`. Whitespace-tokenises
  the input, doubles any embedded `"`, wraps each token in
  `"…"`. Implicit AND between tokens preserved.
- `ProjectionStore::fts_search` applies the escape and
  short-circuits on empty.
- 10 new tests (5 unit on the escape fn, 5 end-to-end on a
  real FTS5 store). All pass.
- Full workspace test suite + `angreal check workspace` green.

Trade-off accepted: FTS5 operator syntax (`OR`,
`column:term`, etc.) is no longer available to callers. Wasn't
a feature anyone used; the agent doesn't know FTS5 grammar.

Ready for review.

Surfaced during ARAWN-I-0052 UAT judge pass (2026-05-20).