---
id: t-e-side-effect-ledger-draft
level: task
title: "T-E: Side-effect ledger + draft/schedule-with-confirmation conversion"
short_code: "ARAWN-T-0449"
created_at: 2026-05-28T22:01:44.832147+00:00
updated_at: 2026-05-28T22:01:44.832147+00:00
parent: ARAWN-I-0062
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0062
---

# T-E: Side-effect ledger + draft/schedule-with-confirmation conversion

## Parent Initiative
[[ARAWN-I-0062]]

## Objective
The confirmation scenarios test that the agent **stages** a write-side action
(send-email, schedule-meeting) and waits for user confirmation before
"sending." With mocks in play, we need a way to assert "draft was prepared
with these fields" without actually shipping anything.

## Technical Approach
- Add a shared `SideEffectLedger` in `ProjectionFixtureBackend` — every mock
  client records `(tool_name, params)` for any write-side call to it.
- `UatGmailClient::send_draft`, `UatCalendarClient::create_event` etc. push to
  the ledger instead of "sending."
- Add a `MechanicalThresholds.staged_actions: Vec<StagedActionExpectation>`
  with fields like `tool_name`, `params_contain_substrings`. Check post-run
  against the ledger.
- `draft-with-confirmation`: agent reaches for `gmail_send` with a draft
  body that includes the agreed alignment language, but only after the
  confirm-before-side-effects guard fires. Ledger should record exactly one
  `gmail_send` call with the expected substrings.
- `schedule-with-confirmation`: analogous with `calendar_event_create`.

## Acceptance Criteria
- [ ] Side-effect ledger collects calls across mock clients.
- [ ] `staged_actions` mechanical check implemented + integrated into
      MechanicalCheckResult output.
- [ ] `draft-with-confirmation` agent stages a gmail draft via the live tool;
      mechanical asserts the staged call; judge completion ≥ 4/5.
- [ ] `schedule-with-confirmation` analogous.
- [ ] `angreal check workspace` + `arawn-tests` green.

## Dependencies
Depends on [[ARAWN-T-0445]], [[ARAWN-T-0446]], [[ARAWN-T-0447]].

## Status Updates
*To be added during implementation*
