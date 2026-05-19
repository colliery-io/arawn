---
id: github-integration-repos
level: initiative
title: "GitHub integration — repos, notifications, issues, PRs as a first-class personal signal"
short_code: "ARAWN-I-0045"
created_at: 2026-05-15T15:09:09.839794+00:00
updated_at: 2026-05-18T13:26:25.914974+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: github-integration-repos
---

## Context **[REQUIRED]**

The arawn vision names GitHub explicitly as one of the channels arawn should monitor ("watches, checks, summarizes, and nudges"). Today we have zero GitHub surface — every other listed input source (email, calendar, drive, Atlassian, Slack) has at least a starter integration. This is a conspicuous gap.

GitHub is also the integration where openhuman's comparative dive (ARAWN-I-0044) does not help us much: their Composio-proxy approach gives them GitHub for free, but the data flows through their backend (rejected per ARAWN-S-0004 §F). We do this direct-OAuth like the rest of `arawn-integrations`.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- Read-side only: pull GitHub notifications, assigned issues, review-requested PRs, and authored issues/PRs into a shape arawn can route, summarise, and surface in the morning brief (I-0041). v1 is strictly an information stream — arawn observes, the human acts.
- Per-account multi-org: a single GitHub identity routinely participates in several orgs/repos. Routing scopes cleanly at the **per-repo** level by default, with a **per-org** binding option for users who want a whole org to fan into one workstream.
- Threat-model parity with the rest of `arawn-integrations`: token encrypted at rest in the data dir (ChaCha20Poly1305 + per-data-dir master key, per ARAWN-A-0001); zero traffic through any third party.

**Non-Goals:**
- **Write tools** (comment / label / mark-read). v1 is read-only. The agent surfaces signals; humans interact via GitHub itself. Write-side may land in a follow-up initiative once the signal stream proves itself.
- Code-review automation. We are not Greptile / CodeRabbit / Cursor.
- Actions / workflow management. Reading workflow runs is fine; orchestrating them is out of scope.
- Org admin operations (member management, security alerts, billing). Skip entirely.
- Webhook delivery to arawn. Openhuman uses a hosted tunnel for this; for us, poll on the existing cron schedule — webhook ingestion is a separate later decision.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

### Functional sketch
- REQ-001: GitHub App (locked) — finer scope control, cleaner refresh story than OAuth App.
- REQ-002: Per-installation scope: the user picks which repos/orgs the install can see.
- REQ-003: Notifications pull (paginated, since-cursor).
- REQ-004: Issues + PRs assigned to or authored by the user across the accessible scope.
- REQ-005: Review-requested PRs (review queue).
- REQ-006: Default poll cadence **30 min** for all read paths (notifications, issues, PRs). User-overridable via per-feed cadence in `[feeds]` config.
- REQ-007: Workstream binding: per-repo (`owner/repo`) is the default; a `github:org:<owner>` binding fans an entire org's repos into one workstream.
- REQ-008: Rate-limit handling that respects GitHub's primary + secondary limits gracefully.

### Non-functional
- NFR-001: Read calls cached with `If-Modified-Since` / `ETag` to keep the rate budget cheap.
- NFR-002: Token storage matches ARAWN-A-0001.
- NFR-003: Operates fully offline against cached state for read paths once warmed.

## Detailed Design **[REQUIRED]**

### Design Decisions (locked 2026-05-18)

- **Auth: GitHub App.** User installs per-org; scope is per-installation; token refresh via app's installation-access-token flow. Token stored encrypted per ARAWN-A-0001.
- **Ingestion: `arawn-feeds` template family.** GitHub gets three feed templates: `github/notifications`, `github/issues-and-prs`, `github/review-queue`. Each emits typed projection rows (vs Gmail's free-form JSON) into a new `github_*` projection table family. Extractor → memory pipeline downstream is unchanged.
- **Routing: per-repo default, per-org option.** Workstream bindings accept both `github:repo:owner/name` and `github:org:owner` forms. Per-repo wins on conflict (more specific). The extractor walks bindings repo-first, then falls back to org.
- **Polling cadence: 30 min default**, user-overridable via `[feeds]` config.
- **Read-only v1.** No write tools in this initiative. Comment / label / mark-read are explicitly out of scope; if they ever land, they're a separate initiative.
- **No webhook ingestion in v1** — poll-only, same as other integrations. Webhook decision deferred.

### Existing pattern to copy
`crates/arawn-integrations/src/gmail/` is the closest parallel for the OAuth dance + encrypted token. For the ingestion shape, `crates/arawn-feeds/src/templates/jira/` and `crates/arawn-feeds/src/templates/calendar/` are better references — they emit typed projection rows. GitHub items (issue / PR / notification) are structured enough to warrant their own typed schemas rather than free-form JSON pass-through.

## Alternatives Considered **[REQUIRED]**

- **Skip GitHub, rely on email notifications from GitHub.** Rejected: email-based GitHub is high-volume, low-structure, and arawn would have to reverse-engineer issue/PR state from notification HTML. Direct API is cheaper and cleaner.
- **Composio-proxied (openhuman's path).** Rejected per ARAWN-S-0004 §F — token off-device.
- **Personal-access-token only.** Rejected: PATs have all-or-nothing scope, no refresh story, and require the user to manage them in GitHub's UI. GitHub App is the modern path even if installation is one extra step.

## Implementation Plan **[REQUIRED]**

Decompose during design phase. Rough shape (~6 tasks):

1. `arawn-integrations::github` scaffold: GitHub App OAuth dance (install-app → device-code or web flow → installation-access-token exchange), encrypted token store, refresh loop. Mirrors Gmail's structure.
2. Typed projection schemas: `github_notifications`, `github_issues_and_prs`, `github_review_queue` tables in projections.db. Rust DTOs + write paths.
3. Feed template `github/notifications`: paginated poll with `since` cursor + `If-Modified-Since`. Rate-limit-aware backoff.
4. Feed template `github/issues-and-prs`: queries the user's open + recently-closed issues/PRs across the installation scope.
5. Feed template `github/review-queue`: PRs where the user is a requested reviewer.
6. Workstream binding extension: accept `github:repo:owner/name` and `github:org:owner`. Bind-backfill hook walks the matching projection rows into the workstream's KB on first bind.

Soft dependency on ARAWN-I-0044's triage drop tier work for routing-confidence scoring — not blocking; the read path lands first.

## Exit Criteria

- GitHub items show up in the morning brief (I-0041) with at least one row per category (notifications / assigned issues / review-requested PRs).
- A user can `/watch github/notifications` and see new entries in the feed runtime within one cron tick.
- A user can `/workstream bind <ws> github:repo:owner/name` (or `github:org:owner`) and the bind-backfill hook pulls relevant rows into the workstream's KB.
- Token survives a daemon restart and stays encrypted on disk per ARAWN-A-0001.
- No write surface — `gh pr comment`-style operations remain a follow-up.