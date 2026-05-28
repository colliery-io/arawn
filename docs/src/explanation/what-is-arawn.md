# What is arawn?

*Explanation. The vision, the agent loop in one paragraph, and the self-hosted thesis.*

## In one sentence

arawn is a lightweight, self-hosted personal agentic assistant that runs scheduled tasks, monitors channels (email, Slack, calendar, tickets, GitHub), surfaces what needs your attention, and provides an interactive chat interface — all from a single Rust binary with a minimal resource footprint.

## What it's for

The vision is *"a personal agentic assistant that helps you stay organized and on top of life. arawn watches, checks, summarizes, and nudges — so you don't have to keep everything in your head."*

In practice:

- **Watch.** Connect Gmail, Slack, Calendar, Atlassian, GitHub. arawn mirrors slices of those services to local disk on a cadence — your inbox, the channels you participate in, your assigned tickets, your review queue.
- **Check.** When you open arawn (or when a ceremony fires), the agent has already seen what's new. You don't trigger a fetch; the fetch already happened.
- **Summarize.** The agent reads the mirrored data with `Read`/`Glob`/`Grep` and answers questions like "what did I miss in #design today?" or "what's on my plate?"
- **Nudge.** Daily, weekly, and retro ceremonies produce structured tablets. Detectors surface stale priorities, neglected lenses, calendar conflicts. Todos from one ceremony roll into the next.

## The self-hosted thesis

arawn runs on your machine. Tokens, mirrored data, knowledge bases — everything lives under `~/.arawn/`. There's no upstream summarizer, no cloud index, no shared inference endpoint that sees your inbox. The only network traffic is to your LLM provider (which you choose) and to the provider APIs you've authorized.

This is a deliberate choice. The same vision can be built as a SaaS product, but:

- Personal life data (calendar, DMs, ticket history) is sensitive in ways that don't map cleanly onto SaaS trust models.
- Your local machine has compute to spare, especially for retrieval-augmented chat — most token cost is in the LLM, which is the one piece that already lives outside.
- Single-binary self-host keeps the surface small. No cluster to operate, no auth model to manage, no multi-tenant isolation to get wrong.

The trade-off: you do operate it. Most of arawn's user-facing complexity (config, OAuth setup, the data directory) is the cost of self-host.

## The agent loop, briefly

Every turn:

```
User message
   │
   │   System prompt assembly:
   │   - identity (always assistant persona)
   │   - relevant global memory entries
   │   - available tools
   │
   ▼
LLM call
   │
   │   LLM emits text + zero-or-more tool calls.
   │
   ▼
For each tool call:
   permission check  →  execute (sandbox if shell)  →  result back to LLM
   │
   ▼
Next iteration, or final response.
```

[The agent loop explanation](./the-agent-loop.md) covers the details.

## The three-layer data model

Most of arawn's value is in how it organizes external data:

```
Feeds        →  raw bytes mirrored from upstream
Projections  →  typed, searchable rows in a single SQLite DB
Palaces      →  per-lens knowledge graph of typed entities + relations
```

Each layer answers a different kind of question. [The three-layer data model explanation](./three-layer-data-model.md) covers the rationale.

## What it isn't

- **Not a coding tool.** The default persona is the personal-assistant role. There's an opt-in `coding` lens profile for engineering work, but arawn isn't aimed at being a Claude Code / Cursor replacement.
- **Not multi-user.** Single-user only. Your machine, your data, your provider keys.
- **Not a chat-only tool.** Watchers and ceremonies run autonomously. Conversations are one of several surfaces.
- **Not cloud-resident.** There's no arawn server in the cloud. The "server" in `arawn serve` is your local Rust process.
- **Not opinion-free.** arawn has views — lenses as the organizing primitive, the steward as a proactive curator, ceremonies as scheduled reflection. You're welcome to disable any of those, but they're the default shape.

## Status

arawn is **alpha**. APIs, config schema, and CLI flags change between commits. The single-user-self-hosted thesis is settled; the surface around it is still moving.

## Related

- [The agent loop](./the-agent-loop.md)
- [The three-layer data model](./three-layer-data-model.md)
- [Lenses](./lenses.md)
- [Identity by lens](./identity-by-lens.md)
- [Ceremonies](./ceremonies.md)
