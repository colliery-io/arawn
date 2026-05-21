---
id: t-l-trim-slack-search-read-scope
level: task
title: "T-L: Trim Slack `search:read` scope reservation comment"
short_code: "ARAWN-T-0389"
created_at: 2026-05-21T14:53:31.955716+00:00
updated_at: 2026-05-21T14:53:31.955716+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# T-L: Trim Slack `search:read` scope reservation comment

## Parent Initiative

[[ARAWN-I-0053]]

## Objective

Reduce or remove the multi-line "Re-add when `slack_search` lands" comment
block in the Slack integration. Per operator decision (Tier 3 candidate 3.9):
since there is no Metis task for a `slack_search` template, the
multi-paragraph reservation comment is more cruft than signal. Implementation
behavior (`search:read` not requested) stays unchanged.

## Acceptance Criteria

- [ ] Shorten the comment at `crates/arawn-integrations/src/slack/integration.rs:19-23` to either a single line (e.g., `// search:read is intentionally absent — slack-morphism doesn't typed-expose search.messages yet`) or remove it entirely.
- [ ] Behavior unchanged: the scope list still does NOT include `search:read`.
- [ ] If `slack_search` ever gets filed as a Metis task in the future, the scope addition is trivially obvious and can be added then.

### Validation

- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `angreal check clippy` clean for arawn-integrations.

## Implementation Notes

### Technical Approach

Pure comment edit. No code or behavior change.

### Dependencies

None.

### Risk Considerations

Negligible.

## Verification

- `angreal check workspace`
- `angreal check clippy`

## Status Updates

*To be added during implementation*
