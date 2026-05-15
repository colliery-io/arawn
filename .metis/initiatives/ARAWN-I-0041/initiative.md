---
id: daily-prep-ceremony-scheduled
level: initiative
title: "Daily prep ceremony — scheduled morning brief with agenda + todo surface"
short_code: "ARAWN-I-0041"
created_at: 2026-05-15T12:25:30.219600+00:00
updated_at: 2026-05-15T12:25:30.219600+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: daily-prep-ceremony-scheduled
---

# Daily prep ceremony — morning brief

## Context

The substrate built in I-0039 (continual feeds) and I-0040 (signal extraction + palaces) gives arawn a workstream-scoped knowledge graph fed by Gmail/Calendar/Drive/Slack/Atlassian. Today it's reactive — nothing happens until the user opens chat and asks. To cross the gap from "knowledge base with chat" to "personal work assistant," arawn needs scheduled ceremonies that run on their own and surface what matters.

**This initiative ships the daily-prep plugin on the `arawn-ceremonies` engine built in I-0043.** All the heavy plumbing (scheduling, artifact tables, gather→compose→write pipeline, citation enforcement, HTTP/WS surface) lives in the engine. This initiative implements the daily-specific gather queries, compose prompt, interactive todo-toggle action, and `/today` TUI client.

**Sequencing:** I-0043 (engine + retro plugin) ships first. Daily prep (this initiative) is the second ceremony plugin. Weekly prep (I-0042) is third. Implementation of this initiative should not start until I-0043's engine and schema are in place.

## Goals & Non-Goals

**Goals:**
- A daily brief that runs on a user-configured cron (default: 07:00 local, on by default) and produces a persistent "tablet" artifact for the day.
- Brief content: today's calendar at a glance, prioritized inbox/Slack items needing attention, open steward proposals worth approving, and a working todo list for the day.
- Persistent surface (not a chat output): an artifact the user can return to, mark items done on, and that the retro can read back.
- Opt-out at the user level (`arawn ceremonies disable daily`) and skip-once (`/skip-daily`).
- Cross-workstream by default — the brief spans everything the user cares about, not one workstream.

**Non-Goals:**
- Weekly views (I-0042) or retrospection (I-0043).
- A web/desktop GUI. The HTTP API defined here anticipates GUI clients but no GUI ships in this initiative — TUI is the only client in v1. (See ADR-pending: ceremonies GUI roadmap.)
- LLM-generated motivational tone, emoji decoration, "good morning!" preamble. Content density over warmth.
- Calendar mutation (rescheduling, creating events). Read-only surface in v1.

## Detailed Design

### Data model (canonical store)

Source of truth is SQLite in the arawn-ceremonies DB (colocated with server data). Markdown is only a rendering output, never the persistence layer.

```sql
CREATE TABLE ceremony_tablets (
    id TEXT PRIMARY KEY,            -- e.g. daily-2026-05-15
    kind TEXT NOT NULL,             -- daily | weekly | retro
    period_key TEXT NOT NULL,       -- date (daily), iso_week (weekly/retro)
    generated_at TEXT NOT NULL,
    status TEXT NOT NULL,           -- open | reviewed | archived
    workstreams_scanned TEXT NOT NULL,  -- JSON array
    UNIQUE(kind, period_key)
);

CREATE TABLE ceremony_sections (
    tablet_id TEXT NOT NULL REFERENCES ceremony_tablets(id),
    section_key TEXT NOT NULL,      -- calendar | attention | proposals | todos | ...
    ordinal INTEGER NOT NULL,
    title TEXT NOT NULL,
    PRIMARY KEY (tablet_id, section_key)
);

CREATE TABLE ceremony_items (
    id TEXT PRIMARY KEY,
    tablet_id TEXT NOT NULL REFERENCES ceremony_tablets(id),
    section_key TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    kind TEXT NOT NULL,             -- calendar_event | attention | proposal | todo | freeform
    body TEXT NOT NULL,             -- structured per kind (JSON)
    citation_id TEXT,               -- signal_id / event_id / proposal_id / null for freeform
    done_at TEXT,                   -- todos only
    created_at TEXT NOT NULL
);

CREATE TABLE ceremony_todos_rolling (
    todo_id TEXT PRIMARY KEY,       -- stable id across days
    body TEXT NOT NULL,
    origin_tablet_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    done_at TEXT,
    last_seen_tablet_id TEXT NOT NULL   -- updated each day a todo rolls forward
);
```

Un-done todos persist as rows in `ceremony_todos_rolling`; each daily generation links them into the new tablet via `ceremony_items` (kind=`todo`).

### HTTP API (canonical interaction surface)

All clients — TUI today, GUI tomorrow — interact via these endpoints on the arawn server:

```
GET    /api/ceremonies/today                          → current daily tablet
GET    /api/ceremonies/{kind}/{period_key}            → specific tablet
GET    /api/ceremonies/{tablet_id}/items              → items (filtered by section)
PATCH  /api/ceremonies/items/{item_id}                → toggle todo done, edit freeform
POST   /api/ceremonies/{tablet_id}/items              → add a todo
POST   /api/ceremonies/{kind}/run                     → manual trigger (idempotent per period_key)
POST   /api/ceremonies/config                         → enable/disable, cron, workstream filter
GET    /api/ceremonies/notifications                  → unread tablets pending review
```

Realtime: server emits `ceremony.tablet.generated` and `ceremony.item.updated` events on the existing WS channel; clients reactively refresh.

### Markdown rendering (TUI-side concern)

`/today` is a TUI **client** of the API. The client:
1. fetches the tablet + items via `GET /api/ceremonies/today`
2. renders the rows to markdown locally using the existing markdown rendering pipeline
3. on `space` toggle, issues `PATCH /api/ceremonies/items/{id} {done: true}`
4. subscribes to WS events for refresh

No markdown lives on disk. If users want an export, it's a `arawn ceremonies export <id>` CLI command that renders from the DB.

### Scheduling

Reuse cloacina (already in-process for feeds). New `arawn-ceremonies` crate exposes one workflow per ceremony kind. Each workflow:
1. reads ceremonies config from DB (`ceremony_config` table; managed via API)
2. queries feeds + signal stores + steward journal for the relevant window
3. drives an LLM brief-generator chain (small, structured prompt; no free-form chat)
4. writes tablet + items rows in a single transaction; emits `ceremony.tablet.generated` event

The ceremonies workflow shares the cloacina runtime with feed ingestion — no new scheduler.

### Brief generator chain

Two-stage to keep the LLM honest:
1. **Gather** (deterministic, no LLM): pull calendar events, top-N unread Slack DMs + mentions, top-N unread Gmail with sender-importance heuristic, open steward proposals across workstreams, yesterday's un-done todos.
2. **Compose** (LLM): given the gathered facts as JSON, write the markdown sections. Cite every claim with a signal_id / event_id / proposal_id. No fact may appear that isn't in the gather payload.

Citation discipline matters: this is the only thing keeping the brief grounded vs. hallucinated.

## Alternatives Considered

- **Chat-only output ("just print it in the TUI when you start")** — rejected. Ceremonies must be persistent so the retro can read them back and the user can mark todos done over the course of the day.
- **External cron + CLI** — rejected for v1. Keeps activation friction up and complicates state (where does the artifact go if arawn isn't running?). Cloacina in-process is simpler.
- **One unified "brief" with mode flag instead of three initiatives** — rejected. Daily/weekly/retro have distinct content shapes and cadences; sharing the artifact-writing + scheduling plumbing is enough.

## Implementation Plan

Decompose into tasks during discovery → design. Rough shape:
1. `arawn-ceremonies` crate scaffold + SQLite schema + migrations + cloacina workflow registration.
2. `ceremony_config` table + HTTP API CRUD for enable/disable/cron/workstream-filter.
3. Tablet + items + rolling-todos tables, plus the read/mutate HTTP endpoints.
4. WS event channel for `ceremony.*` events.
5. Gather phase: calendar/feed/signal/proposal queries → in-memory `GatheredFacts` struct.
6. Compose phase: structured prompt + citation-enforcement check; writes rows transactionally.
7. TUI `/today` rewritten as API client (fetches, renders markdown, mutates via PATCH).
8. End-to-end UAT scenario (synthetic feeds → fake clock → assert DB rows + API responses + citations).
9. Opt-out plumbing via API (`POST /api/ceremonies/config`) and TUI command (`/skip-daily`).

## Exit Criteria

- [ ] User can run `arawn ceremonies enable daily --at 07:00` and the brief generates on schedule.
- [ ] Tablet + items written to `ceremony_*` SQLite tables; HTTP API serves them; `/today` TUI client renders from API.
- [ ] WS events fire on tablet generation and item updates; TUI auto-refreshes.
- [ ] Every fact in the brief carries a verifiable citation id.
- [ ] Un-done todos roll forward to the next day.
- [ ] User has run it on their real accounts for ≥1 week and the output is useful (subjective gate — captured as a written note in the retro initiative once retros exist; until then, user verbal sign-off).
- [ ] Opt-out works at install (default on, but `arawn ceremonies disable daily` cleanly stops it).