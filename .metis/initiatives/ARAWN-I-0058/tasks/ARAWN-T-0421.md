---
id: t-a-required-param-schema-on
level: task
title: "T-A: Required param_schema() on FeedTemplate + impls for all templates"
short_code: "ARAWN-T-0421"
created_at: 2026-05-26T17:28:02.495906+00:00
updated_at: 2026-05-26T17:28:02.495906+00:00
parent: ARAWN-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0058
---

# T-A: Required param_schema() on FeedTemplate + impls for all templates

## Parent Initiative

[[ARAWN-I-0058]]

## Objective

Give `FeedTemplate` a machine-readable description of its parameters, and make
it **required** so the compiler refuses any template that hasn't described its
config. This is the foundation the `/watch` modal renders from. The trait change
and all template implementations land together — there is no default impl, so
the workspace will not compile until every template is covered.

## Type
Feature (foundational) — `arawn-feeds`.

## Technical Approach

1. **New types in `arawn-feeds`** (next to the trait, e.g. `template.rs` or a
   new `param_schema.rs`):
   - `enum ParamKind { Text, Bool, Int, Path, Enum(Vec<String>), List }`
     - `List` = space/comma-separated input → JSON array (e.g. `include`/`exclude`).
     - `Path` is a `Text` variant the UI can treat specially (file picker hint);
       server-side it's just a string.
   - `struct ParamSpec { key: String, label: String, kind: ParamKind, required: bool, default: Option<serde_json::Value>, help: String }`
   - Both `Serialize`/`Deserialize` (they cross the RPC boundary in T-B).
2. **Required trait method**: add `fn param_schema(&self) -> Vec<ParamSpec>;`
   to `FeedTemplate` (NO default body).
3. **Implement for every template.** Inventory the registered templates from
   `default_registry()` and implement `param_schema()` for each, keeping the
   declaration adjacent to that template's `validate()` so they stay in sync.
   For `filesystem/folder`: `root` (Path, required), `recursive` (Bool, default
   true), `include` (List, default `["**/*"]`), `exclude` (List, default excludes).
4. **Audit `ParamKind` coverage** while implementing — if a template has a param
   that doesn't fit the initial kinds (structured/nested), widen the enum here
   (resolves I-0058 open question #2).

## Acceptance Criteria

- [ ] `ParamSpec` / `ParamKind` defined in `arawn-feeds`, with serde derives.
- [ ] `FeedTemplate::param_schema()` is a required method (no default impl).
- [ ] Every template registered in `default_registry()` implements it; workspace
      compiles.
- [ ] `filesystem/folder` schema matches its actual params (root/recursive/
      include/exclude) with correct required-ness and defaults.
- [ ] A test iterates the default registry and asserts each template returns a
      non-empty schema whose `default` values are consistent with the declared
      `kind` (e.g. a `Bool` default is a JSON bool), and whose keys are unique.
- [ ] For at least the templates with a known fixed param set, a test asserts the
      schema keys match what `validate()` accepts (no param the form offers that
      validate rejects, and vice-versa where enumerable).
- [ ] `angreal check workspace` + relevant unit tests pass.

## Dependencies
None — this is the root of the initiative. Blocks T-B, T-C, T-D.

## Risk Considerations
- Touching the trait forces all templates to change at once; keep each impl
  mechanical and close to its `validate()`.
- `validate()` remains the server-side source of truth; the schema is descriptive.
  The coverage test guards against drift but doesn't replace validation.

## Status Updates

*To be added during implementation*