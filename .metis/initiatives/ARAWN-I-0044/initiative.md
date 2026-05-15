---
id: openhuman-comparative-adoption
level: initiative
title: "OpenHuman comparative adoption — lift the small wins, decide on the big ones"
short_code: "ARAWN-I-0044"
created_at: 2026-05-15T14:00:16.710286+00:00
updated_at: 2026-05-15T21:22:52.699167+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


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

3. **Tier 3 (architectural)** — **spun out to `ARAWN-I-0048`** so this
   initiative can close once tier-1 + tier-2-late are tracked. The
   tier-3 questions are slow, deliberate architectural decisions;
   keeping them inside a shipped initiative would obscure both. The
   spec `ARAWN-S-0004` remains the working document; I-0048 tracks
   progress on it.

4. **Tier 4 (non-adoption)** — codified in `ARAWN-S-0004` §F–I as
   decisions, not tasks. They exist to prevent silent drift and are
   not in-scope for any active initiative.

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
3. Tier-3 drive-out happens on `ARAWN-I-0048` against `ARAWN-S-0004`
   §A–E.
4. Tier-4 items in `ARAWN-S-0004` §F–I are decided-non-goals; revisit
   only on a deliberate vision change.

## Exit Criteria

- ✅ All tier-1 tasks (T-0269, T-0271–T-0273, T-0275–T-0278) completed.
- Tier-2-late tasks (T-0270, T-0274) tracked as deferred with explicit
  telemetry-triggered revisit conditions on each task doc.
- Tier-3 drive-out spun out to `ARAWN-I-0048`; tier-4 locked in the
  spec.

## Outcome

Tier 1 of the openhuman comparative adoption shipped:

| Task | Subsystem | Commit |
|---|---|---|
| T-0269 | Tool wall-clock timeout (agent-overridable) | a62211a |
| T-0272 | Hint-style model routing taxonomy | b73800a |
| T-0271 | `arawn doctor` CLI + RPC | 58810a4 |
| T-0273 | Centralised prompt-injection guard | 88478ad |
| T-0275 | LLM resource gate (1-slot local cap) | 5b3d61e |
| T-0276 | Approval tiering (tool, shape) + audit log | 32df4ab |
| T-0277 | Token usage tracker (tokens only, no dollars) | 815742e |
| T-0278 | Routing policy (health-aware local/remote) | 19678d1 |

Workspace test count grew from ~583 to **1602 passed / 0 failed**.
Net +~1000 tests across the eight tasks. Documented deviations from
the openhuman comparative dive recorded on each task doc; major
ones: token tracker records tokens not dollars; routing module lives
in `arawn-llm` not `arawn-engine` (steward + extractor reuse).