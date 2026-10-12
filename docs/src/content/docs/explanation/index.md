---
title: "Explanation"
---
*Understanding-oriented. The "why" behind arawn — concepts, design decisions, trade-offs.*

Explanation pages answer "why does arawn work this way?" They're not how-tos (no steps) and not reference (no lookup tables). They give you the mental models that make the rest of the system coherent.

## The system as a whole

- **[What is arawn?](what-is-arawn/)** — the vision; agent loop in one paragraph; self-hosted thesis.
- **[The agent loop](the-agent-loop/)** — how a turn flows: prompt → LLM → tool call → permission check → execution → result → next iteration.

## Data model and "the three layers"

- **[The three-layer data model](three-layer-data-model/)** — why feeds, projections, and palaces are different layers.
- **[Feeds](feeds/)** — what a feed is, when to make one, why local-first.
- **[Projections](projections/)** — why flat, when to read which type.
- **[Palaces](palaces/)** — the palace metaphor; lifecycle; ADR pointers.
- **[Extraction](extraction/)** — the 4-stage CoT, why two tag fields, UAT war story.
- **[Steward](steward/)** — bounded blast radius + journal + propose-vs-apply.

## Organizing principles

- **[Lenses](lenses/)** — what they are; when to create one vs. scratch.
- **[Identity by lens](identity-by-lens/)** — why arawn's persona is a lens attribute; assistant vs. coding profile.
- **[Memory design](memory-design/)** — global vs. lens; FTS vs. vector; scope-locking.
- **[Permission model](permission-model/)** — deny > allow > ask; plan-mode philosophy.
- **[Workflows](workflows/)** — when to workflow vs. converse; why a DAG; cloacina rationale.
- **[Ceremonies](ceremonies/)** — morning brief / weekly / retro; the "watch, check, summarize, nudge" thesis.

## Integrations

- **[Integrations overview](integrations-overview/)** — how arawn connects to external providers (Slack, Google, Atlassian, GitHub); credential model; per-provider scopes.
- **[OAuth primer](oauth-primer/)** — why OAuth, BYO app vs shared client, token encryption, refresh flow.

## When explanation isn't what you need

- If you want to learn from zero, see **[Tutorials](../tutorials/)**.
- If you want a recipe for a specific task, see **[How-to](../how-to/)**.
- If you want to look something up, see **[Reference](../reference/)**.
