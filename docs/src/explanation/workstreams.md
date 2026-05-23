# Workstreams

*Explanation. What a workstream is, when to create one, how it relates to sessions/feeds/palaces/identity, and what scratch is for.*

A **workstream** is the primary organizational unit in arawn — a logical container for one ongoing concern in your life. Sessions, feeds, knowledge bases, and identity all bind to workstreams. The vision puts them at the center: *"Each workstream partitions data at the filesystem level. Sessions, watchers, and action items all belong to a workstream."*

For the slug rules, lifecycle CLI, and metadata fields, see [workstream CLI reference](../reference/workstream-cli.md). For the agent-facing tools, see [workstream tools reference](../reference/workstream-tools.md). This page is about *why* workstreams.

## What a workstream actually is

A directory plus a database row.

```
<data_dir>/workstreams/<slug>/
  ├── memory.db         # the workstream's knowledge base (palace)
  └── workspace/        # FS-isolated working directory for shell + file tools
```

Plus a row in the global `workstreams` table carrying:

- `name` (slug), `display_name`, `description`
- `bindings[]` (feeds and/or URI schemes attached to this workstream)
- `tags_ontology[]` (the closed list the extractor may use)
- `identity_profile` (`assistant` default, or `coding`)
- `archived` flag

The directory is where the agent's writes land (in `workspace/`) and where the extractor builds the palace. The DB row is the configuration.

## Why workstreams exist

Three problems they solve:

### 1. Locality of knowledge

A single global knowledge base would mix everything — work decisions, home maintenance, hobbies, errands. `signal_search "deployment"` would return three different meanings of "deployment" from three different parts of your life.

Per-workstream palaces give the agent a focused context. "What did we decide about X?" in workstream A returns A's decisions, not B's. The same query in B returns B's decisions.

### 2. Per-topic vocabulary

Each workstream has its own tag ontology — 5-12 slugs the extractor uses to tag entities. A "work" workstream's ontology (`postgres`, `ledger`, `migration`, `hiring`) doesn't help a "home" workstream's queries. One global ontology can't reasonably span both.

The ontology is declared at workstream creation (the `workstream_propose_ontology` step in the create flow) and grows via the `tag-promoter` steward subroutine. See [palaces explanation](./palaces.md).

### 3. Identity routing

Different workstreams want different agent personas. A `coding` workstream wants the engineering tool — bug fixes, refactoring, code explanation. An `assistant` workstream (most workstreams) wants the personal-assistant — summarize, check, nudge.

The `identity_profile` column on the workstream picks the persona. See [identity by workstream](./identity-by-workstream.md).

## When to create a workstream

| Create a workstream when... | Don't, when... |
|---|---|
| You'll come back to this topic over weeks or months | One-off question; use scratch |
| Multiple feeds make sense for it | You're not connecting integrations to it |
| You want a tag ontology for it | You don't have enough recurring concepts to define 5+ tags |
| You want the steward maintaining a KB about it | You won't query the KB |

A workstream without a binding is harmless — extraction has nothing to do. A workstream with a binding but no queries piles up data the agent never reads. **Bind workstreams to the topics you actually ask about.**

## Scratch: the always-available default

The `scratch` workstream is auto-created on first boot and undeletable. It's where one-off / ad-hoc sessions live when you haven't picked a workstream.

- The agent works fine in scratch. You don't need to create a workstream to use arawn.
- Memory entries created in scratch are tagged as such.
- If a scratch session turns into ongoing work, **promote** it: `/promote <name>` or the `workstream_promote` tool. The session moves under the new workstream; its history, memory entries, and any feed bindings rebase.

The promote path is the deliberate "this conversation matters enough to track" choice. Until you promote, sessions are ephemeral-ish — they persist on disk but aren't first-class.

## Workstream vs. session

A **session** is one chat conversation — a sequence of turns with the LLM. Sessions belong to a workstream. Many sessions per workstream, but always exactly one workstream per session.

Switching workstream mid-conversation is rare; usually you start a new session in the new workstream.

## Workstream vs. feed

A feed mirrors upstream content to disk. A workstream is the *interpretation* of feeds — the typed entities the extractor builds, the queries you run against them.

The relationship is many-to-many but per-feed-config:

- One workstream can bind many feeds (your `work` workstream might bind Gmail + Slack + Jira).
- One feed can bind to many workstreams (your `gmail/inbox-archive` could feed both `work` and `personal` workstreams; both extractors run on each new email).

[Bind a workstream to a feed](../how-to/bind-a-workstream-to-a-feed.md) is the recipe.

## Workstream vs. memory

Memory has two tiers: **global** (`<data_dir>/memory.db`) and **workstream** (`<data_dir>/workstreams/<name>/memory.db`).

- Preferences and Person entities are scope-locked to global.
- Decisions, conventions, facts, notes are scope-locked to workstream.

The split is for utility — your `prefer Tokio over async-std` preference should apply in every workstream; the `we use Postgres 16` decision shouldn't bleed across workstreams.

[Memory design](./memory-design.md) covers the rationale.

## Filesystem isolation

The `workstreams/<name>/workspace/` directory is the writable root for shell + file tools when this workstream is active. Writes outside `workspace/` fail with `Permission denied` at the sandbox layer.

This composes with the [shell sandbox](../reference/shell-sandbox.md) to ensure tools can only access data within their workstream's partition — a safety prerequisite, not a later enhancement. Watchers and chat-driven actions run autonomously; per-workstream FS partitioning bounds what they can do.

## When NOT to use workstreams

- **You're tinkering.** Scratch is fine for tinkering.
- **You have one workstream and won't add more.** That's fine too — the abstraction doesn't cost anything when you don't use it.
- **You're not connecting integrations.** Workstreams' main value is in binding integrations; without integrations, you're just creating a folder for a label.

The cost of a workstream is low: a directory and a row. The cost of *not* creating a workstream when you should is a polluted global KB.

## Related

- [Workstream CLI reference](../reference/workstream-cli.md) — slug rules, lifecycle.
- [Workstream tools reference](../reference/workstream-tools.md) — `signal_*`, `workstream_*`.
- [Palaces explanation](./palaces.md) — what lives in the workstream KB.
- [Identity by workstream](./identity-by-workstream.md) — `identity_profile`.
- [Memory design](./memory-design.md) — global vs. workstream tiers.
