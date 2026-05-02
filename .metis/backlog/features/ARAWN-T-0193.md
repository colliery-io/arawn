---
id: readme-end-user-getting-started
level: task
title: "README + end-user getting-started docs"
short_code: "ARAWN-T-0193"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-02T04:35:24.116155+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

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

### 2026-05-02 — Implemented

**Files created:**
- `README.md` at repo root — what arawn is, status (alpha), 30-second quickstart with Groq, repo layout, CLI summary, dev tasks via angreal, link to vision.
- `docs/book.toml` — mdbook scaffold so `angreal docs serve` works.
- `docs/src/SUMMARY.md` — book TOC.
- `docs/src/intro.md` — book landing page.
- `docs/src/getting-started.md` — full walkthrough: prereqs, build, three provider configs (Groq, Ollama Cloud, local Ollama), start server, open TUI, send first message, CLI one-shot mode, troubleshooting table for the failure modes T-0190's warmup and T-0191's error chain expose, plus embedder fallback warning, TUI connection failures, and missing API key.
- `.gitignore` — added `/docs/book/` for mdbook build output.

**Verified:**
- `angreal docs build` succeeds — mdbook scaffolding is wired.
- README renders cleanly as plain markdown (no extensions used).

**Acceptance criteria status:**
- [x] `README.md` at repo root with what/install/prereqs/quickstart/docs link.
- [x] `docs/src/getting-started.md` with provider picker, config, server start, TUI, first message, troubleshooting.
- [x] Groq and Ollama Cloud shown as concrete examples (also covered local Ollama as a third option).
- [x] Both files render in GitHub markdown (mdbook also builds the docs/ tree).

**Cross-references for follow-ups:**
- T-0194 will land `arawn init` — when that ships, update getting-started step 2 to lead with `arawn init` instead of hand-writing TOML.
- T-0195/T-0197 will fill in the `/help`, `/remember`, `/memory` references currently flagged as work-in-progress.
- T-0198 will land workflow examples — getting-started already links forward.