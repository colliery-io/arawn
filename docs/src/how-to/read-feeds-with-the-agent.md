# Agent Read Patterns

Feeds aren't worth much if the agent can't navigate them. The deal
is: the agent uses the same `Read`, `Glob`, and `Grep` tools it uses
on source code, and the feed layout is shaped so those tools are
enough.

This page is recipe-shaped: real prompts, the tool calls the agent
makes, and why the layout lets it work.

> All paths are relative to `~/.arawn/data/`.

## Slack channel: "what happened in #design today?"

**Prompt:** *summarize what happened in #design today*

```
1. Glob   slack/channel-archive/design/2026-05-11.jsonl
2. Read   slack/channel-archive/design/2026-05-11.jsonl
3. Glob   slack/channel-archive/design/threads/*.jsonl  (filter by date)
4. Read   the thread files that overlap today
```

The agent doesn't need a separate query API — day-partitioned JSONL is
greppable and human-readable. For a busy day the agent might read only
the parent file and pull threads on demand for the conversations that
matter.

## Slack mentions: "did anyone @ me about the launch?"

**Prompt:** *did anyone mention me about the launch this week?*

```
1. Glob   slack/my-mentions/me/2026-05-*.jsonl
2. Grep   "launch" in those files (case-insensitive)
3. Read   matching lines + context
```

`my-mentions` is mentions-only, so the agent doesn't have to filter
the world down to "messages aimed at me" — that's the feed's job.

## Gmail by sender: "anything from boss@x.com this week?"

**Prompt:** *did boss@example.com email me anything this week?*

Two routes depending on whether a sender-specific feed exists.

**Route A — dedicated `gmail/sender-filter` feed:**

```
1. Glob   gmail/sender-filter/boss-at-example/2026-05-*/*.json
2. Read   each match (full message JSON: subject, body, snippet)
```

**Route B — falling back to inbox-archive:**

```
1. Glob   gmail/inbox-archive/me/2026-05-*/*.json
2. Grep   "boss@example.com" in those files
3. Read   the matching message JSONs
```

Route A is faster if the user knows which senders they care about; B
works without setup but does more work per query. Both produce the
same answer.

## Jira: "what's on my plate?"

**Prompt:** *what Jira issues are assigned to me right now?*

```
1. Glob   jira/assignee-tracker/me/*/issue.json
2. Read   each (or just the titles via Grep "summary")
```

Snapshots are overwrite-on-update, so what's on disk *is* "currently
assigned to me." No status filter needed — Jira's assignee changes
are reflected within a half-cron-tick (default 30 min).

## Jira: "what was discussed in ENG-742?"

**Prompt:** *what's the latest on ENG-742?*

```
1. Read   jira/project-tracker/ENG/ENG-742/issue.json     (current state)
2. Read   jira/project-tracker/ENG/ENG-742/comments.jsonl (discussion)
3. Read   jira/project-tracker/ENG/ENG-742/history.jsonl  (status changes)
```

Three files, three lenses on the same issue: what it *is*, what
people *said*, what *changed*. The agent can pick whichever the
question needs.

## Calendar: "what's tomorrow look like?"

**Prompt:** *what do I have on tomorrow?*

```
1. Glob   calendar/upcoming-archive/primary/events/*.json
2. Grep   "2026-05-12" in start fields (or filter in-memory after Read)
3. Read   matching event JSONs
```

Calendar's "one file per event id" shape is friendlier to "find events
in window X" than "fetch events for this exact day" because recurring
events expand to many ids without polluting any single day partition.

## Drive folder: "find that report from last week"

**Prompt:** *which file in Reports/2026 had the Q1 budget?*

```
1. Glob   drive/folder-sync/reports-2026/**/*.md
2. Grep   "Q1 budget" in those files
3. Read   the file the grep hit
```

Folder-sync mirrors actual bodies (Google Docs are exported to
markdown), so `grep` and `cat` Just Work. No `drive_read` round trip.

## Drive recent: "what files changed this week?"

**Prompt:** *what Drive files did I touch this week?*

```
1. Glob   drive/recent/me/2026-05-*/*.json
2. Read   names + mime types from the metadata snapshots
```

`drive/recent` is metadata-only, so this is a cheap "what changed"
query. If the agent then wants the body of one file, it can fall back
to the `drive_read` tool with the `id` from the metadata snapshot.

## Cross-feed: "what's the deploy status?"

**Prompt:** *what's the latest on the deploy — has anything blown up?*

```
1. Grep   "deploy" in slack/channel-archive/incidents/2026-05-11.jsonl
2. Grep   "deploy" in slack/my-mentions/me/2026-05-11.jsonl
3. Glob   gmail/inbox-archive/me/2026-05-11/  → Grep "alert" / "deploy"
4. Grep   "deploy" in jira/project-tracker/ENG/*/history.jsonl
```

Cross-feed questions are where local mirroring pays off most — the
agent can hit four different sources in four cheap filesystem reads
instead of four round trips to four different APIs.

## Confluence: "what's our backup runbook?"

**Prompt:** *what's our backup runbook?*

```
1. Grep   "backup" in confluence/space-archive/ENG/*/page.json (titles)
2. Read   body.storage.xml of the matching page
```

Storage-format XML is verbose but searchable. If the agent needs a
cleaner read, it can do a quick XML-to-text pass in-memory.

## Patterns that work well

- **Day partitions** (Slack, Gmail, Drive recent) — perfect for "today",
  "this week", "yesterday" questions. Glob the date pattern.
- **Per-entity directories** (Jira issues, Confluence pages,
  Drive folder-sync) — perfect for "what's the state of X?" questions.
  Read the single file.
- **Append-only logs** (Jira comments/history) — perfect for "what
  happened to X?" timelines.
- **Snapshots** (Calendar events, Jira issue.json) — perfect for
  "what's true now?" questions.

If your question doesn't fit any of these patterns cleanly, consider:
adding a feed that pre-shapes the data the way you ask about it (a
sender-filter feed for a noisy correspondent, a project-tracker for a
recurring topic).

## Patterns to avoid

- **Don't grep `~/.arawn/data` recursively without scoping.** Glob the
  smallest path that contains the answer (`slack/channel-archive/design/2026-05-*`
  beats `slack/**`). The grep tool reads every match — a too-wide pattern
  is slow and noisy.
- **Don't ask the agent to "list all feeds."** Use the `/feeds` slash
  command instead — it reads the runtime DB, which is authoritative.
- **Don't ask "what was on my calendar last week?"** Calendar feeds
  are forward-looking only (`upcoming-archive`); past events fall out
  as they slide before `time_min`. For historical calendar data, use
  the `google_calendar` tool directly.
