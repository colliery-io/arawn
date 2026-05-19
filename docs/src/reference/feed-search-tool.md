# `feed_search` — cross-feed semantic search

`feed_search` is the agent-facing read surface over the projection
layer (see [Continual Data Feeds](./index.md) for context). It's the
no-workstream fallback: when an agent needs to look up something
across all configured feeds without first declaring a workstream
scope, this is the tool to reach for.

Under the hood it queries per-feed-type sqlite tables in
`~/.arawn/projections.db` via FTS5. The projection rows are written
on every successful feed run by `arawn-feeds::dispatch::run_feed`, so
search results stay current with the latest feed cycle.

## Tool surface

```text
feed_search(
  query: string,              // free-text; goes to FTS5
  feed_types: [string]?,      // restrict; default = all known types
  since: string?,             // RFC3339; filter source_ts >= since
  until: string?,             // RFC3339; filter source_ts <= until
  limit: integer?             // 1..=50, default 10
) -> {
  count: integer,
  results: [{
    feed_type: string,        // e.g. "slack_messages", "jira_issues"
    id: string,               // projection row id (stable hash)
    feed_id: string,          // which feed instance produced this
    source_id: string,        // provider's id (gmail message id, slack ts, …)
    source_ts: string,        // RFC3339; the item's authored timestamp
    title: string,            // synthesized per feed type
    snippet: string,          // first 240 chars of body_text
    score: number,            // rank-decayed FTS score (1 / (1 + rank))
    metadata: object,         // per-feed-type fields
  }]
}
```

## Known feed types

| `feed_type`              | Source                              | `metadata` keys |
|--------------------------|-------------------------------------|-----------------|
| `gmail_messages`         | gmail/inbox-archive et al.          | sender, recipients, subject, thread_id, labels |
| `slack_messages`         | slack/channel-archive (top-level)   | channel_id, sender_id, thread_ts, reactions |
| `slack_thread_messages`  | slack/channel-archive (replies)     | channel_id, sender_id, thread_ts, reactions |
| `drive_files`            | drive/folder-sync                   | path, name, mime_type, size_bytes |
| `jira_issues`            | jira/project-tracker, jira/assignee-tracker | project_key, summary, status, assignee, reporter, priority, labels |
| `jira_comments`          | jira/project-tracker                | issue_key, author |
| `jira_history`           | jira/project-tracker (changelog)    | issue_key, field, from, to, author |
| `confluence_pages`       | confluence/space-archive            | space_key, parent_id, version, author |
| `calendar_events`        | calendar/upcoming-archive           | calendar_id, summary, location, start_ts, end_ts, all_day, organizer, attendees, status, recurring_event_id |

## Worked prompts

### "what did alice email me last week about the migration?"

```
feed_search({
  query: "migration",
  feed_types: ["gmail_messages"],
  since: "2026-05-05T00:00:00Z",
  until: "2026-05-12T00:00:00Z",
  limit: 10
})
```

FTS5 matches against the synthesized title (subject) and body. The
`sender` lives in `metadata.sender`; the agent reads it from the
result to confirm "from alice". `since`/`until` constrain the search
to last week.

### "find the slack discussion where we agreed on the rate-limit policy"

```
feed_search({
  query: "rate limit policy",
  feed_types: ["slack_messages", "slack_thread_messages"],
  limit: 20
})
```

Searches both top-level messages and thread replies. The agent picks
threads via the `thread_ts` in `metadata` and can pull the full
thread context from the on-disk mirror if needed (see
[Agent Read Patterns](./agent-read-patterns.md)).

### "any open jira issues mentioning the auth refactor?"

```
feed_search({
  query: "auth refactor",
  feed_types: ["jira_issues"],
  limit: 25
})
```

Title is synthesized as `"ENG-123: Migrate auth"`, so the project key
and issue summary both score against the query. The agent filters on
`metadata.status` to drop closed issues.

### "did the q3 planning doc get updated this month?"

```
feed_search({
  query: "Q3 planning",
  feed_types: ["drive_files", "confluence_pages"],
  since: "2026-05-01T00:00:00Z"
})
```

Cross-source query: covers both Drive docs and Confluence pages with
a single call. The agent compares `source_ts` to identify the most
recent revision.

### "show me everything about the kafka outage in the last 48 hours"

```
feed_search({
  query: "kafka outage",
  since: "2026-05-10T00:00:00Z"
})
```

No `feed_types` filter → searches all 9 types. Useful when the agent
doesn't yet know which surface the conversation happened on.

### "summarize today's standups"

```
feed_search({
  query: "standup",
  feed_types: ["slack_messages"],
  since: "2026-05-12T00:00:00Z",
  until: "2026-05-13T00:00:00Z",
  limit: 30
})
```

For higher-volume slack channels, lift `limit` to 50 and aggregate
client-side.

### "what's on my calendar this week?"

```
feed_search({
  query: "",   // empty query falls back to time-window only
  feed_types: ["calendar_events"],
  since: "2026-05-12T00:00:00Z",
  until: "2026-05-19T00:00:00Z",
  limit: 50
})
```

For pure time-range queries the agent can also walk `metadata.start_ts`
manually after pulling a wide page; the on-disk mirror at
`~/.arawn/data/calendar/upcoming-archive/<feed_id>/events/` is also
greppable.

## What `feed_search` is **not** for

- **Workstream-scoped knowledge.** Use `memory_search` for facts /
  decisions / preferences the agent has explicitly stored, or wait
  for the upcoming `signal_search` (Phase 6 of [I-0040](../../../.metis/initiatives/ARAWN-I-0040/initiative.md)).
- **Cross-projection JOINs.** "Which jira issue does this slack
  message reference?" needs the extractor that lands in Phase 4. Today
  the agent has to do that linking manually by reading the slack
  message and checking the result text for issue keys.
- **Bulk listing.** For "all gmail from alice" without a content
  filter, walk the on-disk mirror directly — FTS is built for
  relevance ranking, not for set iteration.

## Ranking model

Hybrid FTS5 + semantic similarity, fused via reciprocal rank fusion
(RRF). For each feed type:

1. **FTS5 ranked list** — BM25-like text match over title + body_text.
2. **Vector ranked list** — `vec0` (sqlite-vec) similarity search
   against the per-feed-type vector table (when an embedder is
   configured at startup). Distance is ascending; closer wins.

Both lists contribute `1 / (k + rank + 1)` to the row's score with
`k = 60` (Cormack et al. 2009). A row appearing in both lists gets
roughly double the score of a row appearing in only one, which is
the desired "agreement is signal" behavior.

If no embedder is configured (or query embedding fails) the tool
falls back to FTS-only and the score is just the FTS contribution.
The result shape is identical either way; the score column is
comparable within a single response but not across calls.

Cross-feed-type merging is done after each per-type fusion so a
high-confidence slack match can outrank a fuzzy gmail one even when
more gmail rows match.

## Operational notes

- The projections db is created lazily at `~/.arawn/projections.db`
  on first feed run.
- A schema gets created per feed type on first write; an empty
  `feed_search` (no rows for a type yet) returns no results rather
  than erroring.
- Embeddings are filled in by a background pass that runs every
  5 minutes (and once at startup). Rows projected in the last few
  minutes may be FTS-only until the next embed cycle catches up.
- `jira_history` is FTS-only by design — its body is too thin
  ("`<field>` changed `<from>` → `<to>`") to embed usefully.
- Stale projection rows (after a feed item is deleted upstream) are
  not auto-pruned; this is acceptable today and revisited in Phase 5.
