# Bind a lens to a feed

*How-to. Attach a feed (or a whole repo / org) to a lens so the extractor turns each new item into typed entities you can query with `signal_search`.*

A lens by itself is just a label. A feed by itself is just a local mirror. Binding the two tells arawn: *"watch this source, and use it to populate this lens's knowledge graph."*

## Prerequisites

- A lens exists (`/lens create work`).
- A feed exists (auto-created via `/connect` or explicit `/watch` — see [create a feed](./create-a-feed.md)). Or, for GitHub, the GitHub App is installed (see [connect GitHub](./connect-github.md)).

## How to bind: ask the agent

Lens binding is an agent-tool operation (`lens_bind`), not a slash subcommand. The TUI dispatcher only accepts `/lens {create | list | switch}`; everything else goes through the agent.

Just ask, e.g.:

```
bind the work lens to gmail-inbox-me
```

The agent calls `lens_bind { lens: "work", uri: "gmail-inbox-me" }`. Extraction starts on the next feed run; to force it now, ask the agent to run the feed (or use `/feeds run <feed_id>` directly).

## Direct bind by feed_id

Three example tool calls (these are what the agent runs under the hood — useful for scripting via WS-RPC):

```jsonc
lens_bind { "lens": "work", "uri": "gmail-inbox-me" }
lens_bind { "lens": "work", "uri": "slack-design" }
lens_bind { "lens": "work", "uri": "jira-ENG" }
```

## GitHub URI binds

GitHub has two extra URI schemes that handle the feed registration for you.

### `github:repo:owner/name`

Binds a single repository. arawn registers a `github/repo-mirror` feed for that repo and ties it to the lens:

```jsonc
lens_bind { "lens": "work", "uri": "github:repo:acme/platform-api" }
```

### `github:org:owner`

Binds a whole organization. arawn calls `list_org_repos`, registers one `github/repo-mirror` feed per repo, and ties them all to the lens. Org binds **supersede** any per-repo binds in the same lens — if you already had `github:repo:acme/platform-api` bound and then add `github:org:acme`, the per-repo bind is dropped in favor of the org-wide one.

```jsonc
lens_bind { "lens": "work", "uri": "github:org:acme" }
```

Or, in chat: *"bind work to the acme org on GitHub."*

> **Note:** all four GitHub templates (`github/notifications`, `github/issues-and-prs`, `github/review-queue`, `github/repo-mirror`) are read-only. v1 of the GitHub integration doesn't write back.

## Hot-register

Bind operations are live — the feed registration happens immediately (no server restart needed). The new feeds appear in `/feeds` right away and start their cron schedule on the next tick. For an immediate first run, ask the agent (or call `/feeds run <feed_id>` directly).

## Unbinding

Ask the agent (*"unbind the github:org:acme binding from work"*), which calls:

```jsonc
lens_unbind { "lens": "work", "uri": "github:org:acme" }
```

Removes the binding. The feed itself keeps running (you can still query it directly); the lens just stops absorbing its rows. For `github:org:` binds, unbinding removes all per-repo feeds the org-expansion created.

## Verifying

Ask the agent to show the lens, or call the tool directly:

```jsonc
lens_show { "name": "work" }
```

Lists the bindings on the lens. After a successful bind you should see your feed_id or the GitHub URI listed.

After extraction has run at least once, the palace will have entities:

```
signal_query { entity_type: "decision", limit: 5 }
```

If this returns rows, extraction is working.

## What's next

- Query the resulting palace: [read feeds with the agent](./read-feeds-with-the-agent.md), or the [first lens tutorial](../tutorials/first-lens.md).
- Curate steward proposals: [curate a lens](./curate-a-lens.md).
- Understand why lenses and feeds are separate concepts: [explanation: lenses](../explanation/lenses.md).
