---
id: t-a-required-param-schema-on
level: task
title: "T-A: Required param_schema() on FeedTemplate + impls for all templates"
short_code: "ARAWN-T-0421"
created_at: 2026-05-26T17:28:02.495906+00:00
updated_at: 2026-05-26T17:58:55.433460+00:00
parent: ARAWN-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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
   - `enum ParamKind { Text, Int, Bool, Path, List, Since, Enum(Vec<String>) }`
     - `List` = space/comma-separated input → JSON array (e.g. `include`/`exclude`).
     - `Path` is a `Text` variant the UI can treat specially (file picker hint);
       server-side it's just a string.
     - `Since` = the first-run backfill timestamp shared by 6 templates
       (gmail×3, slack×2, jira×2, drive/recent); accepts the `parse_since`
       grammar (`7d` / `2026-01-01` / RFC3339). Distinct kind so the form can
       apply that grammar + show duration help.
     - `Enum(Vec<String>)` = fixed allowed-value set. No current template uses
       it, but it's retained for future templates (user decision 2026-05-26).
     - `Int` carries no min/max — ranges live in `help`; `validate()` enforces.
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

## Acceptance Criteria

- [x] `ParamSpec` / `ParamKind` defined in `arawn-feeds`, with serde derives.
- [x] `FeedTemplate::param_schema()` is a required method (no default impl).
- [x] Every template registered in `default_registry()` implements it; workspace
      compiles.
- [x] `filesystem/folder` schema matches its actual params (root/recursive/
      include/exclude) with correct required-ness and defaults.
- [x] A test iterates the default registry and asserts each template's schema is
      self-consistent: keys unique, every `default` type-matches its `kind`
      (Bool→bool, Int→number, List→array, Text/Path/Since→string), and required
      params carry no default. **Empty schemas are valid** — several feeds are
      genuinely param-less (`slack/my-mentions`, `github/issues-and-prs`,
      `github/review-queue`) or all-optional (`stub/echo`, `github/notifications`).
- [x] `filesystem/folder` and at least one other multi-param template have their
      schema keys cross-checked against what `validate()` accepts (no param the
      form offers that validate rejects).
- [x] `angreal check workspace` + relevant unit tests pass.

## Dependencies
None — this is the root of the initiative. Blocks T-B, T-C, T-D.

## Risk Considerations
- Touching the trait forces all templates to change at once; keep each impl
  mechanical and close to its `validate()`.
- `validate()` remains the server-side source of truth; the schema is descriptive.
  The coverage test guards against drift but doesn't replace validation.

## Status Updates

**2026-05-26 — Implemented + tested.**
- Added `crates/arawn-feeds/src/param_schema.rs`: `ParamKind { Text, Int, Bool,
  Path, List, Since, Enum(Vec<String>) }` and `ParamSpec { key, label, kind,
  required, default, help }` with `required()`/`optional()`/`optional_no_default()`/
  `since()` constructors + `default_matches_kind()`. Serde on both. Re-exported
  from `lib.rs`.
- Added required `FeedTemplate::param_schema()` (no default impl) in `template.rs`.
- Implemented it for all 18 registered templates + the test-only `DummyTemplate`
  in `registry.rs`, declared next to each `defaults()`/`validate()`. Param-less
  feeds return empty vecs.
- Shared first-run `since` backfill declared via `ParamSpec::since()` across the 6
  templates that support it.
- Tests: registry-wide self-consistency, filesystem schema↔params cross-check,
  param-less empty-schema spot check, serde round-trips. `angreal check workspace`
  clean; `cargo test -p arawn-feeds` all green (133 + suites).

**Decisions:** `Since` added (real param), `Enum` kept per user. Acceptance
criterion corrected — empty schemas are valid (param-less feeds).