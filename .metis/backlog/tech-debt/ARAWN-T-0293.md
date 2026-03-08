---
id: add-cli-subcommand-tests-ask
level: task
title: "Add CLI subcommand tests (ask, memory, notes, auth, session)"
short_code: "ARAWN-T-0293"
created_at: 2026-03-08T20:21:15.462112+00:00
updated_at: 2026-03-08T20:21:15.462112+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Add CLI subcommand tests (ask, memory, notes, auth, session)

## Objective

The CLI has 68 tests but several subcommands (`ask`, `memory`, `notes`, `auth`, `session`) have zero test coverage. Add tests for argument parsing, validation, and basic execution paths for each untested subcommand.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (important for user experience)

### Technical Debt Impact
- **Current Problems**: Five CLI subcommands have no tests. Argument parsing bugs, missing required args, and invalid input handling are all untested.
- **Benefits of Fixing**: Catches CLI regressions, validates user-facing argument parsing.
- **Risk Assessment**: Medium — CLI is the primary user interface; broken commands directly affect UX.

## Acceptance Criteria

- [ ] `ask` subcommand: argument parsing, missing required args error
- [ ] `memory` subcommand: list/add/remove argument parsing
- [ ] `notes` subcommand: argument parsing and validation
- [ ] `auth` subcommand: login/logout/status argument parsing
- [ ] `session` subcommand: list/show/delete argument parsing
- [ ] `cargo test -p arawn` passes
- [ ] `angreal check clippy` clean

## Implementation Notes

### Technical Approach
- Add tests in `crates/arawn/src/commands/` modules for each subcommand
- Focus on clap argument parsing (use `try_parse_from`)
- Test error messages for missing/invalid args
- Test that valid args produce expected command structs

### Files
- `crates/arawn/src/commands/ask.rs` (or wherever each subcommand lives)
- `crates/arawn/src/commands/memory.rs`
- `crates/arawn/src/commands/notes.rs`
- `crates/arawn/src/commands/auth.rs`
- `crates/arawn/src/commands/session.rs`

## Status Updates

*To be added during implementation*