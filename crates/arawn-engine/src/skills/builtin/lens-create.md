---
name: lens-create
description: "Use when the user asks to create a new lens or says things like 'make a lens for ...', 'add a new lens', '/lens new', '/lens create'. Walks the user through producing an initial tag ontology before creating."
user_invocable: true
---

# Lens Create

A lens is an isolated scope for one thing the user tracks: a project, a person they collaborate with, a hobby, an initiative. Each lens owns its own KB and a **declared tag ontology** (the closed list of tags the extractor is allowed to attach to entities — per ADR-0004). The ontology is required at creation.

This skill is the agent-side playbook for the create flow. Tools stay atomic; you do the conversational work.

## Flow

The flow has three phases. Don't skip them.

### 1. Ask for the description

Before calling any tool, get a clear description from the user. If they kicked off with `/lens create` and no other context, ask:

> "What is this lens for? Tell me what topics, decisions, people, or systems you want to track here. Anything explicitly out of scope?"

Push back if the description is one word. You need enough detail for the propose step to produce useful tags. A good description is 1–3 sentences naming concrete things (people, projects, systems, recurring artifacts).

### 2. Propose + confirm the ontology

Call `lens_propose_ontology(description)`. It returns `{ tags: [...], rationale: "..." }`. Show the proposal to the user — both the tag list and the rationale.

Then ask:

> "Here's what I'd suggest tracking: `[tag1, tag2, ...]`. Anything you'd add or remove?"

Iterate as needed. The user might want to:
- Add a specific project/person name you missed.
- Drop a tag that doesn't fit their mental model.
- Replace a generic tag with a more specific one.

You can call `lens_propose_ontology` again with a refined description if the user wants a fresh start. But usually the better move is to take their corrections directly and amend the list yourself.

The ontology should land at 5–12 tags. Don't fight the user if they want it bigger or smaller — they know their domain.

### 3. Confirm name + create

Ask for the lens name (or propose one based on the description — slug form, lowercase with dashes):

> "What should I call this lens? (Suggestion: `<your-slug>`)"

When you have name + description + final tag list, call:

```
lens_new(
  name: "<slug>",
  description: "<user's description>",
  tags_ontology: [<final tag list>]
)
```

Report the outcome briefly:

> "Created `<name>`. Bind feeds to it with `lens_bind` when ready — chat reads `signal_*` across every lens, so you don't need to 'switch in'."

## When NOT to use this skill

- The user already specified all three pieces (name + description + ontology) in a single message. Just call `lens_new` directly.
- The user is creating a lens from a scripted source (UAT fixtures, automation). Those paths populate the ontology table directly without going through the agent flow.

## Failure modes to watch

- `lens_new` rejects an empty `tags_ontology`. If the user insists on starting with no tags, push back: the ontology is what makes `lens_dust` / `signal_query` work. Suggest 2–3 minimum.
- The tag-promoter steward subroutine will surface new ontology candidates as the lens gets used. You don't need to make the initial list complete — just enough to bootstrap.
- If `lens_propose_ontology` returns tags that feel generic (`general`, `notes`, `stuff`), call it again with a more specific description or amend by hand. Generic tags defeat clustering.

## Related tools

- `lens_propose_ontology(description)` — LLM-backed; returns `{tags, rationale}`.
- `lens_new(name, description, tags_ontology)` — atomic create; ontology required.
- `lens_show(name?)` — surfaces the ontology of an existing lens; useful for "what tags can I use?" questions later.
- `lens_apply <id>` — accepts a `tag-promoter` proposal to grow the ontology over time.
- `lens_rollback <id>` — undoes any ontology change.
