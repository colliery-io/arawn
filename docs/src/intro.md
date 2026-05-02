# Arawn

A self-hosted personal agentic assistant in a single Rust binary.

Arawn runs a local WebSocket server that hosts an agent loop, talks to any
OpenAI-compatible LLM (Groq, Ollama Cloud, local Ollama, OpenAI, vLLM,
LM Studio, ...), and exposes:

- A **TUI client** for interactive sessions.
- A **one-shot CLI** for scripting (`arawn "draft a commit message"`).
- A **workflow runtime** for scheduled background pipelines.
- A **persistent memory system** (global + per-workstream knowledge bases).
- A **plugin system** with hot reload.

## Where to start

If you've never run arawn before, start with the
[Getting Started](./getting-started.md) walkthrough. It takes about ten
minutes from `git clone` to a working chat session.

For the project goals and roadmap, see the vision in `.metis/vision.md` and
active initiatives under `.metis/initiatives/`.

## Status

Arawn is **alpha**. APIs, config schema, and CLI flags will change. Don't
build production workflows on it yet.
