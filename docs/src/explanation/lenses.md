# Lenses

*Explanation. What a lens is, how memory / signals / lenses relate, when to create one, and what scratch is for.*

A **lens** is a *standing, memory-aware extractor over your corpus*. You define
one around an ongoing concern (work, home, a project, the people you manage)
with its own prompt, tag ontology, and (optionally) the feeds it's bound to.
Feeds flow into the system, and each lens's extractor continuously decides which
incoming items are in its scope and what typed entities — its **signals** — they
contribute to that lens's KB.

You don't switch into a lens. There's no "active" lens for the chat. Lenses just
*run*.

## Memory vs. signals vs. lenses

The three concepts mean different things and stay separate:

- **Memory** is global. *Statements of fact and behavioral tuning that should be
  generally known* — "Pat Collins is someone I manage", "prefer terse
  responses", "we use Postgres 16 in production". Stored in `<data_dir>/memory.db`,
  available to the chat and to every lens's extractor. `/remember` writes
  memories; `/memory` views them; `memory_search` recalls them. **Memory has no
  lens.**
- **Signals** are extracted from your **corpus** (feeds) by each lens. They're
  the typed entities — decisions, facts, notes, events, mentions — that
  accumulate as feeds bring in new material. They live in the lens that
  extracted them (`<data_dir>/lenses/<slug>/memory.db`). The agent reads them
  with `signal_search` / `signal_query` / `signal_timeline` across *every* lens
  at once; each hit is labeled with its source lens.
- **A lens** is the standing extractor that produces those signals. It carries a
  prompt + ontology that say "here's what's in scope, and these are the tags we
  organize it under." It **reads from memory** when it decides scope ("Pat
  Collins is a direct report, so this 1:1 transcript is in scope for the people-
  I-manage lens"). It does not own memory.

Concretely: lenses pull (signals) from memory's standing facts; they don't store
memory of their own.

## What a lens actually is

A directory plus a database row.

```
<data_dir>/lenses/<slug>/
  ├── memory.db         # this lens's signal store (the palace)
  └── workspace/        # FS-isolated working directory for shell + file tools
```

Plus a row in the global `lenses` table carrying:

- `name` (slug), `display_name`, `description`
- `bindings[]` (feeds and/or URI schemes attached to this lens)
- `tags_ontology[]` (the closed list the extractor may use)
- `archived` flag

The directory is where the extractor builds the lens's signal palace. The DB row
is the configuration that drives extraction.

## Why lenses exist

### 1. Per-topic vocabulary + organized signals

Each lens has its own tag ontology — typically 5–12 slugs the extractor uses to
tag entities. A "work" lens's ontology (`postgres`, `ledger`, `migration`,
`hiring`) is its own; a "home" lens's is different. One global ontology can't
reasonably span both, and filing each new entity under the lens whose ontology
fits keeps the signal stream organized at extraction time.

The ontology is declared at lens creation (the `lens_propose_ontology` step in
the create flow) and grows via the `tag-promoter` steward subroutine. See
[palaces](./palaces.md).

### 2. Provenance without partitioning

A single undifferentiated signal stream mixes everything — work decisions, home
maintenance, hobbies. Lenses keep extracted knowledge *attributed*: a hit
always tells you which lens it belongs to, so "we decided Postgres 16" reads as
a *work* decision, not a free-floating fact. Attribution is not isolation — a
question still searches across all lenses, so you never have to remember which
one a fact lives in.

### 3. Memory-aware scope decisions

A lens's extractor reads from global memory when classifying. "Pat Collins is
someone I manage" lives once, globally, and every lens that cares about that
fact (the people-I-manage lens, the team-health lens, the 1:1-prep lens) draws
on it when deciding scope. You write the fact once; the lenses pick it up.

## When to create a lens

| Create a lens when... | Don't, when... |
|---|---|
| You'll come back to this topic over weeks or months | One-off question; use scratch |
| Multiple feeds make sense for it | You're not connecting integrations to it |
| You want a tag ontology for it | You don't have enough recurring concepts to define 5+ tags |
| You want the steward maintaining a signal palace about it | You won't query the palace |

A lens without a binding is harmless — extraction has nothing to do. A lens with
a binding but no queries piles up data the agent never reads. **Bind lenses to
the topics you actually ask about.**

## Scratch: the default chat sandbox

The `scratch` lens is auto-created on first boot and undeletable. It's the
default session context — somewhere to run a conversation that isn't anchored
to any topic-specific extractor. Reads still roam every lens from a scratch
session; scratch just doesn't run an extractor of its own.

You don't have to create a lens to use arawn. Scratch is fine for tinkering and
one-off chats. Create lenses when you want their continuous extraction over the
topics you care about.

## Lens vs. session

A **session** is one chat conversation — a sequence of turns with the LLM.
Sessions are not "in" a lens; they read across every lens's signal stream by
default and draw on global memory. There is no `/lens switch`.

## Lens vs. feed

A feed mirrors upstream content (Gmail, Slack, filesystem) into a queryable
**corpus**. A lens is the *interpretation* of that corpus — the typed entities
its extractor builds and the queries you run against them.

The relationship is many-to-many:

- One lens can bind many feeds (a `work` lens might bind Gmail + Slack + Jira).
- One feed can bind to many lenses (a `gmail/inbox` could feed both `work` and
  `personal` lenses; both extractors run on each new message).

[Bind a lens to a feed](../how-to/bind-a-lens-to-a-feed.md) is the recipe.

## Lens vs. memory

Memory is **global** and unrelated to lens membership.

- `/remember` writes memory: a global statement (`"Pat Collins is someone I
  manage"`, `"prefer terse responses"`).
- `/memory` views the global memory store.
- `memory_search` recalls memory by content.
- A lens's extractor *reads* from global memory while classifying signals, so
  facts you remember once influence every relevant lens's extraction.

Lenses do **not** have their own memory tier — what looks like memory in a lens
is *signals*: extracted activity, recallable with `signal_*`.

See [memory design](./memory-design.md) for the rationale.

## Filesystem isolation

The `lenses/<slug>/workspace/` directory is the writable root for shell + file
tools when an action is associated with a lens. Writes outside `workspace/` fail
with `Permission denied` at the sandbox layer.

This composes with the [shell sandbox](../reference/shell-sandbox.md) to bound
what autonomously-running tools (watchers, chat-driven actions) can touch on
disk — a safety prerequisite, not a later enhancement. (Filesystem isolation is
a *write*-side boundary; signal *reads* are cross-lens.)

## When NOT to use lenses

- **You're tinkering.** Scratch is fine for tinkering.
- **You have one lens and won't add more.** That's fine too — the abstraction
  doesn't cost anything when you don't use it.
- **You're not connecting integrations.** Lenses' value comes from extraction
  over feed-supplied material; without feeds, you're just creating a folder for
  a label.

The cost of a lens is low: a directory and a row. The cost of *not* creating a
lens when you should is a signal stream you can't navigate.

## Related

- [Lens CLI reference](../reference/lens-cli.md) — slug rules, lifecycle.
- [Lens tools reference](../reference/lens-tools.md) — `signal_*`, `lens_*`.
- [Palaces explanation](./palaces.md) — what lives in a lens's signal store.
- [Memory design](./memory-design.md) — why memory is global.
