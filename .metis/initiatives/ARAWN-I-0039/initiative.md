---
id: continual-data-feeds-opinionated
level: initiative
title: "Continual data feeds — opinionated, configurable, local-first ingestion across personal + watched spaces"
short_code: "ARAWN-I-0039"
created_at: 2026-05-06T23:33:02.124351+00:00
updated_at: 2026-05-12T00:47:40.961433+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: continual-data-feeds-opinionated
---

# Continual data feeds — opinionated, configurable, local-first ingestion across personal + watched spaces Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

Today every integration in arawn is **pull-on-demand**. The agent calls `gmail_inbox_read`, `slack_channel_history`, `jira_search`, etc. only when the user (or the agent during a turn) explicitly invokes the tool. There is no local cache, no continual fetch, and no concept of a "feed" — every question costs a round-trip to a provider, every answer is bounded by what the agent thought to ask in that turn.

This blocks every assistant-shaped feature on the roadmap:

- I-0035 (personal-assistant identity) wants an ambient summary line, briefing widget, and action-item panel — all of which need fresh per-integration state without a per-render API call.
- I-0035 Phase 2 (`/brief`) does cross-integration aggregation; on demand it's a 2–10 second fanout to 4–6 providers per call.
- "Watch X for me" — *"tell me when @design pings me in #design-team"*, *"alert me when ENG-1234 changes status"*, *"summarize updates in PROJ space daily"* — has no place to live. Watchers don't exist.
- The agent has no way to look back across days/weeks. Every invocation starts blind.

The current pull-only model also forces the user to be the only data source for "what should the assistant know about." There is no model for **spaces I choose to watch** — channels, projects, repos, calendars belonging to others I have legitimate access to and want surfaced.

The vision (`.metis/vision.md`) frames arawn as *"a personal agentic assistant that helps you stay organized and on top of life. You watch, check, summarize, and nudge — so you don't have to keep everything in your head."* The "watch" verb has no implementation today.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- **Per-provider templates** that encode opinionated patterns: "archive a Slack channel," "track a Jira project with all comments," "save emails matching a sender filter," "sync a Drive folder to disk." User configures named instances (provider + template + parameters + cadence) in `arawn.toml`.
- **Disk-first, raw, browseable.** Every fetch writes provider-shaped data to plain files under `~/.arawn/data/`. JSON / JSONL / markdown / native binary depending on what fits each template. No DB, no schema migration, no query layer in v1 — `find`, `grep`, `cat` are the query language.
- **Two source classes, same templates:**
  - **Personal feeds** — auto-configured on integration connect from sensible defaults (Gmail inbox, Calendar today+7d, Slack `@me` mentions, Jira `assignee = currentUser()`).
  - **Watched spaces** — explicit user opt-in. A Slack channel, a Jira project, a Confluence space, a Drive folder, a GitHub repo.
- **Cloacina-driven scheduling with catchup.** Each configured feed registers as a cloacina cron task; cloacina's existing recovery/catchup machinery handles "machine was asleep when this was due" — runs are made up on next wake rather than silently skipped. We get retry, instance management, and the audit trail for free.
- **Local-only.** Tokens never leave the machine; neither does data. The data dir is just files — back it up however you back up your home dir.
- **Substrate for I-0035 and beyond.** Briefing aggregation, action items, watcher rules, agent retrieval — all read from the data dir using the same file tools the agent already has (read, grep, glob).

**Non-Goals:**
- **Real-time push** for v1. Some providers (Slack, GitHub) support webhooks/Events APIs; this initiative ships polling first because polling is universal. Push is a follow-up that swaps the trigger without changing the storage model.
- **Multi-tenant data sharing.** Every feed is per-user, per-machine. No syncing between machines, no collaborative spaces.
- **Reading provider data into the LLM context window.** This initiative produces a **store**; surfacing it to the agent (briefing pipeline, watch-tool injection) is I-0035's job.
- **Server-hosted deployment.** Local-only. A future hosted variant would re-think privacy, but that's not this.
- **Replacing the existing on-demand tools.** `gmail_inbox_read` and friends stay — they're still the right surface for "search my Gmail for X" type asks. Feeds add a *standing* layer alongside the existing *imperative* layer.

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

### Three core concepts

```
Template               Feed                          Run
────────               ────                          ───
slack/channel-archive  "#design every hour"          a single fetch + write
jira/project-tracker   "ENG daily with comments"     a single fetch + write
gmail/sender-filter    "boss@x.com daily"            a single fetch + write
drive/folder-sync      "/Documents/projects daily"   a single sync pass
```

- **Template** — a named, parameterized fetch+write recipe owned by an integration. `slack/channel-archive` knows how to enumerate messages in a channel since the last cursor and append them as JSONL. `drive/folder-sync` knows how to walk a folder and rsync changed files to disk. Templates are code in `arawn-feeds/src/templates/<provider>/<name>.rs`.
- **Feed** — a configured instance of a template: which provider creds, which template, what parameters, what cadence, where on disk. Defined in `arawn.toml` or added at runtime via `/watch`.
- **Run** — a single execution of a feed, scheduled by cloacina. Run is responsible for: refresh token if needed → call provider → write to disk → persist cursor for next run → emit audit row.

### Disk layout

```
~/.arawn/data/
├── slack/
│   └── channel-archive/
│       └── design-CABCDEF/
│           ├── meta.json              # { template, source, cursor, last_run, ... }
│           ├── 2026-05-06.jsonl       # one Slack message per line, append per run
│           ├── 2026-05-05.jsonl
│           └── ...
├── jira/
│   └── project-tracker/
│       └── ENG/
│           ├── meta.json
│           ├── ENG-123/
│           │   ├── issue.json         # latest snapshot, overwrite per run
│           │   ├── comments.jsonl     # append-only
│           │   └── history.jsonl
│           └── ENG-124/...
├── gmail/
│   └── sender-filter/
│       └── boss-at-company/
│           ├── meta.json
│           ├── 2026-05-06/
│           │   ├── 17a3...json        # one file per email, raw API payload
│           │   └── 17a4...json
│           └── ...
└── drive/
    └── folder-sync/
        └── documents-projects/
            ├── meta.json
            ├── design-doc.md          # actual file body, native format
            ├── meeting-notes.md
            └── slides.pdf
```

Each template owns its own layout convention. Common rules:

- One subdirectory per `feed` instance (so two `slack/channel-archive` feeds for `#design` and `#eng` don't collide).
- A `meta.json` at the feed root with `{ template, source, cursor, last_run_at, last_status, run_count }`. Cloacina's job state is the source of truth for *scheduling*; `meta.json` is the source of truth for *what we've fetched*.
- Time-partitioned (`YYYY-MM-DD/`) for high-volume append (Slack channels, emails). Per-record (`per-id/`) for low-volume but mutable (Jira issues with comments).
- Native-format mirroring for content stores (Drive). The agent's existing `read` / `grep` / `glob` tools just work on these.

### Cloacina coupling — feeds are NOT pipelines

Decision (locked 2026-05-08): **feeds are cloacina cron schedules over per-feed in-process workflows that wrap a single generic dispatcher task.** They are *not* compiled `.cloacina` packages.

For each configured feed we:

1. Register a one-task workflow with `Runtime::register_workflow(format!("feed::{feed_id}"), || WorkflowBuilder::new(...).add_task(Arc::new(FeedDispatchTask { feed_id })).build()?)`. The closure captures the feed's id; the inner `FeedDispatchTask` looks up the template + params from the `feeds` table at run-time.
2. Register a cron schedule via `runner.register_cron_workflow(&format!("feed::{feed_id}"), &cadence, "UTC")`. cloacina's catchup/retry/audit machinery applies per-feed automatically because each feed has a distinct `workflow_name`.

Only one Rust `Task` impl exists (`FeedDispatchTask`); workflows are thin wrappers naming the feed. Adding a new template is one Rust file in `arawn-feeds/src/templates/<provider>/<name>.rs`. No cargo project per feed, no `.cloacina` archive, no reconciler involvement.

Why not "one shared `arawn_feeds_dispatch` workflow with feed_id passed via context"? `register_cron_workflow` doesn't accept a per-schedule context payload — at fire time the workflow gets a fresh empty context. Without a per-feed `workflow_name` we'd lose audit clarity and have no clean way to recover which feed triggered.

### Scheduling — cloacina with catchup

arawn already runs cloacina as the workflow runner (`arawn-workflow` crate, started in `main.rs` server bootstrap). Each configured feed registers as described above. Cloacina handles:

- **Catchup**: if the machine was asleep at 3am when an hourly feed was due, cloacina's recovery service picks it up on next start. Configurable per-feed (`catchup_max_misses` — collapse N missed runs into one execution rather than running it N times back-to-back).
- **Single-instance enforcement**: a long-running run won't double-trigger if the next tick fires.
- **Retry / backoff**: for transient provider errors, leverages cloacina's existing retry policy.
- **Audit trail**: every run is in cloacina's run history, queryable for debugging.

We don't write a per-feed scheduler — feeds are just cron tasks with a custom `Task::execute` body that calls the template's `fetch + write`.

### Template trait

```rust
#[async_trait]
trait FeedTemplate {
    /// Stable identifier "<provider>/<template>" — e.g. "slack/channel-archive".
    fn name(&self) -> &'static str;

    /// Validate parameters from arawn.toml at startup; fail fast on
    /// missing channel id / unknown project / etc.
    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError>;

    /// Suggest a default cadence + initial cursor for the parameters.
    fn defaults(&self, params: &TemplateParams) -> FeedDefaults;

    /// Run one fetch+write cycle. Reads cursor from `feed_dir/meta.json`,
    /// writes new content under `feed_dir/`, updates cursor + last_run.
    /// Returns a small RunSummary for cloacina's audit row.
    async fn run(
        &self,
        ctx: &TemplateCtx,
        params: &TemplateParams,
        feed_dir: &Path,
    ) -> Result<RunSummary, FeedError>;
}
```

`TemplateCtx` carries the integration handle (already-authed clients from `arawn-integrations`), the `arawn-auth` token store for refreshes, and a structured logger.

### Configuration shape (arawn.toml)

```toml
# All examples — pick the ones you want.

[[feed]]
provider = "slack"
template = "channel-archive"
channel  = "#design"           # template-specific param
cadence  = "0 * * * *"         # cron — every hour
catchup_max_misses = 3         # collapse missed runs

[[feed]]
provider = "jira"
template = "project-tracker"
project  = "ENG"
include_comments = true
cadence  = "0 0 * * *"         # daily at midnight

[[feed]]
provider = "gmail"
template = "sender-filter"
sender_pattern = "boss@company.com"
cadence  = "0 6 * * *"         # daily at 6am

[[feed]]
provider = "drive"
template = "folder-sync"
folder   = "/Documents/projects"
direction = "pull"              # one-way pull only for v1
cadence  = "0 */4 * * *"       # every 4 hours

# Auto-feeds (created on /connect <provider> from per-provider defaults).
# These appear in arawn.toml on first connect and you can edit / remove.
[[feed]]
provider = "gmail"
template = "inbox-archive"      # default personal feed
cadence  = "*/15 * * * *"

[[feed]]
provider = "slack"
template = "my-mentions"        # default personal feed
cadence  = "*/5 * * * *"
```

A `/watch <provider> <template> [params]` slash command adds feeds without editing the file.

### Reading the data

In v1 the agent reads via existing `read` / `grep` / `glob` / `shell` tools. The data dir is intentionally browseable. A `feeds` slash command in the TUI lists configured feeds + last-run state for human inspection.

A future task layers an indexer / query tool on top once we know which queries actually matter (likely "give me everything written under `data/` since timestamp T" for the briefing pipeline).

### Failure modes

- **Token refresh fails / scope removed** — template returns `FeedError::Auth`; cloacina marks the run failed; feed pauses until next manual `/connect` succeeds.
- **Provider rate-limited** — `FeedError::RateLimited(retry_after)`; cloacina's retry policy honors `retry_after`.
- **Disk full / permission denied** — `FeedError::Storage`; alerts on next render.
- **Cursor corruption** — `meta.json` parse fails; template falls back to a "from now" cursor and emits a warning, no data loss but a gap in the archive.

### What this initiative does NOT decide

- The exact UI for showing feed state — that's I-0035 + the future TUI layout rework initiative.
- The watcher-rule DSL ("alert me when X"). This initiative ships the data; rules over the data are a follow-up.
- LLM-based summarization of feed contents. Data first.
- A SQLite indexer over the file tree. The disk store IS the v1 data layer; an indexer comes when query patterns warrant it.

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

**A. Stay pull-on-demand, tighten existing tools.** Rejected. Cross-integration aggregation (`/brief`) is unworkable on demand at scale (4–6 providers × seconds each). Watching spaces is impossible without a persistent fetch loop. Every assistant-shaped feature would have to invent its own cache.

**B. Push-only via webhooks where supported.** Rejected as v1 — coverage is uneven (Slack and GitHub support webhooks well; Calendar/Drive less so; Atlassian's webhook story is mixed). Forces a public callback URL or a hosted relay (recreates I-0037's "we own infrastructure" problem). v1 is polling via cloacina; webhooks land later as an alternate trigger over the same templates + disk layout.

**C. SQLite-backed normalized store with a query API (the original draft).** Rejected. Conflates "acquire data" with "answer questions about data." Forces every adapter into a normalization step, makes the storage layer schema-coupled, and adds a query surface before we know the queries. Disk + the agent's existing file tools cover v1 needs and let us add an indexer (SQLite or otherwise) later when query patterns are clear.

**D. Provider-owned local mirrors (e.g. Gmail's offline IMAP, Jira's REST cache).** Rejected. Each provider has its own opinion about "the local mirror"; we'd be writing a different abstraction per integration. The template model lets each provider express its own opinion *as a template*, in one consistent runtime.

**E. Roll our own scheduler.** Rejected. cloacina is already running in arawn for workflows and it solves catchup, retries, instance management, and audit trails. Per-feed tokio task with custom backoff would reinvent all four poorly.

**F. Embed in `arawn-integrations` directly.** Rejected. Mixing the imperative tool surface (existing `gmail_inbox_read` etc.) with the standing-feed runtime in one crate confuses ownership. New `arawn-feeds` crate keeps concerns separated; templates live alongside their integrations but the runtime is its own crate.

## Implementation Plan **[REQUIRED]**

Six phases, each a task. Phase 1 lands the runtime + cloacina wiring. Phases 2–4 add template families in parallel. Phase 5 hooks the data into I-0035. Phase 6 is the `/watch` UX.

### Phase 1 — Feed runtime + cloacina wiring (T-TBD, ~500 LOC)
New `arawn-feeds` crate. `FeedTemplate` trait + `TemplateParams` / `TemplateCtx` / `RunSummary` / `FeedError` types. Config loader that reads `[[feed]]` blocks from `arawn.toml` and registers each as a cloacina cron task. `meta.json` reader/writer at the feed_dir level. One stub template for tests. No real provider work yet — runtime + scheduler integration only.

### Phase 2 — Slack templates (T-TBD)
`slack/channel-archive` (append messages from a channel as JSONL, time-partitioned), `slack/my-mentions` (personal default, all channels), `slack/dm-archive` (per-user). Cursor: `latest_ts` per channel.

### Phase 3 — Gmail + Calendar templates (T-TBD)
`gmail/inbox-archive` (personal default), `gmail/sender-filter` (watched-sender scope), `gmail/label-archive` (watched label). `calendar/upcoming-archive` (today + N days, per-event JSON). Cursor: `historyId` for Gmail, `updated_min` for Calendar.

### Phase 4 — Jira + Confluence + Drive templates (T-TBD)
`jira/project-tracker` (issues + comments + history per issue), `jira/assignee-tracker` (personal default for `assignee = currentUser()`), `confluence/space-archive` (page list + bodies), `drive/folder-sync` (rsync-style mirror, native files on disk).

### Phase 5 — I-0035 wiring (T-TBD)
I-0035's briefing service rewrites to read from `~/.arawn/data/` instead of fanning out to providers. Action-item baseline reads from feed contents (e.g. find Slack messages under `slack/my-mentions/` since last brief). No new tools — uses existing `read` / `glob` / `grep`.

### Phase 6 — `/watch` slash command + feed UI (T-TBD)
TUI command to add/remove feeds without editing arawn.toml. `/watch slack channel-archive #design`-style ergonomics with picker support where the provider can enumerate (channels, projects, spaces). `feeds` slash command lists configured feeds with last-run + status. Depends on I-0036 layout decisions.

### Sequencing

1 first; then 2/3/4 in parallel; 5 once at least one template family ships; 6 last.

### Design decisions locked (2026-05-06)

1. **Cadence floor: 15 minutes.** No per-feed override. Sub-minute / sub-15-minute "real-time" needs (mention pings, etc.) are a different surface — webhooks/push, future initiative.
2. **Watch-space discovery: both paths.** TUI shows a picker when params are omitted; explicit name (`/watch slack channel-archive #design`) honored when provided. Agent does its own discovery by chaining existing list tools (e.g. `slack_channels_list`) → `/watch` with explicit name. No special agent-side picker needed.
3. **Catchup: cloacina-native.** Use `CronRecoveryService`'s default behavior (24h window, missed schedules replayed back-to-back). No per-feed override knob in our config — trust cloacina.
4. **Workstream scoping: feeds are global.** Workstreams are the *processing* layer — they read raw global feed data and refine it into per-workstream context. Acquisition layer (this initiative) is one shape; refinement is a separate concern.
5. **Feed definitions in DB, not config file.** New `feeds` table in `~/.arawn/arawn.db` (`id`, `template`, `params JSON`, `cadence`, `enabled`, timestamps). Source of truth for *what we're configured to fetch*. cloacina jobs are derived at boot + on `/watch`. `meta.json` per feed dir on disk still holds the cursor + last_run for forensics — disk is self-describing.
6. **Pure pull, unidirectional only — ever.** No bidirectional/writeback templates in scope, ever. The data dir is a one-way mirror.
7. **Disconnect = pause + preserve.** Disconnecting an integration pauses its feeds and preserves their data dirs. Reconnect resumes from the persisted cursor. Decommissioning a feed (deletes record + data) is a separate explicit user flow (`/feeds rm <id>` — Phase 6).
8. **No encryption at rest.** Data dir is plain files, file-system permissions only. The disk-as-data-layer model depends on the agent reading raw files via existing `read` / `grep` / `glob` tools; encryption would break that. High-trust personal-device posture is the assumed deployment.

### Open questions superseded by the locks above

(All 8 prior open questions are resolved. None remain blocking.)

1. **Cadence floor.** What's the minimum cadence we accept? Slack at 1-minute × 50 channels is 50 calls/min — close to rate limits. Lean: hard floor of 1min for personal feeds, 5min for watched-space; override needs explicit config flag.
2. **Watch-space discovery.** `/watch slack channel-archive` with a picker (we list channels you're in, you pick) vs `/watch slack channel-archive #design` with explicit name resolved at registration time. Lean picker for first-class providers, explicit fallback always available.
3. **Catchup policy default.** If you're offline for 3 days and an hourly feed has 72 missed runs, do we run them all, just the last one, or run with `catchup_max_misses = 3` collapsing into 3? Lean **collapse missed into 1 catch-up run by default** with `catchup_strategy: collapse | run-all | skip` per feed.
4. **Workstream scoping.** Feeds global, or scoped per workstream? Lean **global by default with optional `workstream` field** — most feeds are personal, but a "code" workstream might want a GitHub-repo feed isolated from a "personal" workstream's family-calendar feed.
5. **`arawn.toml` vs separate config file.** With `[[feed]]` blocks the toml will get long. Split feeds into `~/.arawn/feeds.toml` to keep `arawn.toml` focused on engine config? Lean **separate file**, auto-merged at load.
6. **Permission model for write-touching templates** (e.g. `drive/folder-sync` writes files to disk under `~/.arawn/data/` — fine; but a `drive/folder-sync direction = "push"` would write back to Drive). Lean **v1 is read/pull only** — writeback templates are explicitly out of scope until we have a permission story for "agent told me to mirror this folder back."
7. **What happens on integration disconnect.** Feeds fail fast (FeedError::Auth) and pause. Do we preserve the data or wipe it? Lean **preserve indefinitely** — the data dir is a journal; a disconnect doesn't invalidate history.
8. **Encryption at rest.** Tokens are already chacha20-poly1305 encrypted under `~/.arawn/tokens/`. Is the data dir encrypted too? Lean **no** for v1 (file-system permissions only, matching `arawn.db`); revisit if anyone asks.