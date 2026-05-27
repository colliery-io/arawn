# Lenses

*Explanation. What a lens is, when to create one, how it relates to sessions/feeds/palaces/identity, and what scratch is for.*

A **lens** is the primary organizational unit in arawn — a logical container for one ongoing concern in your life. Sessions, feeds, knowledge bases, and identity all bind to lenses. The vision puts them at the center: *"Each lens partitions data at the filesystem level. Sessions, watchers, and action items all belong to a lens."*

For the slug rules, lifecycle CLI, and metadata fields, see [lens CLI reference](../reference/lens-cli.md). For the agent-facing tools, see [lens tools reference](../reference/lens-tools.md). This page is about *why* lenses.

## What a lens actually is

A directory plus a database row.

```
<data_dir>/lenses/<slug>/
  ├── memory.db         # the lens's knowledge base (palace)
  └── workspace/        # FS-isolated working directory for shell + file tools
```

Plus a row in the global `lenses` table carrying:

- `name` (slug), `display_name`, `description`
- `bindings[]` (feeds and/or URI schemes attached to this lens)
- `tags_ontology[]` (the closed list the extractor may use)
- `identity_profile` (`assistant` default, or `coding`)
- `archived` flag

The directory is where the agent's writes land (in `workspace/`) and where the extractor builds the palace. The DB row is the configuration.

## Why lenses exist

Three problems they solve:

### 1. Locality of knowledge

A single global knowledge base would mix everything — work decisions, home maintenance, hobbies, errands. `signal_search "deployment"` would return three different meanings of "deployment" from three different parts of your life.

Per-lens palaces give the agent a focused context. "What did we decide about X?" in lens A returns A's decisions, not B's. The same query in B returns B's decisions.

### 2. Per-topic vocabulary

Each lens has its own tag ontology — 5-12 slugs the extractor uses to tag entities. A "work" lens's ontology (`postgres`, `ledger`, `migration`, `hiring`) doesn't help a "home" lens's queries. One global ontology can't reasonably span both.

The ontology is declared at lens creation (the `lens_propose_ontology` step in the create flow) and grows via the `tag-promoter` steward subroutine. See [palaces explanation](./palaces.md).

### 3. Identity routing

Different lenses want different agent personas. A `coding` lens wants the engineering tool — bug fixes, refactoring, code explanation. An `assistant` lens (most lenses) wants the personal-assistant — summarize, check, nudge.

The `identity_profile` column on the lens picks the persona. See [identity by lens](./identity-by-lens.md).

## When to create a lens

| Create a lens when... | Don't, when... |
|---|---|
| You'll come back to this topic over weeks or months | One-off question; use scratch |
| Multiple feeds make sense for it | You're not connecting integrations to it |
| You want a tag ontology for it | You don't have enough recurring concepts to define 5+ tags |
| You want the steward maintaining a KB about it | You won't query the KB |

A lens without a binding is harmless — extraction has nothing to do. A lens with a binding but no queries piles up data the agent never reads. **Bind lenses to the topics you actually ask about.**

## Scratch: the always-available default

The `scratch` lens is auto-created on first boot and undeletable. It's where one-off / ad-hoc sessions live when you haven't picked a lens.

- The agent works fine in scratch. You don't need to create a lens to use arawn.
- Memory entries created in scratch are tagged as such.
- If a scratch session turns into ongoing work, **promote** it: `/promote <name>` or the `lens_promote` tool. The session moves under the new lens; its history, memory entries, and any feed bindings rebase.

The promote path is the deliberate "this conversation matters enough to track" choice. Until you promote, sessions are ephemeral-ish — they persist on disk but aren't first-class.

## Lens vs. session

A **session** is one chat conversation — a sequence of turns with the LLM. Sessions belong to a lens. Many sessions per lens, but always exactly one lens per session.

Switching lens mid-conversation is rare; usually you start a new session in the new lens.

## Lens vs. feed

A feed mirrors upstream content to disk. A lens is the *interpretation* of feeds — the typed entities the extractor builds, the queries you run against them.

The relationship is many-to-many but per-feed-config:

- One lens can bind many feeds (your `work` lens might bind Gmail + Slack + Jira).
- One feed can bind to many lenses (your `gmail/inbox-archive` could feed both `work` and `personal` lenses; both extractors run on each new email).

[Bind a lens to a feed](../how-to/bind-a-lens-to-a-feed.md) is the recipe.

## Lens vs. memory

Memory has two tiers: **global** (`<data_dir>/memory.db`) and **lens** (`<data_dir>/lenses/<name>/memory.db`).

- Preferences and Person entities are scope-locked to global.
- Decisions, conventions, facts, notes are scope-locked to lens.

The split is for utility — your `prefer Tokio over async-std` preference should apply in every lens; the `we use Postgres 16` decision shouldn't bleed across lenses.

[Memory design](./memory-design.md) covers the rationale.

## Filesystem isolation

The `lenses/<name>/workspace/` directory is the writable root for shell + file tools when this lens is active. Writes outside `workspace/` fail with `Permission denied` at the sandbox layer.

This composes with the [shell sandbox](../reference/shell-sandbox.md) to ensure tools can only access data within their lens's partition — a safety prerequisite, not a later enhancement. Watchers and chat-driven actions run autonomously; per-lens FS partitioning bounds what they can do.

## When NOT to use lenses

- **You're tinkering.** Scratch is fine for tinkering.
- **You have one lens and won't add more.** That's fine too — the abstraction doesn't cost anything when you don't use it.
- **You're not connecting integrations.** Lenses' main value is in binding integrations; without integrations, you're just creating a folder for a label.

The cost of a lens is low: a directory and a row. The cost of *not* creating a lens when you should is a polluted global KB.

## Related

- [Lens CLI reference](../reference/lens-cli.md) — slug rules, lifecycle.
- [Lens tools reference](../reference/lens-tools.md) — `signal_*`, `lens_*`.
- [Palaces explanation](./palaces.md) — what lives in the lens KB.
- [Identity by lens](./identity-by-lens.md) — `identity_profile`.
- [Memory design](./memory-design.md) — global vs. lens tiers.
