# Curate a lens

*How-to. Review the steward's proposals — tag suggestions, new relations, dust summaries — and accept the ones you want.*

The **steward** is the maintenance loop for a lens's knowledge graph. Periodically it looks at what the extractor has been writing and proposes changes: a new tag for entities the extractor labeled with a low-confidence catch-all, a relation between two entities it sees co-occurring, a "dust" summary that compresses cold material.

Steward proposals are *journaled* — every one has a unique id and can be rolled back. Nothing changes the palace until you apply.

## Prerequisites

- A lens with at least one bound feed that's been running long enough for the steward to have something to say (usually a day or two of activity).

## 1. See what's pending

```
lens_refine
```

Returns a list of pending proposals. Each row has:

- A **journal id** — used to apply or rollback.
- A **subroutine name** — `tag-promoter`, `relation-suggester`, `dust-summarizer`, etc.
- A **summary** of what the proposal does.
- A **preview** of the entities it touches.

You can call this tool directly or just ask the agent: *"what's the steward suggesting?"*

## 2. Apply a proposal

```
lens_apply <journal_id>
```

The proposal lands in the palace. The journal records the change with full before/after state.

## 3. Rollback if you change your mind

```
lens_rollback <journal_id>
```

Reverses the apply. The journal records the rollback as another entry; the original proposal is still there (in case you want to re-apply later).

## Common steward subroutines

### `tag-promoter`

The extractor writes a fallback tag (often `other` or a low-confidence guess) when nothing in the ontology fits well. After enough hits, the steward proposes adding a new explicit tag to the ontology and re-tagging the affected entities.

Apply this when you see the tag suggested matches a real recurring theme.

### `relation-suggester`

When two entities co-occur in the same projection rows above a threshold, the steward proposes a relation linking them. Common case: a decision and the meeting where it was made.

Apply this when you'll want to query across the relation (e.g. "what was decided in the platform sync?").

### `dust-summarizer`

When entities older than a threshold haven't been read in a long time, the steward proposes a compact summary entity that points back to them, then archives the originals.

Apply this when you don't need full detail on the cold tail but want to keep the high-level take.

## Reviewing without applying

```
lens_journal
```

Shows the journal — every change ever applied or rolled back, in order. Useful for "what did I accept last week?"

## What's next

- See the full list of steward subroutines: [steward subroutines reference](../reference/steward-subroutines.md).
- Understand why steward is bounded + journaled: [explanation: steward](../explanation/steward.md).
- Bind more feeds to grow the palace: [bind a lens to a feed](./bind-a-lens-to-a-feed.md).
