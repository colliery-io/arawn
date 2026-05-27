# Lenses

*Explanation. What a lens is, when to create one, how it relates to sessions/feeds/palaces/identity, and what scratch is for.*

A **lens** is a *refined view of your knowledge* — one ongoing concern (work, home, a project), defined by its own tag ontology and its own curated knowledge base. Feeds flow in; the extractor files typed entities under the relevant lens.

The key thing about a lens is how a chat uses it: **reads roam, writes file into one.**

- **Reading is lens-agnostic.** When you ask a question, search (`signal_search`, `memory_search`, …) spans **every lens** and the global tier at once, ranks the hits together, and labels each with the lens it came from. You don't "enter" a lens to see its knowledge — the chat sees across all of them and narrows only if you ask (`lens=<name>`).
- **Writing goes to one lens.** New learnings (memory writes, extraction) file into the session's **write-target** lens — the one set by `/lens switch`. That's the only thing "switching" a lens changes: where new knowledge lands, not what you can recall.

So a lens is a *destination for knowledge and a vocabulary for it*, not a box you sit inside. (The multi-user, single-corpus version of this model is the separate **AWEN** project; in ARAWN each lens is still backed by its own on-disk KB.)

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

Two problems they solve:

### 1. Per-topic vocabulary + organized knowledge

Each lens has its own tag ontology — 5-12 slugs the extractor uses to tag entities. A "work" lens's ontology (`postgres`, `ledger`, `migration`, `hiring`) is its own; a "home" lens's is different. One global ontology can't reasonably span both, and filing each new entity under the lens whose ontology fits keeps the knowledge organized at write time.

The ontology is declared at lens creation (the `lens_propose_ontology` step in the create flow) and grows via the `tag-promoter` steward subroutine. See [palaces explanation](./palaces.md).

### 2. Provenance without partitioning

A single undifferentiated knowledge base mixes everything — work decisions, home maintenance, hobbies. Lenses keep that knowledge *attributed*: a hit always tells you which lens it belongs to, so "we decided Postgres 16" reads as a *work* decision, not a free-floating fact. But attribution is not isolation — a question still searches across all lenses, so you never have to remember which one a fact lives in. You get focus (each lens's own vocabulary + a labeled source) without walling knowledge off from a chat that needs it.

> **Not identity routing.** Earlier, the active lens also picked the agent persona. It no longer does — a lens-agnostic chat has no single lens to take a persona from, so the chat always uses the default `assistant` persona. A lens's `identity_profile` column still exists (and matters for AWEN), but it doesn't drive the chat system prompt. See [identity by lens](./identity-by-lens.md).

## When to create a lens

| Create a lens when... | Don't, when... |
|---|---|
| You'll come back to this topic over weeks or months | One-off question; use scratch |
| Multiple feeds make sense for it | You're not connecting integrations to it |
| You want a tag ontology for it | You don't have enough recurring concepts to define 5+ tags |
| You want the steward maintaining a KB about it | You won't query the KB |

A lens without a binding is harmless — extraction has nothing to do. A lens with a binding but no queries piles up data the agent never reads. **Bind lenses to the topics you actually ask about.**

## Scratch: the default write-target

The `scratch` lens is auto-created on first boot and undeletable. It's the **default write-target** — where new learnings file until you've pointed the session at a named lens. Not a place you're confined to: reads still roam every lens from a scratch session just the same.

- The agent works fine in scratch. You don't need to create a lens to use arawn.
- Memory entries written while scratch is the target land in scratch's KB.
- To consolidate ad-hoc notes once you know where they belong, **promote** them: `/promote <name>` or the `lens_promote` tool files scratch's entities into a named lens (existing duplicates reinforce).

## Lens vs. session

A **session** is one chat conversation — a sequence of turns with the LLM. A session carries a **write-target** lens (defaulting to `scratch`) that says where its new learnings file. It does *not* scope what the session can read — every session reads across all lenses.

`/lens switch` changes the write-target for the rest of the session; you can do it mid-conversation, and it only redirects future writes.

## Lens vs. feed

A feed mirrors upstream content to disk. A lens is the *interpretation* of feeds — the typed entities the extractor builds, the queries you run against them.

The relationship is many-to-many but per-feed-config:

- One lens can bind many feeds (your `work` lens might bind Gmail + Slack + Jira).
- One feed can bind to many lenses (your `gmail/inbox-archive` could feed both `work` and `personal` lenses; both extractors run on each new email).

[Bind a lens to a feed](../how-to/bind-a-lens-to-a-feed.md) is the recipe.

## Lens vs. memory

Memory has two tiers: **global** (`<data_dir>/memory.db`) and **lens** (`<data_dir>/lenses/<name>/memory.db`).

- Preferences and Person entities are written to global.
- Decisions, conventions, facts, notes are written to the write-target lens.

This is about *where things are stored and attributed*, not what's visible: a read spans both tiers and every lens. Your `prefer Tokio over async-std` preference lives in global (it's not tied to a topic); the `we use Postgres 16` decision lives in — and is labeled as belonging to — the lens you filed it under, even though a search from any session will still surface it.

[Memory design](./memory-design.md) covers the rationale.

## Filesystem isolation

The `lenses/<name>/workspace/` directory is the writable root for shell + file tools, scoped to the session's current write-target lens. Writes outside `workspace/` fail with `Permission denied` at the sandbox layer.

This composes with the [shell sandbox](../reference/shell-sandbox.md) to bound what autonomously-running tools (watchers, chat-driven actions) can touch on disk — a safety prerequisite, not a later enhancement. (Note this is a *write*-side boundary on the filesystem; knowledge *reads* are cross-lens.)

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
