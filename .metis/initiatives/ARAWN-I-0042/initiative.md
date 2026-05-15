---
id: weekly-prep-ceremony-scheduled
level: initiative
title: "Weekly prep ceremony — scheduled Monday brief setting priorities for the week"
short_code: "ARAWN-I-0042"
created_at: 2026-05-15T12:25:31.682847+00:00
updated_at: 2026-05-15T12:25:31.682847+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: weekly-prep-ceremony-scheduled
---

# Weekly prep ceremony — Monday priorities brief

## Context

One of three ceremonies (I-0041 daily, I-0042 weekly, I-0043 retro). The daily brief is tactical — "what's today" — but it can't answer "what should this week be about." Without a weekly anchor, daily prep risks becoming a treadmill of incoming items with no sense of direction.

The weekly prep runs Monday morning and produces a **priorities tablet**: 3–5 things the user wants to make progress on this week, the calendar shape that supports (or doesn't support) those priorities, and inbound commitments coming due. Daily prep then reads this tablet as input and frames each day relative to the week's priorities.

**This initiative ships the weekly-prep plugin on the `arawn-ceremonies` engine built in I-0043.** The engine handles all plumbing; this initiative implements weekly-specific gather queries, the priorities-candidate compose prompt, the confirm/reject interactive actions, and the `/week` TUI client. The only weekly-specific schema addition is `ceremony_priorities`.

**Sequencing:** ships last of the three. Depends on I-0043 (engine + retro plugin) being in place, and on I-0041 (daily plugin) producing per-day completion records. Also reads back the most-recent retro for context when proposing priorities — so the retro→weekly loop closes only once retros exist.

## Goals & Non-Goals

**Goals:**
- A weekly brief that runs Monday (default: 07:00 local, on by default) producing a `~/.local/share/arawn/ceremonies/weekly/2026-W20.md` tablet.
- Brief content: this week's calendar shape (deep-work blocks vs. meeting load), upcoming deadlines from Jira/Calendar/Gmail, top-of-mind from last retro, a user-editable priorities list (3–5 items).
- Daily prep (I-0041) reads the active weekly tablet and prepends a "This week's priorities" reminder to each daily tablet.
- Priorities list is interactive — the Monday tablet opens with a prompt to confirm/edit priorities; subsequent days are read-only.

**Non-Goals:**
- Auto-generating priorities from signal data. The user picks priorities; arawn proposes candidates but does not decide.
- OKR / quarterly planning. Week is the unit.
- Calendar mutation (blocking time for priorities).
- Retro content (I-0043).

## Detailed Design

### Data model

Reuses the `ceremony_tablets` + `ceremony_sections` + `ceremony_items` tables from I-0041 (kind=`weekly`, period_key=ISO week). Weekly-specific additions:

```sql
CREATE TABLE ceremony_priorities (
    id TEXT PRIMARY KEY,
    tablet_id TEXT NOT NULL REFERENCES ceremony_tablets(id),
    body TEXT NOT NULL,
    rationale TEXT NOT NULL,
    citation_id TEXT,                   -- source row in feeds/signals/last_retro/etc.
    confirmed_at TEXT,                  -- null until user confirms
    done_at TEXT,
    ordinal INTEGER NOT NULL
);

-- column added to ceremony_tablets via migration:
-- priorities_confirmed_at TEXT
```

Candidates are inserted unconfirmed; the Monday confirmation flow either flips `confirmed_at` on selected ones or deletes the rest.

### HTTP API additions

```
GET    /api/ceremonies/week/current               → current weekly tablet (with priorities)
POST   /api/ceremonies/priorities/{id}/confirm    → confirm a priority
DELETE /api/ceremonies/priorities/{id}            → reject a candidate
PATCH  /api/ceremonies/priorities/{id}            → edit body
POST   /api/ceremonies/{tablet_id}/priorities     → add a user-written priority
```

WS events: `ceremony.priority.confirmed`, `ceremony.tablet.generated` (kind=weekly).

### Sample rendering

Same tablet, rendered to markdown by the TUI client from the DB rows:

```yaml
---
kind: weekly
iso_week: 2026-W20
date_start: 2026-05-11
date_end: 2026-05-17
generated_at: 2026-05-11T07:00:00Z
status: open
priorities_confirmed_at: null
---

# Week of May 11–17

## Priorities  (confirm Monday morning — `space` to confirm a candidate)
- [ ] {candidate 1}
- [ ] {candidate 2}
- [ ] {candidate 3}

## Calendar shape
- 12h meetings, 23h deep-work blocks
- Heaviest: Tue (5 meetings)
- Open afternoons: Mon, Thu

## Deadlines this week
- Wed — Jira ARW-142 review (signal_id=…)
- Fri — Q2 budget reply (gmail_id=…)

## From last retro
- {pattern/diary line carrying forward, citing retro_id=…}

## Inbound from last week
- {un-done items from previous weekly tablet}
- {open steward proposals across workstreams}
```

### Priorities-candidate generator

The "candidates" the user picks from are the only LLM-touched part of this ceremony. Gather phase pulls:
- un-done priorities from last week's tablet
- un-done todos that have rolled forward 3+ days in daily tablets (signal: this matters but never finds time)
- workstreams with the most recent activity (signal: this is what's hot)
- explicit calendar items the user has tagged as project work
- patterns + diary excerpts from the most-recent retro

The LLM proposes 5–7 candidate priorities with one-line rationale each, citing the source. User picks 3–5.

### Daily-prep integration

When daily prep (I-0041) runs Tue–Sun, the gather phase queries `ceremony_priorities WHERE tablet_id = (current weekly tablet) AND confirmed_at IS NOT NULL`. Confirmed priorities are written as `ceremony_items` (section_key=`priorities`) on the daily tablet. Daily todo items that share a citation_id with a priority get a marker (computed at render time by the TUI client).

If no weekly tablet exists for the current ISO week (e.g. user skipped Monday or it's day 1 of using the system), daily prep proceeds with no priorities section — graceful degradation handled by the gather phase returning empty.

### TUI client

`/week` is a TUI client of the weekly API. Monday-morning interactive flow:
1. fetch tablet via `GET /api/ceremonies/week/current`
2. render priorities as a selectable list
3. `space` issues `POST /priorities/{id}/confirm`; `d` issues `DELETE`; `a` opens an inline editor → `POST /tablet/priorities`
4. on close, if any priorities confirmed, server flips `ceremony_tablets.priorities_confirmed_at`

Tue–Sun `/week` is read-only (returns the same data; no confirm UI shown).

## Alternatives Considered

- **Combine weekly prep into daily prep "every Monday do extra stuff"** — rejected. The Monday confirmation step needs distinct UX; mixing it into daily prep makes both worse.
- **Auto-confirm priorities (skip user step)** — rejected. The point of priorities is the user committing to them; rubber-stamping defeats the purpose.
- **OKR / quarterly framing** — rejected. Too long a cycle for first-pass; weekly already exercises the ceremony machinery enough.

## Implementation Plan

Decompose during discovery → design. Rough shape:
1. Weekly cloacina workflow + cron registration (reuses I-0041's ceremonies crate).
2. `ceremony_priorities` table + migration; HTTP endpoints for confirm/delete/patch/add.
3. Priorities-candidate gather: SQL queries over rolling-todos, workstream activity, deadlines, last retro.
4. Candidate LLM compose + citation enforcement; writes candidate rows transactionally.
5. Monday interactive confirmation flow in TUI as API client (`/week` mutates via PATCH/DELETE/POST).
6. Daily-prep cross-read hook (I-0041 gather queries `ceremony_priorities`).
7. UAT scenario: synthetic feeds + fake clock + Monday confirmation API calls + assert Tuesday daily tablet rows include priorities section.

## Exit Criteria

- [ ] Monday morning brief generates on schedule with priorities candidates as DB rows.
- [ ] Confirm/reject/edit priorities works via HTTP API + TUI client; `priorities_confirmed_at` updates.
- [ ] Tuesday daily prep gather queries `ceremony_priorities` and includes confirmed ones as daily-tablet items.
- [ ] Un-confirmed weekly tablet still produces a usable daily prep (graceful degradation).
- [ ] Most-recent retro's patterns + diary appear as candidate-priority context.
- [ ] User has run it for ≥2 weeks and confirms it changes how they approach the week.

## Context **[REQUIRED]**

{Describe the context and background for this initiative}

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- {Primary objective 1}
- {Primary objective 2}

**Non-Goals:**
- {What this initiative will not address}

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Detailed Design **[REQUIRED]**

{Technical approach and implementation details}

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered **[REQUIRED]**

{Alternative approaches and why they were rejected}

## Implementation Plan **[REQUIRED]**

{Phases and timeline for execution}