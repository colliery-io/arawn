---
id: phase-c-2-tutorials-trim-first
level: task
title: "Phase C-2: Tutorials — trim first-chat, write first-workstream"
short_code: "ARAWN-T-0334"
created_at: 2026-05-19T01:39:35.221362+00:00
updated_at: 2026-05-19T02:20:22.914885+00:00
parent: ARAWN-I-0051
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0051
---

# Phase C-2: Tutorials — trim first-chat, write first-workstream

## Parent Initiative

[[ARAWN-I-0051]]

## Objective

Trim `docs/src/getting-started.md` to its tutorial core (~200 lines) at `tutorials/first-chat.md` — build → configure (one provider) → first message → first tool call, no OAuth detours. Promote the §8 workstream quickstart into a full new tutorial at `tutorials/first-workstream.md` — end-to-end create → bind → first projection → `signal_search`. Delete `getting-started.md` once content distributed.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

- [x] `tutorials/first-chat.md` written — ~125 lines, build → configure (Groq) → server → TUI → first message + tool call. No OAuth detours.
- [x] `tutorials/first-workstream.md` written — ~140 lines, end-to-end `/workstream create` → ontology → `/workstream bind` → `/feeds run` → `signal_search` / `signal_query` / `signal_timeline` + steward proposal preview.
- [x] `intro.md` updated to point at the new tutorials and the four-quadrant structure.
- [x] `SUMMARY.md` updated — Tutorials section lists first-chat then first-workstream.
- [x] `getting-started.md` deleted (`git rm`). Source prose recoverable via `git show HEAD~1:docs/src/getting-started.md` if C-3/C-4/C-5 want it.
- [x] `angreal docs build` clean.

## Status Updates

### 2026-05-18 — Completed (uncommitted)

Wrote both tutorials. Decisions worth noting:

- **`first-chat.md` uses Groq as the default example.** Free tier + fast warmup is the lowest-friction first session. Other providers (Ollama Cloud, local Ollama, OpenAI, Anthropic) get a one-line mention with a pointer to `config-schema.md`.
- **`first-workstream.md` assumes Gmail.** The Phase A audit found that arawn auto-creates a `gmail/inbox-archive` feed on `/connect gmail` — that's the lowest-friction "real data" path for a tutorial. Other providers and the `github:repo:` / `github:org:` URI schemes get pointers to `bind-a-workstream-to-a-feed.md` (C-4).
- **`getting-started.md` deleted outright** rather than parked as a "legacy" transitional file (per operator preference). Original prose lives at HEAD~1. C-3 will rebuild OAuth recipes from code + provider docs + that git ref; C-4 will rebuild the `/watch` recipe similarly; C-5 will rebuild the troubleshooting tables.
- **No accuracy drift introduced.** Used `/feeds rm` (not the stale `/unwatch` from the original), didn't repeat the WIP disclaimers on `/remember`/`/memory`/`/forget`, used real tool names.

Forward references to pages that don't exist yet (`how-to/connect-google.md`, `how-to/recover-from-llm-warmup-failure.md`, `how-to/curate-a-workstream.md`, `reference/config-schema.md`, etc.) are intentional — they'll resolve as later C-tasks land. mdbook doesn't fail on them since they're inline markdown links, not `SUMMARY.md` entries.