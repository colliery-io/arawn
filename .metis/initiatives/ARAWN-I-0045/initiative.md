---
id: github-integration-repos
level: initiative
title: "GitHub integration — repos, notifications, issues, PRs as a first-class personal signal"
short_code: "ARAWN-I-0045"
created_at: 2026-05-15T15:09:09.839794+00:00
updated_at: 2026-05-15T15:09:09.839794+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: github-integration-repos
---


## Context **[REQUIRED]**

The arawn vision names GitHub explicitly as one of the channels arawn should monitor ("watches, checks, summarizes, and nudges"). Today we have zero GitHub surface — every other listed input source (email, calendar, drive, Atlassian, Slack) has at least a starter integration. This is a conspicuous gap.

GitHub is also the integration where openhuman's comparative dive (ARAWN-I-0044) does not help us much: their Composio-proxy approach gives them GitHub for free, but the data flows through their backend (rejected per ARAWN-S-0004 §F). We do this direct-OAuth like the rest of `arawn-integrations`.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- Read-side: pull GitHub notifications, assigned issues, review-requested PRs, and authored issues/PRs into a shape arawn can route, summarise, and surface in the morning brief (I-0041).
- Write-side, narrow: comment on an issue or PR; add/remove labels; mark notifications as read. These cover the agent-as-assistant story without making arawn a code-review tool.
- Per-account multi-org: a single GitHub identity routinely participates in several orgs/repos. The integration must scope cleanly per repo or per org for routing decisions.
- Threat-model parity with the rest of `arawn-integrations`: token encrypted at rest in the data dir (ChaCha20Poly1305 + per-data-dir master key, per ARAWN-A-0001); zero traffic through any third party.

**Non-Goals:**
- Code-review automation. We are not Greptile / CodeRabbit / Cursor. Arawn surfaces what is happening; humans review.
- Actions / workflow management. Reading workflow runs is fine; orchestrating them is out of scope.
- Org admin operations (member management, security alerts, billing). Skip entirely.
- Webhook delivery to arawn. Openhuman uses a hosted tunnel for this (their `webhooks/` module). For us, poll on the existing cron schedule — webhook ingestion is a separate later decision.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

### Functional sketch
- REQ-001: OAuth (GitHub Apps preferred over personal tokens — finer scope control, refresh story is cleaner).
- REQ-002: Per-installation scope: the user picks which repos/orgs the install can see.
- REQ-003: Notifications pull (paginated, since-cursor).
- REQ-004: Issues + PRs assigned to or authored by the user across the accessible scope.
- REQ-005: Review-requested PRs (review queue).
- REQ-006: Comment + label + mark-read agent tools.
- REQ-007: Rate-limit handling that respects GitHub's primary + secondary limits gracefully.

### Non-functional
- NFR-001: Read calls cached with `If-Modified-Since` / `ETag` to keep the rate budget cheap.
- NFR-002: Token storage matches ARAWN-A-0001.
- NFR-003: Operates fully offline against cached state for read paths once warmed.

## Detailed Design **[REQUIRED]**

### Pre-design open questions
- GitHub App vs OAuth App. App is the right answer for scope control but adds installation friction (the user installs the app per org). Decide before decomposition.
- Storage shape: do GitHub items become `arawn-memory` entries directly, ride the existing feed-style ingestion (`arawn-feeds`), or get their own crate? The existing `arawn-integrations/gmail` pattern probably applies and is the lowest-friction path.
- Routing: how does a "github issue assigned to you" item turn into a routing decision (workstream vs drop)? Depends on the triage drop tier (ARAWN-S-0004 §E) — log a soft dependency, do not block on it.
- Per-repo vs per-org granularity in routing. Likely per-repo because that maps to how users *actually think* about their attention.

### Existing pattern to copy
`crates/arawn-integrations/src/gmail/` is the closest parallel: OAuth dance, encrypted token, paginated fetch, since-cursor. Reuse the structure. Diff from Gmail: GitHub has structured items (issue, PR, notification) rather than free-form messages; the ingestion shape benefits from typed Rust models instead of raw JSON pass-through.

## Alternatives Considered **[REQUIRED]**

- **Skip GitHub, rely on email notifications from GitHub.** Rejected: email-based GitHub is high-volume, low-structure, and arawn would have to reverse-engineer issue/PR state from notification HTML. Direct API is cheaper and cleaner.
- **Composio-proxied (openhuman's path).** Rejected per ARAWN-S-0004 §F — token off-device.
- **Personal-access-token only.** Rejected: PATs have all-or-nothing scope, no refresh story, and require the user to manage them in GitHub's UI. GitHub App is the modern path even if installation is one extra step.

## Implementation Plan **[REQUIRED]**

Discovery-phase deliverables, not decomposed yet:

1. Decide GitHub App vs OAuth App (recorded as an ADR if it provokes discussion).
2. Decide ingestion storage shape (memory direct / feed / new crate).
3. Decompose into tasks (~6–8 expected): OAuth + token store; notifications poll; issues + PRs pull; comment + label tools; mark-read tool; rate-limit + caching; routing hookup.

Soft dependency on ARAWN-I-0044's triage drop tier work (the routing question). Not blocking — we can land the read path first and refine routing later.

## Exit Criteria

- GitHub items show up in the morning brief (I-0041) with at least one row per category (notifications / assigned issues / review-requested PRs).
- Agent can comment + label + mark-read on a target issue/PR via tool call, end-to-end, against a real account.
- Token survives a daemon restart and stays encrypted on disk.
