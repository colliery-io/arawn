---
id: personal-assistant-identity-layer
level: initiative
title: "Personal-assistant identity layer — make arawn feel like an assistant, not a coding REPL"
short_code: "ARAWN-I-0035"
created_at: 2026-05-06T10:41:01.119111+00:00
updated_at: 2026-05-06T10:41:01.119111+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: personal-assistant-identity-layer
---

# Personal-assistant identity layer

## Context

Three parallel design reviews (visual, interaction, identity) all converged on the same root finding: **arawn reads like a coding REPL with OAuth buttons bolted on, not the personal agentic assistant the vision describes.** Specific evidence:

- `crates/arawn-engine/src/system_prompt.rs:13` `DEFAULT_IDENTITY` opens with the assistant frame, then capsizes: *"Use the tools available to you to assist the user with software engineering tasks, research, file management, and general questions."* That trailing sentence reframes arawn as a coding REPL.
- `system_prompt.rs:22-30` `DEFAULT_DOING_TASKS` is verbatim Claude-Code coding guidance: *"primarily request software engineering tasks — solving bugs, adding features, refactoring."*
- `system_prompt.rs:48-60` `DEFAULT_WORK_PROTOCOL`: *"You are an agent that BUILDS things, not an assistant that DESCRIBES things… use file_write… create files."* For an assistant whose job is mostly summarize + nudge + check, this directive is wrong.
- `crates/arawn/src/main.rs:26` CLI's own `about = "LLM-powered coding assistant"`. Top-level `--help` literally calls itself a coding assistant.
- Status bar (`render.rs:107-192`) surfaces six fields, all about the LLM's plumbing (model, tokens, workstream, permission mode, session id, generating spinner). Zero about the user's life.
- First-run is a blank chat with `Type your message...`. No greeting, no state surface, no hint of what arawn knows or can do.
- `App` state has no field for action items, briefings, last-watcher-run, unread counts, scheduled workflows, or integration health. The data model itself has no concept of an inbox.

The vision (`.metis/vision.md`) explicitly frames arawn as *"a personal agentic assistant that helps you stay organized and on top of life. You watch, check, summarize, and nudge — so you don't have to keep everything in your head."* — but no surface in the TUI does any of those things proactively today.

## Goals & Non-Goals

**Goals:**
- A new user opening arawn for the first time perceives a personal assistant within ~5 seconds. Not "a CLI."
- The empty-chat state surfaces what arawn knows ("3 unread DMs, 14:30 standup, 2 Jira mentions") rather than asking the user to drive everything.
- The system prompt + CLI metadata + status bar all reflect the assistant identity — no internal contradictions.
- Cross-system reasoning is visible at zero cognitive cost (an ambient summary line beats six pull-only commands).
- Workstream-conditioned identity: code-flavored workstreams keep the engineering prompt; life workstreams get a personal-assistant prompt.
- Watcher / nudge surface exists — the `ServerNotice` broadcast is wired to deliver workflow findings as toasts, not just plugin/config events.

**Non-Goals:**
- Replacing the coding-assistant role for users who want that — workstream-conditional identity preserves both.
- Native UIs beyond TUI. Web/desktop is out of scope.
- The full default-workflow-templates surface (filed separately as a follow-up to T-0204; this initiative depends on it but doesn't include it).

## Requirements

### User Requirements
- **Briefing-on-empty:** when chat is empty AND it's the first session of the day, agent emits an automatic briefing turn before any user input.
- **Action-item visibility:** unresolved items (mentions, due tickets, calendar conflicts) accessible without prompting — third sidebar section.
- **Status-bar life-fields:** unread count + next calendar item + alerts visible always.
- **Cross-system summary:** single ambient line above input showing aggregate state across connected services.
- **Identity rewrite:** model behavior reflects assistant role.

### System Requirements
- **REQ-001:** New `briefing_service` in arawn-service produces a structured digest (unread counts, today's calendar, pending Jira/Confluence mentions, recent watcher findings). Used by both the auto-brief turn and the status-bar widgets.
- **REQ-002:** `App` gains state fields for `daily_brief: Option<BriefingSummary>`, `action_items: Vec<ActionItem>`, `integration_health: HashMap<String, IntegrationHealth>`. Populated via RPC at session start and on integration connect/disconnect.
- **REQ-003:** `DEFAULT_IDENTITY`, `DEFAULT_DOING_TASKS`, `DEFAULT_WORK_PROTOCOL` rewritten for personal-assistant framing. New const set `CODING_*` preserves the coding-tool variant. Workstream metadata picks which set is loaded into the system prompt.
- **REQ-004:** CLI `--help` description updated. README + docs updated.
- **REQ-005:** Status-bar layout swaps token-usage/permission-mode for life-fields by default; `/diag` modal shows the engineer-oriented info on demand.
- **REQ-006:** `ServerNotice` broadcast carries new categories: `briefing_ready`, `watcher_finding`, `integration_health`. TUI renders these as ephemeral toasts above the status bar.
- **NFR-001:** Briefing computation runs in <2s for 4-6 connected integrations. Cached for the session; refreshes only on demand or session start.
- **NFR-002:** Workstream identity switch is transparent — no user-visible flip when changing workstreams.

## Use Cases

### Use Case 1: Morning brief on first session of the day

- **Actor:** User opens arawn for the first time today.
- **Scenario:**
  1. TUI launches, creates a session, asks server for a daily brief.
  2. Server runs the briefing pipeline (cross-integration digest) and returns a `BriefingSummary`.
  3. TUI renders the brief as the first chat turn, before any user input. Format: a few bullet points across systems, plus "what would you like to do first?"
- **Expected Outcome:** User reads the brief and either picks an item to dig into or types something else. Either way, the assistant has demonstrated value before being asked.

### Use Case 2: Ambient awareness while typing

- **Actor:** User opens arawn mid-day to ask an unrelated question.
- **Scenario:**
  1. TUI shows the ambient summary line above the input: `📥 5 · 📅 14:30 standup · ⚡ 2 alerts`.
  2. User types their question; agent answers; alerts persist.
- **Expected Outcome:** User saw at-a-glance whether anything was urgent without breaking flow. The 2 alerts can be expanded via `/alerts` or by asking the agent.

### Use Case 3: Workstream-conditioned identity

- **Actor:** User has two workstreams: `personal` (default) and `code/arawn` (engineering).
- **Scenario:**
  1. In `personal` workstream, agent's system prompt reads as a personal assistant. "what's on my plate today" works naturally.
  2. User Tab-switches to `code/arawn`, agent's system prompt swaps to coding tool. "fix the lint errors in this PR" works.
- **Expected Outcome:** Same agent, same UI, two distinct personas based on workstream. No flag, no command — automatic.

## Architecture

### Overview

Three new pieces:

1. **Briefing service** (server-side, `arawn-service` or new `arawn-briefing` crate): aggregates across connected integrations to produce a structured `BriefingSummary`. Runs on demand (RPC) and at session start (auto-fired).
2. **TUI dashboard surfaces:** action-item panel (sidebar), ambient summary line, status-bar life-fields, briefing widget. All read from a per-session `BriefingSummary` cache plus periodic `ServerNotice` updates.
3. **Workstream-conditioned identity:** system_prompt.rs gets `CODING_*` and `ASSISTANT_*` const sets. `LocalService::build_engine_config` picks based on the workstream's `identity_profile` field (added to workstream metadata; defaults to assistant unless explicitly opt-in to coding).

### Notification surface

`ServerNotice` broadcast already exists (`crates/arawn/src/main.rs:611-639`) but only carries plugin/config reload events today. Extend categories:
- `briefing_ready` — new digest available, TUI fetches + renders
- `watcher_finding` — workflow produced something noteworthy, TUI shows toast
- `integration_health` — token rotation, scope mismatch, etc.

TUI gets a small toast renderer (1-line ephemeral overlay above status bar, decays after ~5s).

## Detailed Design

### Phase 1 — Identity correction (small, immediate)

- Rewrite `DEFAULT_IDENTITY` / `DEFAULT_DOING_TASKS` / `DEFAULT_WORK_PROTOCOL` for personal-assistant framing.
- Preserve coding variants under new const names; load based on workstream's `identity_profile` field (new optional column).
- Update CLI `about` string in `main.rs:26`.
- Welcome system message on empty chat: `"Welcome to arawn. Type / for commands, /connect to wire up Gmail/Calendar/Slack, or just chat."`

### Phase 2 — Briefing pipeline

- New `BriefingSummary` struct: counts + headlines per integration.
- `briefing_service` aggregates via existing tool surface (slack_users_list isn't needed at this layer; we just use slack_history, gmail_inbox_read, calendar_upcoming, jira_search via `assignee = currentUser()`, drive_search recent).
- New RPC `get_daily_brief(workstream_id) -> BriefingSummary`.
- `App` gains `daily_brief: Option<BriefingSummary>`; populated on session start.

### Phase 3 — TUI surfaces

- **Briefing widget** rendered in the empty-chat state (replaces today's blank state and the welcome message).
- **Ambient summary line** above the input row: one line of compact glyphs + numbers from `daily_brief`.
- **Action-item panel** as a third sidebar section (after Workstreams, Sessions). Renders `briefing.action_items`.
- **Status-bar life-fields** replace the token-usage cluster. Tokens + model + permission mode move to a `/diag` modal.

### Phase 4 — Notification surface

- Extend `ServerNotice` categories.
- TUI toast renderer (1-line, fades after 5s, queueable).
- Briefing service can fire-and-forget toasts via the broadcast.

## Alternatives Considered

### Alternative A — Keep coding identity, add personal-assistant skills as opt-in

Treat arawn-the-coding-tool as the canonical persona; add `/assistant` mode for the personal-assistant feel.

- **Pro:** Less prompt-rewrite churn.
- **Con:** Defeats the vision's framing entirely. The vision is *"a personal agentic assistant that helps you stay organized and on top of life"* — making that opt-in inverts the priority. The coding role becomes the special case, not the default.

### Alternative B — Web dashboard for the briefing surface

Build a web UI for the briefing/action items, leave the TUI as a chat interface only.

- **Pro:** Richer rendering possible.
- **Con:** Doubles the surface area. The TUI is the canonical client; a separate web dashboard would split attention. TUIs in 2026 (helix, lazygit, atuin) do dashboard-class UI fine.

### Alternative C — Conversational-only, no dashboard surfaces

Skip the dashboard widgets entirely; rely on the agent to surface things in chat when asked.

- **Pro:** Simpler implementation.
- **Con:** Pull-driven. Defeats "watch, check, summarize, nudge." The whole point is ambient awareness without prompting.

## Implementation Plan

Decompose into tasks at design-phase exit. Likely shape:

1. **Phase 1 — Identity correction.** System prompt rewrites, workstream `identity_profile`, CLI `--help`, welcome message. ~200 LOC. Smallest, highest-leverage perception shift.
2. **Phase 2 — Briefing service.** New service trait + RPC + cross-integration aggregation. ~400 LOC + tests.
3. **Phase 3 — TUI dashboard surfaces.** Briefing widget, ambient line, action-item panel, status-bar redesign. ~400 LOC.
4. **Phase 4 — Notification surface.** Extend `ServerNotice` categories, add toast renderer, wire workflow findings. ~200 LOC. Depends on default workflow templates landing (separate follow-up from I-0033 / T-0204).

Estimated complexity: **L** (~1,200 LOC across multiple phases). High strategic value — this is the initiative that converts arawn from "chat tool with integrations" to the assistant the vision describes.

## Status Updates

### 2026-05-06 — Filed (discovery)

Initiative filed after a parallel three-agent design review converged on identity-as-the-single-most-leveraged-finding. All three reviews (visual, interaction, personal-assistant-fit) flagged the system prompt and status bar as fundamentally misaligned with the vision. Filing as L initiative because the right scope spans engine prompt, briefing service, TUI dashboard surfaces, and notification plumbing — too big for one task, too foundational to land piecemeal without a coordinating doc.

Pre-existing work this depends on:
- I-0033 followup item: `workflow_create_decision_task` scaffolding (so workflows can fire briefing-ready notices without per-workflow Rust authoring).
- I-0033 followup item: default workflow templates (the "watching" mechanism).
- The visual coherence pass (ARAWN-I-0036) is a peer initiative — should land in parallel for the dashboard surfaces to look like a designed product.