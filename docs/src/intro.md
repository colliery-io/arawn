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

If you've never run arawn before, start with [your first chat
session](./tutorials/first-chat.md) — about ten minutes from `git
clone` to a working chat session. Then [your first
workstream](./tutorials/first-workstream.md) covers the end-to-end
workstream + extraction + `signal_search` flow.

If you have a specific goal in mind (connect a provider, lock down
permissions, debug an OAuth failure), the [how-to
guides](./how-to/index.md) are recipe-style and self-contained.

If you want to look something up (a CLI flag, a config key, an agent
tool), [reference](./reference/index.md) is the catalog.

If you want to understand *why* arawn works the way it does,
[explanation](./explanation/index.md) covers concepts, design
decisions, and trade-offs.

For the project goals and roadmap, see the vision in `.metis/vision.md` and
active initiatives under `.metis/initiatives/`.

## Status

Arawn is **alpha**. APIs, config schema, and CLI flags will change. Don't
build production workflows on it yet.
