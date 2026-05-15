---
id: arawn-doctor-cli-rpc-first-class
level: task
title: "`arawn doctor` CLI + RPC — first-class diagnostics surface"
short_code: "ARAWN-T-0271"
created_at: 2026-05-15T14:12:04.426122+00:00
updated_at: 2026-05-15T14:12:04.426122+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0044
---

# arawn doctor CLI + RPC

## Tier
Tier 1 — small win, no architectural blockers.

## Reference
`/tmp/openhuman/src/openhuman/doctor/` — RPC `doctor.*` + CLI subcommand. Checks config validity, workspace state, daemon health, model reachability.

## Goal
`arawn doctor` runs a battery of checks and prints a structured report; failures exit non-zero. Same checks exposed as RPC for programmatic use.

## Acceptance
- New `crates/arawn/src/cli/doctor.rs` + supporting `arawn-engine` module if shared logic warrants.
- Checks at minimum:
  - Config file exists and parses
  - Data dir exists and is writable
  - Configured LLM endpoint reachable + auth works (HEAD or trivial completion)
  - Memory store opens without error
  - At least one integration: token present + token store decryptable (skip per integration when not configured)
  - Plugin directory scans without parse errors
- Output: human-readable by default, `--json` for machine.
- Each check named, each result Pass / Fail / Skip with one-line reason.
- Unit tests per check; integration test that runs `arawn doctor` against a synthesised data dir.
