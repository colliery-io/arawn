---
id: readme-end-user-getting-started
level: task
title: "README + end-user getting-started docs"
short_code: "ARAWN-T-0193"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-02T00:00:00+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# README + end-user getting-started docs

## Objective

Repo root has zero end-user docs. A non-developer who clones the repo cannot figure out what arawn is, how to install it, how to set an API key, or how to run their first conversation. Add a README and a short getting-started guide that takes a user from `git clone` to a working `arawn tui` session.

## Type / Priority
- Feature (documentation)
- P1 — Blocker. Without this, every other improvement is invisible.

## Acceptance Criteria

- [ ] `README.md` at repo root covering: what arawn is in 2 sentences, install/build (`cargo build --release` + binary location), prerequisites (Rust toolchain, LLM provider API key), 30-second quickstart, link to `docs/`.
- [ ] `docs/getting-started.md` walking through: pick a provider, get an API key, write `~/.arawn/arawn.toml`, start the server, open the TUI, send a first message. Include troubleshooting for the top 3 failure modes (missing API key, model unavailable per T-0190 warmup, embedder model not present).
- [ ] Both Groq and Ollama Cloud shown as concrete examples (not "configure your provider").
- [ ] Both files render in GitHub markdown.

## Implementation Notes

- Don't try to document everything — getting-started + a "where to learn more" pointer is enough. Reference docs grow later.
- Use `~/.arawn/arawn.toml` (the user's working config) as the example template.
- The angreal `docs:serve` task exists — check whether `mkdocs` or similar is already wired for longer-form docs.
- Coordinate with T-0194 (config UX) — the troubleshooting section should reflect the actual error messages that ticket lands.

## Status Updates

*To be added during implementation*
