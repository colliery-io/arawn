---
id: openhuman-comparative-adoption
level: specification
title: "OpenHuman comparative adoption — tier 3 architectural questions + tier 4 non-adoption decisions"
short_code: "ARAWN-S-0004"
created_at: 2026-05-15T14:03:58.720422+00:00
updated_at: 2026-05-15T14:03:58.720422+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# OpenHuman comparative adoption — tier 3 architectural questions + tier 4 non-adoption decisions

> **Status:** draft. Tier-3 drive-out is tracked on **ARAWN-I-0048**.
> Each §A–E question is open and needs an explicit *Decide* outcome
> (adopt now / adopt later / defer indefinitely / reject). Tier-4 items
> are documented as decisions, not questions — they exist so future
> contributors do not relitigate them silently.
>
> Originally parented to ARAWN-I-0044; that initiative shipped tier 1
> and closed. The spec remains the working document for §A–E decisions.

## Overview **[REQUIRED]**

OpenHuman (`tinyhumansai/openhuman`) ships a number of large-grain architectural
choices that arawn either lacks today or has implemented differently. The
parent initiative (ARAWN-I-0044) handled the small + medium wins as tasks.
This spec captures the choices that are not just "do the work" — they require
a directional decision about what kind of system arawn wants to be.

Two buckets:

- **Tier 3 — open architectural questions.** Each section ends with a
  *Decide* block. The decision unblocks (or kills) follow-on tasks under
  ARAWN-I-0044.
- **Tier 4 — deliberate non-adoption.** Decisions already taken in
  conversation; recorded here so they have a citable home.

## Tier 3 — open architectural questions

### A. Per-domain typed `Config` refactor

**What openhuman does.** A single root `Config` struct hangs ~50 typed
sub-configs (`AgentConfig`, `MemoryConfig`, `RoutingConfig`,
`SchedulerConfig`, …) with per-section RPC update endpoints
(`openhuman.config.update_model_settings`, …) and a `settings` CLI surface.
~177 internal consumers read `Config` directly. Env-variable overrides and
encrypted-secrets sections are first-class. See
`/tmp/openhuman/src/openhuman/config/README.md`.

**What arawn does today.** Flatter TOML (`~/.arawn/arawn.toml`) hand-parsed
into a handful of structs. Adding a new subsystem means adding a new section
ad-hoc — no consistent override / RPC / encrypted-secret story.

**Why it matters now.** Several tier-1 and tier-2 adoptions
(routing-policy, scheduler-gate, cost-tracker, tool-timeout, approval) each
need their own config slice. We can either (a) eat the refactor up front so
they all land on the new shape, or (b) keep adding flat sections and refactor
later.

**Tradeoff.** Up-front refactor is ~2 weeks but every subsequent adoption
becomes cheaper. Deferring means each adoption pays a small tax and we
eventually pay the big tax anyway, probably with migrations.

**Decide:** {adopt now / adopt after tier-1+2 / defer indefinitely}.

---

### B. Typed cross-module event bus

**What openhuman does.** Process-wide singleton (`src/core/event_bus/`) with
a typed `DomainEvent` enum (Channel(*), Cron(*), Memory(*), Skill(*), …).
Modules `publish_global` and `subscribe_global`; the bus also supports
typed native request/response. Decouples cron → notifications, memory →
agent, channel → triage, etc.

**What arawn does today.** Direct calls + a hook system in
`arawn-engine/src/hooks/`. No pub/sub. Subsystem-to-subsystem wiring is
explicit and grows quadratic-ish as we add cross-cutting concerns.

**Why it matters now.** The tier-2 work introduces several genuine
cross-cutting consumers: routing telemetry, cost-tracker events, prompt-
injection verdicts, scheduler-gate signals, steward-journal writes. Each
either gets ad-hoc plumbing or rides a bus.

**Tradeoff.** Bus is ~1 week and a process-wide singleton (which we mostly
avoid). But the alternative is N×M direct wiring that gets visibly worse
as N and M grow. The forcing function is "do we have ≥3 concrete
cross-cutting consumers lined up?" — current count is 4–5.

**Decide:** {adopt now / wait until ≥5 consumers / defer / reject — keep
direct-call model}.

---

### C. Tree summarizer placement (year→month→day→hour markdown tree)

**What openhuman does.** `src/openhuman/tree_summarizer/` drains a buffer
each hour, summarizes it into the hour leaf, propagates summaries upward.
Stored as markdown files under `memory/namespaces/{ns}/tree/`. Distinct
from their bucket-seal `memory/tree/` retrieval architecture.

**What arawn does today.** Nothing equivalent. I-0041 (daily prep) and
I-0042 (weekly prep) and I-0043 (weekly retro) all want hierarchical
time-based summaries; today each would build its own one-off summary path.

**Why it matters now.** I-0041 is *next up* — if we build the summary
path inside I-0041, I-0042 and I-0043 will either duplicate it or
refactor it. Doing it once, generically, before I-0041 saves both.

**Tradeoff.** Building the summarizer first delays I-0041's first
visible output. Building it inside I-0041 ships sooner but creates
work in I-0042 / I-0043.

**Decide:** {fold tree summarizer into I-0041 as foundational task /
make it its own initiative blocking I-0041 / build per-ceremony, refactor
later}.

---

### D. Learning candidate/producer split

**What openhuman does.** `src/openhuman/learning/` runs a post-turn
pipeline: producer modules (signature parser, edit-window heuristic,
correction-repeat detector, length-ratio detector) write
`LearningCandidate` entries to a thread-safe ring buffer; a downstream
stability detector decides which signals are durable enough to commit
to memory. Cleanly separates "evidence collection" from "what to keep."

**What arawn does today.** `arawn-steward` maintains the *files* the user
already wrote (reshelve / dust / map / doorwatch). It does not extract
preferences from how the user *behaves* inside turns. The two concerns
overlap (both write to memory) but they're different axes.

**Why it matters now.** The vision speaks to a system that "becomes you
over time." Steward is half of that (curating what's there). The
producer/consumer learning loop is the other half (noticing patterns
from interaction). Without an explicit decision, future memory work
will smear across both.

**Tradeoff.** An ADR clarifies the two-axis model (curation vs.
extraction) and prevents drift. No ADR risks accidentally bolting
preference-extraction onto steward and breaking its bounded-blast-radius
contract.

**Decide:** {write ADR separating curation vs. extraction / fold into
steward / defer until ceremonies surface concrete preference signals}.

---

### E. Triage drop/notify/act tier

**What openhuman does.** `src/openhuman/agent/triage/` runs *before*
routing on every external trigger (webhook, cron, channel message). The
`evaluator` classifies as drop / notify / act; the `escalation` layer
handles the "act" path. The explicit *drop* tier is what stops
monitoring noise from turning every email into a workstream.

**What arawn does today.** `crates/arawn-engine/src/workstream_router.rs`
routes and acts. It does not have a first-class drop tier — every
inbound effectively flows to a workstream decision.

**Why it matters now.** Once we wire feeds + email + GitHub
notifications, we will drown without a drop tier. Better to introduce
it before those integrations rather than retrofit.

**Tradeoff.** *Layer* in front of routing: cleaner, but two-stage
pipeline. *Refactor* routing to include a drop verdict: less code, but
mixes concerns.

**Decide:** {layer in front (new triage module) / refactor router /
defer until first drown}.

---

## Tier 4 — deliberate non-adoption (decisions, not questions)

### F. Composio backend proxy for 1000+ integrations

OpenHuman's "118+ integrations" are one HTTP call to their hosted
backend (`src/openhuman/composio/`); the backend owns the API keys,
billing markup, HMAC verification, and Socket.IO trigger fan-out.
**Decision:** arawn does not adopt this. We ship direct OAuth integrations
in `arawn-integrations`. The whole point of arawn is that workflow
knowledge stays on-device and there is no vendor account. A backend
proxy is a different product. We accept that this caps integration
breadth for the foreseeable future.

**Threat-model and consent reasoning.**

The openhuman model concentrates trust in three parties the user did
not directly authenticate to:

1. **OpenHuman's backend.** The OAuth `access_token` is issued *to
   their backend*, not to the user's laptop. The backend has read
   access to the user's connected data perpetually, whether the app
   is running or not.
2. **Composio (transitively).** Tokens or requests are forwarded to
   Composio, whose security posture is now part of the user's
   security posture. The user has no relationship with Composio and
   typically does not know it exists.
3. **The auto-fetch loop.** Every 20 minutes their backend actively
   pulls from every connected app, even when the user is not using
   the product. Maximum data exposure, maximum blast radius if either
   backend is compromised.

The mitigating factor is supposed to be that this is *documented* —
the privacy & security gitbook explains the architecture, the source
is open. We reject this as meaningful consent. Being open-source about
a threat model is the SaaS equivalent of fine print in a contract:
technically available, practically unread, requiring expertise the
user does not have to interpret. The UX presented to the user is
"click connect," and nothing in that flow signals that they have just
granted a third party perpetual remote access to their inbox.

**Why this matters for arawn beyond the standalone decision.** The
direct-OAuth model is not chosen primarily because it is "more
secure" in the abstract — it is chosen because *it matches the
mental model a non-expert user forms when they click "connect
Gmail."* They expect the token to live on their laptop because
that is the obvious interpretation. Any architecture that
contradicts that interpretation requires fine print, and fine print
is not consent.

This is the architectural posture arawn enforces: *the security
model must match the obvious user assumption, without requiring
the user to read documentation to be safe.* This rules out
Composio-style proxies regardless of which specific vendor offers
the proxy. It is a property of arawn, not a comment about openhuman.

### G. Channel sprawl

OpenHuman has 14 messaging providers (Slack, Discord, Telegram,
WhatsApp, IRC, Matrix, Signal, iMessage, Email, Lark, Mattermost,
DingTalk, QQ, Linq) behind a `Channel` trait. **Decision:** arawn
resists this. We read inputs from a small set (email, GitHub) and
surface to one or two outputs (TUI, possibly one channel). We are
not a Slack client. The `Channel` trait *shape* is worth borrowing if
and when we add a second output channel; the *breadth* is not.

### H. `agentmemory` shared backend

OpenHuman optionally proxies their memory layer to `rohitg00/agentmemory`
so Claude Code / Cursor / Codex / OpenCode share a single memory store.
**Decision:** arawn does not adopt this. Our steward + workstream model
*is* the differentiator; sharing memory across heterogeneous agents
dissolves the bounded-blast-radius contract that ARAWN-A-0003 codifies.

### I. Subconscious / heartbeat (always-on local reflection)

OpenHuman runs a background loop that reads a `HEARTBEAT.md` checklist
and periodically reflects against memory/graph/skills using the local
model (`src/openhuman/subconscious/`, `src/openhuman/heartbeat/`).
**Provisional decision: defer, not reject.** This is the strongest
"personal AI" feature in their codebase and pairs naturally with
I-0041/42/43, but it adds a continuous background LLM consumer that
fights the "minimal resource footprint" line in the vision. **Revisit
after I-0041 ships** — at that point we will know how much local-model
time the ceremonies already eat and can decide whether always-on
reflection is additive or duplicative.

## Out of Scope

- Memory architecture (their bucket-seal LLD, concentric trees,
  agentmemory proxy). Memory is the differentiator; not for adoption.
- Voice / mascot / Google Meet agent. Out of scope by vision.
- Their plugin system (skills-only). Arawn already has a broader
  out-of-process plugin model in `arawn-engine/plugins`.

## Exit Criteria

- Every tier-3 question (A–E) has a recorded `Decide:` outcome.
- Each `Decide:` outcome either spawns a follow-on task on ARAWN-I-0044
  or is closed with rationale.
- Tier-4 section is locked: no items move from "decided" back to
  "open" without an explicit vision-level revisit.
