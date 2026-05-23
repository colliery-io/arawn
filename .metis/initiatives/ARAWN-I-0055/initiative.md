---
id: capability-driven-tool-filter-fix
level: initiative
title: "Capability-driven tool filter — fix T-0394 structurally"
short_code: "ARAWN-I-0055"
created_at: 2026-05-22T16:35:24.788690+00:00
updated_at: 2026-05-22T16:35:24.788690+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: true
estimated_complexity: M
initiative_id: capability-driven-tool-filter-fix
---

# Capability-driven tool filter — fix T-0394 structurally

## Context

The UAT scenario `schedule-with-confirmation` (backlog bug ARAWN-T-0394) failed at I-0053 close with the agent hallucinating a `gcal` shell command instead of calling `calendar_upcoming`. At I-0054 close (pure restructuring, no behavior change) it passed cleanly. Investigation revealed the failure mode is **structural, not just flaky LLM**:

1. `filter_tools_for_context` in `crates/arawn-engine/src/query_engine.rs` gates tools by `ToolCategory` using **keyword scans over the latest user message**.
2. On iteration 1 of turn 1, `session.messages().len() ≤ 2` so the filter early-returns all tools — agent sees `calendar_upcoming`. Sometimes it picks the right tool; sometimes it picks `shell("gcal")` (model decision error). That's the flaky surface.
3. From iteration 2 onward, the filter activates. Calendar tools are tagged `ToolCategory::Web` (`calendar/tools.rs:96,182,319`). The Web keyword set (`http, url, web, search, fetch, api, github, google`) doesn't match a calendar prompt — so calendar tools get **dropped from the catalog**.
4. Meanwhile `Ceremony` *is* active (prompt contains "week", "today", etc.), so daily/weekly tools stay visible. Agent re-scans, sees only ceremony tools, and tries `weekly_run` — wrong tool, wrong week.
5. **No recovery path.** Once iteration-1 misfires, calendar tools are gone for the rest of the turn.

The filter is load-bearing on small models — 102 registered tools × ~250 tokens per definition ≈ 25K tokens for the full catalog. For a 32K-context model (`gemma4:31b-cloud`) that's 80% of the window. Removing the filter is not an option for small models.

So we keep the filter and fix it.

## Goals & Non-Goals

**Goals:**

- Eliminate the structural recovery-path failure: once an integration is connected, its tools are always visible to the agent regardless of user message text or iteration number.
- Re-categorize integration tools (calendar/gmail/drive/slack/atlassian/github) off the `Web` bucket and into per-service categories aligned with prior art (LangChain Toolkits, MCP servers, OpenAI Actions).
- Audit + expand keyword sets for non-integration categories so legitimate prompts route correctly.
- Promote `Workstream` + `Memory` to always-on — both surface "ambient" capabilities the agent should never lose access to mid-turn.
- For large-context models (≥100K), bypass the filter entirely. The catalog cost is tractable and the brittleness isn't worth it.
- Convert ARAWN-T-0394 to a regression test asserting `calendar_upcoming` survives the original failure mode.

**Non-Goals:**

- Removing the filter entirely. The small-model case still needs it.
- Re-categorizing non-integration tools (signal_*, feed_*, ceremony tools, etc.). Those are already sensibly bucketed.
- MCP tool categorization. MCP tools have unknown category and are passed through the filter unconditionally today — keep that behavior.
- Re-tuning model defaults. The model choice (gemma4:31b-cloud) is a separate question.

## Detailed Design

### Layer 1 — Per-service integration categories

Add to `arawn-tool::ToolCategory`:

```rust
pub enum ToolCategory {
    // ... existing variants ...
    Calendar,
    Gmail,
    Drive,
    Slack,
    Atlassian,
    GitHub,
}
```

Move each integration tool's `category()` from `ToolCategory::Web` to its per-service variant. Files touched:

- `crates/arawn-integrations/src/calendar/tools.rs` — 3 tools.
- `crates/arawn-integrations/src/gmail/tools.rs` — N tools (audit during T-A).
- `crates/arawn-integrations/src/drive/tools.rs` — N tools.
- `crates/arawn-integrations/src/slack/tools.rs` — 6 tools (`SlackListChannelsTool`, etc.).
- `crates/arawn-integrations/src/atlassian/tools.rs` — N tools.
- `crates/arawn-integrations/src/github/tools.rs` — N tools (if any registered — check).

`Web` category retains only `web_fetch`, `web_search`.

### Layer 2 — Capability-driven inclusion for integration categories

Rewrite the integration branch of `filter_tools_for_context`. Currently the function does keyword scans for every category. After this change:

```rust
// Pseudo-code
let connected = (config.prompt_context.integration_capabilities)();  // already exists

for tool in all_tools {
    let cat = registry.get(&tool.name).map(|t| t.category());
    let include = match cat {
        Some(ToolCategory::Core | ToolCategory::Utility) => true,
        Some(ToolCategory::Calendar) => connected.contains("calendar"),
        Some(ToolCategory::Gmail) => connected.contains("gmail"),
        // ... etc per service
        Some(other) => active_categories.contains(&other),  // keyword-driven for non-integrations
        None => true,  // unknown (MCP) — pass through
    };
    // ...
}
```

The `integration_capabilities` provider already exists in `query_engine.rs:654` and is queried fresh each turn. No new plumbing required; we just consume it differently.

### Layer 3 — Audit + expand non-integration keyword sets

Walk each non-integration category's keyword list and add missing terms:

- `Ceremony`: add "agenda", "morning", "afternoon", "tomorrow", "yesterday", "this week", "next week".
- `Web`: narrow now that integrations are out — keep `http, url, web, search, fetch, api`. Drop `github, google` (they were proxies for integration tools that are now capability-gated).
- `Plan`: keep "plan", "planning"; add "design", "approach", "strategy".
- `Task`: keep "task", "todo", "background"; add "queue", "schedule" (note: `schedule` ALSO triggers calendar capability — that's fine, both routes are correct).
- `Memory`: keep "remember", "recall", "memory", "forget"; (will be moot after Layer 4 promotion).
- `Agent`: keep "agent", "delegat"; add "subagent", "spawn".
- `Workstream`: (will be moot after Layer 4 promotion).

Add a negative-test per category: assert each keyword routes correctly AND assert that omitting all keywords drops the category.

### Layer 4 — Always-on promotions + model-context bypass

**4a — Promote `Workstream` + `Memory` to always-on:** treat both like `Core`/`Utility`. The agent should never lose `workstream_switch` (need to navigate) or `memory_recall` (need to recall). Both are pure ambient capabilities — small surface, high frequency, no semantic reason to filter.

**4b — Model-context-aware bypass:** the filter is only justified for small models. Add a guard at the top of `filter_tools_for_context`:

```rust
const FILTER_BYPASS_CONTEXT_THRESHOLD: u32 = 100_000;
if model_limits.context_window >= FILTER_BYPASS_CONTEXT_THRESHOLD {
    return all_tools.to_vec();
}
```

For Claude/GPT-4 (≥128K), this means the full catalog ships every turn. For gemma4:31b-cloud (32K) the existing filter applies. The boundary is conservative — even at 100K, 25K tokens of catalog leaves 75K for conversation, which is comfortable.

### Layer 5 — T-0394 regression test

Convert ARAWN-T-0394 from a backlog bug to a closed task whose artifact is a test asserting the structural fix:

```rust
#[test]
fn calendar_tools_survive_filter_when_capability_present_regardless_of_keywords() {
    // Build a registry with calendar tools.
    // Build a session with messages.len() > 2 (filter active).
    // User message contains NO calendar keywords ("Bob's open Tue mornings...").
    // capabilities = ["calendar"].
    // Assert: filtered tools include "calendar_upcoming".
}
```

## Alternatives Considered

- **Drop the filter entirely.** Considered and rejected for the small-model case (25K tokens of catalog = 80% of a 32K window).
- **Hand-tuned keyword expansion alone (no capability gating).** Rejected — the keyword approach is fundamentally brittle. The user can talk around any keyword set. Capability is authoritative.
- **Single `ToolCategory::Integration` bucket.** Considered briefly. Rejected — prior art uniformly uses per-service grouping (LangChain Toolkits, MCP servers, OpenAI Actions). Per-service categories also future-proof: when a new integration lands, it gets its own keyword/capability symmetry without polluting the others.
- **Score-based ranking instead of binary include/exclude.** More flexible but adds a tunable. The current binary approach has the virtue of clarity; a ranking system would be a bigger redesign than I-0055 warrants.

## Implementation Plan

Six tasks, sequenced:

- **T-A** — Add per-service `ToolCategory` variants + re-categorize all integration tool impls. Mechanical sweep across `crates/arawn-integrations/src/*/tools.rs`. Verify `cargo build --workspace --release` clean and 1,758 lib tests still pass.
- **T-B** — Capability-driven filter branch for integration categories. Edit `filter_tools_for_context` in `query_engine.rs`. Add positive + negative unit tests (capability present → tool visible; capability absent → tool dropped).
- **T-C** — Audit + expand non-integration keyword sets. Add negative-test per category.
- **T-D** — Promote `Workstream` + `Memory` to always-on (Layer 4a).
- **T-E** — Model-context-aware bypass (Layer 4b).
- **T-F** — T-0394 regression test (Layer 5). Closes ARAWN-T-0394.

Close gate: full `angreal test uat` (13/13 mechanical PASS) + `angreal test uat-judge` (13/13 judge PASS).

## Exit Criteria

- [x] All six tasks landed (T-A through T-F).
- [x] Integration tool catalogs are now per-service (`Calendar`, `Gmail`, `Drive`, `Slack`, `Atlassian`, `GitHub`).
- [x] `filter_tools_for_context` includes integration tools iff the corresponding capability is in the connected set — no keyword scan for integration categories.
- [x] Workstream + Memory tools are always visible.
- [x] Models with `context_window ≥ 100_000` bypass the filter entirely.
- [x] T-0394 regression test asserts the structural fix and is part of the workspace lib test suite.
- [x] `cargo check --workspace` clean, `cargo build --workspace --release` clean.
- [x] `cargo test --workspace --lib` green — **1,785 lib tests pass** (1,758 baseline + 27 new across T-B/C/D/E/F).
- [x] `angreal test uat` green — **13/13 mechanical PASS** (2026-05-22, 62-min run, log `/tmp/uat-i0055-mech.log`, data dir `/tmp/arawn-uat-20260522-191854`).
- [x] `angreal test uat-judge` green — **13/13 judge PASS**, 0 FAIL.

## Closing Summary

Six tasks landed in one Ralph loop session. Every UAT scenario passes mechanical + judge.

The original symptom (T-0394 — `schedule-with-confirmation` flaking on `gemma4:31b-cloud`) is now structurally blocked: in this UAT run the agent reached the right behavior via `workstream_switch personal` → `feed_search ×3` → propose-slot → ask-confirm (judge `completion=4/5, quality=4/5`). Note that the agent used `feed_search` rather than `calendar_upcoming` directly — different path, same outcome. The point of the T-B fix isn't that the model picks `calendar_upcoming` every time; it's that when it DOES pick it, the tool survives iter-2+ regardless of user message text. The unit tests in T-F pin that contract structurally; the UAT shows the user-visible scenario works end-to-end.

## Related

- ARAWN-V-0001 — vision.
- ARAWN-T-0394 — backlog bug; closed with regression test artifact at I-0055 T-F.
- ARAWN-I-0053 — predecessor (cruft removal) that surfaced the bug.
- ARAWN-I-0054 — predecessor (module restructuring) whose UAT re-run confirmed the failure is structural, not behavioral-regression.
