---
id: lens-agnostic-chat-read-across-all
level: initiative
title: "Lens-agnostic chat — read across all lenses, file into one"
short_code: "ARAWN-I-0060"
created_at: 2026-05-27T02:34:56.579533+00:00
updated_at: 2026-05-27T02:34:56.579533+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: lens-agnostic-chat-read-across-all
---

# Lens-agnostic chat — read across all lenses, file into one

## Context

The `workstream → lens` rename (ARAWN-I-0059) changed the word, not the model:
a chat session is still **pinned to one active lens**, and `signal_search` /
`memory_search` only see that lens. But the lens *framing* implies something
different — a lens is a **refined dataset** (the product of `prompt + ontology`
over the corpus), and a chat should be able to **read across any/all lenses**
depending on the question, not sit inside one.

This initiative lands that model in ARAWN (single-user; the multi-user build is
AWEN's job). Decision: keep the storage model essentially as-is — the change is
to **decouple reads from a single active lens**, not to re-home the data.

## Goals & Non-Goals

**Goals:**
- Chat reads are **lens-agnostic**: `signal_search` / `memory_search` default to
  a union across **all lenses + global**, ranked together, with each hit
  labeled by its source lens. An optional `lens=` arg narrows to one.
- Remove the **active-lens read pin** (`SessionLens`/`LensMemoryRouter` on the
  read path).
- New learnings still **file into a target lens** (storage stays per-lens), so a
  "current lens" survives — reframed as the *write target only*, set by
  `/lens switch`.
- Chat uses the **default `assistant` persona**; persona stops flipping with the
  active lens.

**Non-Goals:**
- No storage teardown. Global `memory.db` + per-lens `lenses/<name>/memory.db`
  stay. Lenses are **not** collapsed into one corpus (that's the bigger AWEN
  shape, explicitly out of scope).
- No multi-user / shared-lens visibility (AWEN).
- No change to feeds, projections, extraction *shape* (extraction still writes
  into the target lens's KB with that lens's ontology).

## Decisions (locked with user 2026-05-27)

- **Land in ARAWN**, not AWEN.
- **Reads cross-lens, writes per-lens** ("read across all, file into one").
  Storage unchanged.
- **`scratch` stays** as the default write bucket, re-described as "where general
  / un-filed learnings land" — not "a place you chat in." (Possible later rename
  to `general`; avoided now to skip a second rename.)
- **Persona** defaults to `assistant` for chat; no per-active-lens persona flip.
  A lens may still carry `identity_profile`/`tags_ontology` used when *writing*
  into it.
- **`/promote`** reframed to "file this session's scratch learnings into lens X"
  (mostly existing logic).

## Detailed Design

Today: `Session.lens_id`/`lens_name` pin → `SessionLens` (`Arc<Mutex<String>>`)
→ `LensMemoryRouter.current()` opens *that* lens's `memory.db`; `signal_search`
/`memory_search` query only it. `feed_search` is already global — it's the model
to copy.

Target:
- **Read path**: a cross-lens query in `arawn-memory` enumerates the lens
  registry, opens each lens KB (lazy + cached handles), runs FTS + vector search
  per store, and RRF-merges results with a `lens` label per hit. `global` tier
  included. `signal_search`/`memory_search` call this by default; `lens=<name>`
  narrows to one store (today's behavior, opt-in).
- **Write path**: `SessionLens` survives but only governs **writes** — the
  extractor and memory writes file into the target lens's KB. `/lens switch`
  sets the target. Reads no longer consult it.
- **Persona**: system-prompt builder defaults to `assistant`; drop the
  per-active-lens persona injection from the read/session path.

### Hard point
The one genuinely new capability is the **cross-lens merged query** (open N lens
stores, search each, fuse). Everything else is *unwiring* the read pin. Watch
perf: open lens handles lazily and cache them; a user has a handful of lenses,
not thousands.

## Alternatives Considered

- **Single corpus + lenses-as-views (no per-lens stores).** The "truest" view
  model: one knowledge store, lenses are saved `prompt+ontology` filters applied
  at read time; writes go to the corpus tagged by ontology. Cleaner long-term,
  but a storage teardown. Rejected for ARAWN per user ("storage doesn't need to
  change much"); it's the AWEN shape.
- **Keep the active-lens pin, add an explicit "search all" flag.** Smaller, but
  keeps the container mental model as the default — the opposite of the goal
  (chat should roam by default, narrow on request). Rejected.
- **Target-less / auto-classified writes.** Agent routes each new entity to the
  best-fit lens by ontology. More magic, more failure modes; per-lens storage
  still needs *a* destination. Deferred — keep an explicit write target.

## Implementation Plan

- **T-A — Cross-lens merged read in `arawn-memory`.** Enumerate + lazily open all
  lens KBs; FTS + vector search per store; RRF-merge with per-hit lens label.
  Unit-tested against ≥2 seeded lens stores.
- **T-B — Retarget search tools.** `signal_search` / `memory_search` default to
  the cross-lens read; optional `lens=` narrows; drop the active-lens read
  dependency. Hits show their source lens.
- **T-C — Demote the active-lens shim to write-target.** `SessionLens` /
  `LensMemoryRouter` stop governing reads; writes (extractor + memory) use it as
  the file-into target. `/lens switch` reframed.
- **T-D — Decouple persona.** Default `assistant` for chat; system-prompt builder
  no longer swaps persona per active lens.
- **T-E — CLI / tools / docs reframe.** `/lens switch`, `lens_switch` /
  `lens_promote` descriptions, `scratch`'s meaning, `lenses.md` / `lens-tools.md`
  rewritten to the roam model.
- **T-F — TUI.** Sidebar reflects "filing into X" rather than "scoped to X";
  surface cross-lens hit provenance where search results render.

## Open Questions
- Ranking/fusion across stores with different sizes/embedders — does a global RRF
  need per-lens normalization? Resolve in T-A with the seeded multi-lens test.
- Does `memory_search` `scope` (global vs lens) still make sense, or fold into the
  `lens=` selector? Decide in T-B.
