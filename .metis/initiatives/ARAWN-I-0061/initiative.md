---
id: correct-the-memory-signal-lens
level: initiative
title: "Correct the memory / signal / lens model in code and UX"
short_code: "ARAWN-I-0061"
created_at: 2026-05-27T19:02:19.758458+00:00
updated_at: 2026-06-14T15:53:01.664503+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: correct-the-memory-signal-lens
---

# Correct the memory / signal / lens model in code and UX

## Context

A whole-product UX review after merging ARAWN-I-0060 surfaced inconsistencies in
how the lens model reaches the user. Probing those with the user (2026-05-27)
revealed the issue is deeper than copy: the **conceptual model itself is
conflated in the code**, and I-0060 made part of it worse.

**The correct model (locked with user):**
- **Memory** — global statements of fact / preferences / behavioral tuning
  (e.g. *"Pat Collins is someone I manage"*). Always known to the agent; shapes
  behavior. **Global. Not lens-filed. Not extracted into signals.** `/remember`
  stores; `/memory` views. Generally available to everything — chat *and* lenses
  read from it.
- **Signals** — activity/events extracted from the corpus (feeds). The
  time-stamped stream of what's happening. `signal_search` / `signal_query` /
  `signal_timeline`.
- **Lens** — authored independently (own prompt + ontology), tuned over time, and
  **reads from memory** to inform its behavior/preferences. A standing,
  memory-aware extractor that continuously pulls **signals** from the corpus
  around its focus. NOT a place you switch into; NOT derived from memory.

The continuous per-lens extraction already exists for feeds
(`crates/arawn-extractor/src/cot.rs`: in-scope classify + ontology extract,
`run_for_all_lenses`). The conflations are on the memory path and the UX.

## Goals & Non-Goals

**Goals:**
- Make **memory global**: `/remember` / `memory_store` write to the global store,
  not "the active lens."
- **Remove the write-target / `/lens switch` mechanic** entirely — nothing is
  hand-filed or switched into. `/lens` becomes create / list / show.
- **Un-smear memory vs signals** (corrects I-0060): `memory_search` → global
  memory facts only; `signal_*` own the cross-lens extracted stream.
- Ensure **lenses read from memory** (memory generally available to the extractor
  / lens behavior).
- Make **signal provenance visible in chat** (which lens(es) a cross-lens signal
  answer drew from) via a sources footer chip.
- Clean up the **doc/description drift** and **onboarding/feedback dead-ends**
  found in the review, reframed to the corrected model.

**Non-Goals:**
- No change to the feed → projection → extraction pipeline (it already embodies
  the model).
- No multi-user / shared lenses (AWEN).
- No new lens-authoring UX beyond what's needed to drop switching.

## Relationship to I-0060 (partial correction)

I-0060 shipped "reads roam across all lenses, writes file into one write-target
lens, persona = assistant." This initiative **keeps** the cross-lens *signal*
read and the assistant persona, but **corrects** two things it got wrong under
the locked model:
1. `memory_search` was made cross-lens — that smeared global **memory** into
   lens **signals**. Revert `memory_search` to global-only.
2. The **write-target lens + `/lens switch`** mechanic should not exist — memory
   is global, signals are extracted, lenses are standing. Remove it (and the
   status-bar `✎` write-target indicator).

## Findings (2026-05-27 UX review) mapped to the corrected model

- **Memory mis-routed** — `/remember` files into the active lens
  (`memory_store.rs:134`); must be global.
- **Write-target / switch shouldn't exist** — `/lens switch`
  (`event_loop/mod.rs:511-531`, also wipes chat), status-bar `✎`
  (`render/status_bar.rs:36-45`), SessionLens write-target plumbing.
- **memory_search smear** — `memory_search` reads all lenses + global
  (`memory_search.rs:147-185`); revert to global memory only.
- **Signal descriptions still lie** — `signal_query` (`signal.rs:256`),
  `signal_timeline` (`signal.rs:424`), module doc (`signal.rs:1-9`) say "active
  lens" though they roam. (Correct framing now: signals are cross-lens, labeled
  by source lens.)
- **Signal provenance invisible** — labels live only in collapsed/raw tool cards
  (`render/chat.rs:96-141`). Add a "sources: <lens>…" footer chip on assistant
  turns that consumed `signal_*`. *(Q1 locked: footer chip.)*
- **Doc drift** — `memory-design.md:22`, `the-agent-loop.md:14-15`,
  `lens-tools.md:5`, `agent-tools.md:75,99`, `identity-by-lens.md` body,
  `first-lens.md:36`, plus `lenses.md` (rewrite again to memory/signal/lens, not
  "reads-all/writes-one").
- **`/day` phantom** — dashboard says "run /day" (`render/dashboard.rs:54`); no
  such command. Use `/today` / `/brief`.
- **Silent "no model"** — `render/status_bar.rs:20`; no guidance toward
  `arawn doctor` / config.
- **Feed-run results invisible** — after `/feeds run`, row/entity counts only in
  server log (D1).
- **`/brief` undocumented** (D2); **`.metis/vision.md` stale** workstream/watcher
  vocabulary (D3).

## Resolved Decisions

- **Q1 — Signal provenance UI:** sources **footer chip** on assistant turns
  (`◆ sources: work · personal`).
- **Q2 — `/lens switch` / write-target:** **remove**. Lenses aren't switched into.
- **Q3 — `/memory`:** memory is the **global** fact store; `/memory` views it;
  `/remember` stores it. Un-conflate from signal search.

## Alternatives Considered

- **Docs-only fix.** Rejected — the conflation is in routing and tool behavior,
  not just copy.
- **Keep write-target as an escape hatch.** Rejected — reintroduces the container
  feel the user explicitly rejected; memory-global + extraction covers the need.
- **Leave `memory_search` cross-lens.** Rejected — it conflates global memory
  with lens signals, the exact distinction this initiative restores.

## Implementation Plan (proposed — pending approval to decompose)

- **T-A — Memory goes global.** Route `/remember` / `memory_store` to the global
  store; stop consulting the active lens for memory writes.
- **T-B — Remove write-target + `/lens switch`.** Drop the switch subcommand,
  status-bar `✎` write-target, and SessionLens-as-write-target plumbing; `/lens`
  → create/list/show. Ensure writes default to global memory.
- **T-C — Un-smear `memory_search`.** Revert to global-memory-only; signals stay
  cross-lens via `signal_*`.
- **T-D — Lenses read memory.** Confirm/wire the extractor + lens behavior to read
  global memory (in-scope/ontology decisions can use known facts).
- **T-E — Signal provenance footer chip** in chat (Q1) + unify signal/memory
  result rendering.
- **T-F — Truth-up descriptions + docs** to the memory/signal/lens model
  (`signal_*` descriptions, `lenses.md`, `memory-design.md`, `the-agent-loop.md`,
  `lens-tools.md`, `agent-tools.md`, `identity-by-lens.md`, `first-lens.md`).
- **T-G — `/day` phantom + `/brief` docs** (B1/D2).
- **T-H — First-run "no model" guidance** (B2).
- **T-I — Feed-run feedback in TUI (D1) + vision refresh (D3).**

## Status Updates

**2026-05-27 — Created, then reframed in discovery.** Initial framing ("make
I-0060's reads-all/writes-one visible") was corrected by the user into the
memory/signal/lens model above. This initiative now partially *corrects* I-0060
(memory_search smear, write-target/switch). Decisions Q1–Q3 locked. Decomposition
proposed (T-A…T-I); awaiting approval before creating task docs.