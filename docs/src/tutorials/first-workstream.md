# Your first workstream

*Tutorial — about twenty minutes to take a connected integration and turn it into a queryable knowledge graph the agent can reason over.*

By the end of this tutorial you'll have:

- A workstream named `work` with a small tag ontology.
- A feed bound to it (we'll use Gmail in the worked example).
- A few extracted **decisions**, **conventions**, or **facts** the extractor pulled out of recent emails.
- Working `signal_search` queries answering questions about your own data.

If something here feels abstract, the [explanation pages](../explanation/three-layer-data-model.md) cover the *why* in depth. This tutorial focuses on the *how* and the *what does it feel like*.

## What you need

- **arawn server running** (from [your first chat session](./first-chat.md)).
- **At least one integration connected.** Easiest path is Gmail — follow [Connect Google](../how-to/connect-google.md) if you haven't. The rest of this tutorial assumes Gmail; substitute whichever provider you have.
- **About 10 minutes of activity** in that account in the past day, so there's something for the extractor to find.

## Concept in one paragraph

A **workstream** is a logical container for one ongoing concern in your life — a project, a job, a team, a campaign. Sessions, feeds, and the extracted knowledge graph all belong to a workstream. Binding a **feed** (a continually-mirrored source like your inbox) to a workstream tells arawn: *"watch this source, and use it to populate this workstream's knowledge graph."* The extractor reads each new feed item and writes typed entities — decisions, conventions, facts, references — that you can query with `signal_search`.

## 1. Create the workstream

In the TUI:

```
/workstream create work
```

The agent walks you through a short ontology-proposal flow. It asks what the workstream is about, looks at the connected integrations, and proposes 5-12 tags — things like `architecture`, `hiring`, `vendor-meta` for a typical software-team workstream. Confirm or edit the list.

> **About the ontology:** these tags are the *closed list* the extractor will use to label entities. Keep them broad enough to cover what shows up regularly. The steward can suggest new tags later via [`workstream_refine`](../reference/steward-subroutines.md) — you don't need to get it perfect now.

Once you confirm, the workstream is created and arawn switches to it. The status bar shows `work` as the active workstream.

## 2. Bind a feed

List your feeds:

```
/feeds
```

You should see at least one Gmail feed (the auto-created `gmail/inbox-archive` from `/connect gmail`). Note its `feed_id`. Then ask the agent to bind it:

```
bind the work workstream to <feed_id>
```

The agent calls `workstream_bind { workstream: "work", uri: "<feed_id>" }`. (Binding is an agent tool, not a slash subcommand — see [bind a workstream to a feed](../how-to/bind-a-workstream-to-a-feed.md).) Other URI schemes work too — for example, `github:repo:owner/name` if you have GitHub connected.

After bind, extraction starts on the next feed run (or immediately for already-mirrored rows via the backfill loop).

## 3. Trigger an immediate run

You can wait for the next scheduled run, or kick it now:

```
/feeds run <feed_id>
```

This forces a fresh poll and, because the feed is bound to a workstream, also runs the extractor over any new projection rows.

In the server log you'll see lines like `extraction.run feed=… rows=N entities=M`. The `entities=M` is what the extractor pulled out.

## 4. Ask the agent what it found

Back in the TUI:

```
What's new in my work emails?
```

The agent will reach for the `signal_search` tool, which queries the populated palace. It returns extracted entities (decisions, conventions, facts) with their source-row pointers, then the LLM summarizes them.

Try more specific queries:

```
What did we decide about postgres recently?
```

```
Who's been asking about the new hire process?
```

The agent will call `signal_search` or `signal_query` under the hood. You can call the tools directly too — they're documented in [workstream tools](../reference/workstream-tools.md).

## 5. Look at the typed structure

```
signal_query { entity_type: "decision", since: "2026-04-01T00:00:00Z" }
```

This returns every decision-typed entity extracted since the date. Each row has:

- The decision text the extractor pulled.
- The source projection row (a specific email).
- The ontology tags the extractor assigned.
- A confidence score.

```
signal_timeline { limit: 10 }
```

Shows the last 10 entities the extractor wrote, in order.

## 6. Peek at what the steward wants to do

Periodically the **steward** proposes maintenance — suggesting new ontology tags for entities it had to label with a low-confidence catch-all, proposing relations between entities, summarizing cold material into "dust" digests. To see what's pending:

```
workstream_refine
```

This returns a list of proposals. Each has a unique journal id. You can review them with:

```
workstream_apply <journal_id>
```

…or undo with `workstream_rollback <journal_id>`. The full flow is in [curate a workstream](../how-to/curate-a-workstream.md).

## What just happened

You set up the three-layer data model end-to-end:

1. **The feed** continually mirrors a slice of Gmail to disk under `<data_dir>/data/gmail/inbox-archive/<feed_id>/`. Each new email becomes a row.
2. **The projection** is the typed flat record per row — sender, subject, body, threading metadata.
3. **The palace** is the knowledge graph the extractor builds *on top of* projections — typed entities (decisions, conventions, facts), tags from your ontology, source-row pointers back to the projection.

The agent reads the palace, not the raw inbox — that's what makes `signal_search "what did we decide about postgres?"` actually work.

## What's next

- **Curate.** As the steward suggests new tags or relations, review with `workstream_refine`. See [curate a workstream](../how-to/curate-a-workstream.md).
- **Connect more sources.** Bind more feeds to the same workstream — Slack channels, Jira projects, Drive folders. Each new feed feeds the same palace.
- **Understand the parts.** [Palaces](../explanation/palaces.md), [extraction](../explanation/extraction.md), [steward](../explanation/steward.md), and the [three-layer data model](../explanation/three-layer-data-model.md) all live in the explanation quadrant.
- **Look up tools.** [workstream tools](../reference/workstream-tools.md) lists all `workstream_*` and `signal_*` operations.
