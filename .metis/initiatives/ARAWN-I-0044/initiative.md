---
id: openhuman-comparative-adoption
level: initiative
title: "OpenHuman comparative adoption — lift the small wins, decide on the big ones"
short_code: "ARAWN-I-0044"
created_at: 2026-05-15T14:00:16.710286+00:00
updated_at: 2026-05-15T14:00:16.710286+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: openhuman-comparative-adoption
---

# OpenHuman comparative adoption Initiative

## Context **[REQUIRED]**

OpenHuman (`tinyhumansai/openhuman`, GPL, Rust core ~50 domain modules) is a
similar-shaped self-hosted personal AI harness that has been under active
development longer than arawn. A walk through their `src/openhuman/*` tree
surfaced a list of subsystems they have built that we either lack or have built
more narrowly. They are *broader* (channels, 118-integration Composio proxy,
desktop mascot, voice, screen capture) — we are intentionally *more focused*
(workstreams, steward with bounded blast radius, plugin system, Cypher-graph
memory). Both projects converge on the same shape for most of the supporting
infrastructure (config, routing, cost, prompts, scheduling), and that supporting
infrastructure is where this initiative looks for adoption opportunities.

The original comparative dive is preserved in the conversation that spawned
this initiative; the working list of candidates is captured in this initiative
plus `ARAWN-S-0004` (the tier-3/4 spec) and the tier-1/2 tasks attached below.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- Adopt the high-leverage subsystems they have built and we lack — eight
  tasks promoted to a single active tier 1: tool timeout, hint routing
  taxonomy, doctor CLI, prompt-injection guard, LLM resource gate,
  approval tiering, token usage tracker, routing policy.
- Defer token-efficiency work (redirect-link shortener, TokenJuice
  compaction) as tier 2-late: revisit only when the token usage tracker
  surfaces a real signal. Token cost is not measurable pain today.
- Drive out a discovery decision on the five tier-3 architectural questions
  (per-domain config refactor, typed event bus, tree summarizer placement,
  learning candidate/producer split, triage drop tier) via the companion
  specification document.
- Capture our deliberate non-adoption posture on the four tier-4 items
  (Composio backend proxy, channel sprawl, agentmemory shared backend,
  subconscious heartbeat) so future contributors do not relitigate them.

**Non-Goals:**
- Wholesale port of openhuman. Most of their surface (desktop mascot, voice
  loops, Google Meet agent, 14 messaging channels) is explicit non-goal for
  arawn — the focus differential is the point.
- Adopting their *memory model*. Our workstream + steward + Cypher-graph
  story is the differentiator. We may borrow scoring or canonicalisation
  ideas later, but this initiative does not put memory architecture on the
  table.
- Adopting their *integration strategy* (backend-proxied 1000+ Composio
  toolkits). We stay direct-OAuth per provider. This is reaffirmed in the
  tier-4 spec.

## Detailed Design **[REQUIRED]**

### Phasing

1. **Tier 1 — active work, in delivery order:**
   1. `T-0269` — tool wall-clock timeout (agent-overridable per call).
   2. `T-0272` — hint-style model routing taxonomy.
   3. `T-0271` — `arawn doctor` CLI + RPC.
   4. `T-0273` — centralized prompt-injection guard.
   5. `T-0275` — LLM resource gate.
   6. `T-0276` — approval tiering.
   7. `T-0277` — token usage tracker.
   8. `T-0278` — routing policy.

   The order reflects delivery preference, not strict dependencies. The
   only hard dependency chain is `T-0272` + `T-0277` → `T-0278`.
   Everything else in tier 1 can ship in any order or in parallel.

2. **Tier 2-late (deferred, telemetry-triggered)** — `T-0270` redirect-link
   shortener and `T-0274` TokenJuice compaction. Both are pure
   token-efficiency work. Token usage has not been measurable pain so far;
   revisit only when the token usage tracker (`T-0277`) shows a specific
   tool or inbound boundary burning measurable tokens. Kept on the
   initiative so the design work is not lost.

3. **Tier 3 (architectural)** — *not decomposed yet*. The companion
   specification `ARAWN-S-0004` enumerates the open questions; this
   initiative blocks on resolving those before tier-3 tasks land. The
   spec is the discovery vehicle.

4. **Tier 4 (non-adoption)** — codified in the spec as decisions, not
   tasks. They exist to prevent silent drift.

### Source material

- Cloned reference: `/tmp/openhuman` at upstream commit `e7c2eb7c`
  (fix(tauri): disable GPU on Linux for Mesa 26+ EGL compatibility).
- Key directories scanned: `src/openhuman/{config,routing,tokenjuice,
  scheduler_gate,prompt_injection,approval,cron,cost,doctor,redirect_links,
  tool_timeout,learning,subconscious,tree_summarizer,agent/triage,
  composio,channels}` and `src/core/event_bus/`.

## Alternatives Considered **[REQUIRED]**

- **Do nothing, watch from a distance.** Rejected: several of these
  subsystems are blocking pain we already have (cargo output bloat, no
  global tool timeout, no cost ceiling).
- **Fork openhuman and strip.** Rejected: their architectural choices
  (backend-proxied integrations, agentmemory backend, desktop mascot)
  are exactly the focus-loss we wrote the arawn vision to avoid.
- **Adopt one big-bang refactor.** Rejected: the tier-1 wins are
  independent and ship-now; bundling them with the config refactor (a
  tier-3 question) would block easy work on hard decisions.

## Implementation Plan **[REQUIRED]**

1. Land tier-1 tasks (this initiative, immediately). Each is a thin
   vertical slice with tests; merge order does not matter.
2. Land tier-2 tasks. Most are independent; only `T-0278` (routing
   policy) is sequenced — it depends on `T-0272` and `T-0277`. No ADRs
   required for these tasks; each task document is self-contained.
3. Resolve the tier-3 spec (`ARAWN-S-0004`) — read, comment, decide
   per question (A–E). Each `Decide` outcome either generates a new
   task on this initiative or is closed as deferred.
4. Tier-4 items: archive into the spec as decided-non-goals; revisit
   only on a deliberate vision change.

## Exit Criteria

- All tier-1 and tier-2 tasks completed or explicitly deferred with
  rationale.
- Tier-3 spec moved to `published` (every question has a recorded
  decision, even if the decision is "defer").
- Tier-4 spec section locked — no open questions left.
