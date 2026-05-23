---
id: facility-system-ontology-and
level: specification
title: "Facility System — Ontology and Definition Requirements"
short_code: "ARAWN-S-0003"
created_at: 2026-04-17T13:17:42.816745+00:00
updated_at: 2026-04-17T13:17:42.816745+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Facility System — Ontology and Definition Requirements

## Overview

Arawn is an agent. To be useful, it must act on behalf of a user in the world outside itself — scheduling meetings, managing a task list, sending a message, pushing a notification. The set of things arawn *supports doing* is not a random collection of tools; it is a small, deliberate set of **classes of activity** — a task-list facility, a scheduling facility, a messaging facility, a push facility, and so on.

This specification defines the **Facility System**: the host-side machinery that names those classes of activity, lets multiple concrete implementations compete to satisfy them, and governs how new classes are added over time.

Out of scope: the individual Providers (Google Workspace, Slack, iCal, ...) that implement Facilities. Those are covered by separate initiatives and plugins.

## Ontology

These terms are **locked**. Every design document, code module, and docstring from this point forward uses them consistently.

### Facility

A named, well-bounded class of activity arawn supports. Abstract; describes *what* is available, not *how*.

Examples: `TodoListManagement`, `CalendarScheduling`, `ExternalMessaging`, `PushNotification`.

A Facility is realized as a Rust trait annotated with `#[fidius::plugin_interface(version = N, ...)]`. The trait is the contract; fidius's ABI-stable macros turn the trait into a loadable dylib interface and emit an `interface_hash` so drift between the host's trait and a Provider's compiled view is detected mechanically at plugin-load time. The trait itself is the schema — there is no separate hand-maintained wire-schema document.

### Provider

A concrete implementation of one or more Facilities. `GoogleWorkspaceProvider` implements `TodoListManagement` + `CalendarScheduling`. `SlackProvider` implements `ExternalMessaging` + `PushNotification`. A Provider is code + configuration + credential material, shipped as a plugin.

A Provider may implement multiple Facilities if it's natural to do so under a single credential/client (Google's one OAuth consent gives access to both Tasks and Calendar).

### Operation

A single callable method on a Facility. `TodoListManagement::create_task`, `CalendarScheduling::free_busy`. Each Operation has a typed signature, a typed success output, and a declared set of error conditions.

### Facility Definition

The formal specification of one Facility. Lives in-tree as:
1. A Rust trait annotated with `#[fidius::plugin_interface]` — the ABI contract and (via fidius) the schema.
2. A `SPEC.md` covering purpose, scope boundaries, lifecycle, error-code mapping, and concurrency.
3. A static `OPERATIONS` const declaring per-Operation Effect and Idempotency — a bridge until fidius's `#[method_meta]` feature lands, at which point the const is replaced with macro attributes and code-generated.
4. A conformance test kit exported from the Facility module and consumed by every Provider's CI.

A Provider claims a Facility by implementing the trait, being compiled against the matching interface version, and passing the conformance kit green.

### Binding

The runtime association of a Facility slot to a Provider. Declared by the user in `arawn.toml` under `[facilities]`:

```toml
[facilities]
todo_list       = "google-workspace"
calendar        = "google-workspace"
messaging       = "slack"
push            = "slack"
```

At most one Provider is bound per Facility slot in a given arawn instance.

### Facility Slot

The registry's type-level placeholder for one Facility. Slots are exhaustive per Facility type: there is exactly one `TodoListManagement` slot, exactly one `CalendarScheduling` slot, etc. Either a Provider is bound to a slot, or the slot is empty.

When a tool or workflow requests an unbound Facility, the host returns `MissingFacility`. The tool is expected to surface this to the agent as an actionable error ("no task list is configured; run `arawn setup <provider>`").

### Facility Registry

The runtime store of all active Bindings. Constructed once at startup from arawn.toml + the discovered Provider plugins. Tools and workflows consult the Registry to obtain a trait object for a needed Facility.

### Credential

The authentication material a Provider needs to function (OAuth token, API key, bot token, …). Credentials are **owned by the host**, not by Providers. The host's encrypted TokenStore persists them; at runtime the host hands the decrypted material to the Provider via a narrow, Provider-scoped interface. A Provider never reads or writes the TokenStore directly.

---

### Term mapping (for readers of earlier drafts)

| Earlier term | Now |
|---|---|
| Capability | Facility |
| Capability Definition | Facility Definition |
| Capability Slot | Facility Slot |
| Capability Registry / IntegrationRegistry | Facility Registry |
| `TaskListProvider` (the trait) | `TodoListManagement` (the trait) |
| `IntegrationError::MissingCapability` | `FacilityError::MissingFacility` |
| `arawn-integration` crate | `arawn-facility` crate (rename) |

## Mechanism — Fidius

Arawn uses [fidius](https://crates.io/crates/fidius) as the trait-to-dylib contract and evolution system for Facilities. Fidius provides a C-ABI-stable plugin framework built around `#[repr(C)]` vtables, magic bytes, and FNV-1a interface hashes. Every Facility trait carries `#[fidius::plugin_interface(version = N, buffer = PluginAllocated)]`; every Provider implementation carries `#[fidius::plugin_impl(FacilityTraitName)]` and emits `fidius::fidius_plugin_registry!()`. Plugins are `.arawn_provider` archives (cargo-built native dylibs) loaded via `fidius_host::PluginHost`.

### What fidius handles

| Concern | Fidius mechanism |
|---|---|
| ABI-stable trait → dylib | `#[repr(C)]` vtable, magic bytes, ABI versioning in descriptor |
| Interface versioning | `version = N` on `#[plugin_interface]`; carried in descriptor |
| Drift detection | FNV-1a `interface_hash` over method signatures; mismatched plugins fail to load with `LoadError::InterfaceHashMismatch` |
| Optional-method evolution | `capabilities: u64` bitfield — additive minor changes don't break older Providers |
| Serialization | `Serialize + Deserialize` enforced by macro; wire format auto-selected |
| Concurrency | `Send + Sync` required on trait; vtable entries are `&self` |
| Plugin packaging & loading | `.arawn_provider` archives, cargo-built dylibs, `PluginHost::discover/load` |
| Error transport | Generic `fidius::PluginError { code, message, details }` wire-type |

### What this specification defines on top of fidius

- **Facility ontology** — names, config keys, slot discipline (R1, Registry semantics).
- **Operation semantics metadata** — Effect, Idempotency (R4).
- **`FacilityError` taxonomy and `PluginError.code` mapping** (R6).
- **Credential delivery** — the mandatory `initialize` method (R7).
- **Conformance test kit** — property and golden tests every Provider runs in CI (R10).
- **Binding, Registry, `arawn setup` integration, Security Contract** — host-side, covered in later sections.

### Pending fidius upstream work

A feature request has been filed with fidius to add method-level metadata attributes (`#[fidius::method_meta(key, value)]`) surfaced via the plugin descriptor. When that lands, R4's Effect/Idempotency declarations migrate from a hand-written `OPERATIONS` const to macro attributes. Until then we use the const form, validated by conformance tests.

## Facility Definition Requirements

A Facility Definition is the **contract** between arawn core and the Providers that implement it. This section specifies what a Facility Definition must contain, who may author one, and how it evolves. See the Mechanism section above for how these requirements compose with fidius.

### What constitutes a Facility Definition

Every Facility Definition **MUST** include all of the following. A Definition that omits any of these is incomplete and **MUST NOT** be merged into `main`.

#### R1 — Identity

- **Stable name** in `UpperCamelCase` matching the Rust trait (`TodoListManagement`).
- **Stable config key** in `snake_case` matching the `[facilities]` binding slot (`todo_list`).
- **Interface version** declared exactly once via `#[fidius::plugin_interface(version = N, ...)]`. Fidius carries this in the plugin descriptor; the host uses it for compatibility checks. No parallel version constant exists — fidius is the single source of truth.
- **Interface hash** is computed automatically by fidius from the trait's method signatures; no author action required.

#### R2 — Purpose statement

One paragraph of prose, kept in the Facility's `SPEC.md`, answering:
- What class of activity does this Facility cover?
- Why is it a first-class Facility rather than an Operation under some other Facility?
- What is the boundary with adjacent Facilities?

Exhibit — good: *"TodoListManagement covers the agent's ability to record, recall, and mark-done items the user has asked to be tracked. It is distinct from `CalendarScheduling` (which deals with time-bound events with start and end instants) and from `Notes` (freeform content with no actionable status). A TodoListManagement item has a title, optional notes, optional due date, and a binary complete/incomplete state."*

#### R3 — Scope boundaries

Explicit lists in `SPEC.md`:
- **In scope:** Operations, concepts, and data the Facility covers.
- **Out of scope:** adjacent concerns a reader might reasonably expect to be in scope but aren't, each with a one-line reason and a pointer to where the concern is handled instead.

#### R4 — Operations

The complete, minimal set of methods on the trait. Each Operation **MUST** specify:
- **Name** (`snake_case`) on the trait method.
- **Typed input** — a struct or primitive. Never `serde_json::Value`.
- **Typed success output** — ditto.
- **Error type** — `Result<T, PluginError>` at the trait boundary (fidius's wire-type); mapped to `FacilityError` by the host adapter per R6.
- **Effect class** — `Read` (no side effects), `Write` (mutates external state), or `LongRunning` (may take > 5 s; host should treat as cancellable).
- **Idempotency contract** — `Idempotent` (retry-safe identically), `IdempotentWithKey` (retry-safe with a caller-supplied token), or `NonIdempotent` (retry may duplicate).

Until fidius's `#[method_meta]` feature ships, Effect and Idempotency are declared via a hand-written static const alongside the trait:

```rust
pub const OPERATIONS: &[OperationMeta] = &[
    OperationMeta {
        method: "create_task",
        effect: Effect::Write,
        idempotency: Idempotency::NonIdempotent,
    },
    OperationMeta {
        method: "list_tasks",
        effect: Effect::Read,
        idempotency: Idempotency::Idempotent,
    },
    // one entry per trait method
];
```

The conformance kit (R10) asserts the method-name set in `OPERATIONS` exactly equals the trait's method set (including `initialize` from R7). When fidius ships `#[method_meta]`, this const becomes a generated artifact derived from per-method attributes and the hand-written form is removed.

#### R5 — Domain types

All types appearing in Operation signatures **MUST**:
- Live in the Facility's module (not imported from a Provider crate).
- Implement `Serialize + Deserialize` (required by fidius at the wire boundary).
- Use `chrono::DateTime<Utc>` for all timestamps. No provider-native formats (Google RFC3339 quirks, Slack `ts` strings, etc.) at the domain boundary — Providers translate.
- Avoid Provider-specific shapes. Keep the domain type semantic, not wire-shaped.

Method receivers are `&self` and the trait is `Send + Sync` — both enforced by fidius.

#### R6 — Error taxonomy and code mapping

Every Facility Definition uses the shared `FacilityError` enum, at minimum:

- `AuthExpired` — credential no longer valid.
- `RateLimited { retry_after: Duration }`.
- `NotFound` — referenced external entity doesn't exist.
- `PreconditionFailed { reason: String }` — operation can't be satisfied in the current external state.
- `Network` — transient network failure.
- `ProviderInternal { message: String }` — a bug in the Provider or its upstream.
- `NotImplemented` — the Provider didn't set the fidius capability bit for this optional Operation.
- `MissingFacility` — no Provider is bound to this Facility slot.

Facility-specific errors may extend the enum but **MUST NOT** silently remap it. Every Operation documents in `SPEC.md` which subset it may raise.

**`PluginError.code → FacilityError` mapping.** Fidius transports a `PluginError { code, message, details }` across the FFI. Every Facility Definition publishes a mapping table in `SPEC.md` specifying the canonical code strings Providers return and the `FacilityError` variant each maps to. The baseline table, shared by all Facilities:

| Provider-returned `PluginError.code` | Maps to `FacilityError` |
|---|---|
| `AUTH_EXPIRED` | `AuthExpired` |
| `RATE_LIMITED` | `RateLimited { retry_after: Duration::from_secs(details.retry_after_secs) }` |
| `NOT_FOUND` | `NotFound` |
| `PRECONDITION_FAILED` | `PreconditionFailed { reason: message }` |
| `NETWORK` | `Network` |
| *anything else* | `ProviderInternal { message }` |

The host-side adapter performs this mapping when returning from an Operation. Providers are expected to use the canonical codes; unknown codes degrade to `ProviderInternal` without losing their message. Facility-specific codes (e.g., `CONFLICT_VERSION`) extend this table.

#### R7 — Lifecycle and the `initialize` Operation

Every Facility trait **MUST** declare `initialize` as its first Operation:

```rust
fn initialize(&self, credential: Credential) -> Result<(), PluginError>;
```

`initialize` is called exactly once by the host after the plugin is loaded and before any other Operation on the same handle. The Provider **MUST**:
1. Validate the Credential covers the required scopes.
2. Establish any HTTP client, rate limiter, or cached state the Provider needs.
3. Return `AUTH_EXPIRED` if the Credential is stale; the host surfaces `FacilityError::AuthExpired` and refuses to bind.

`SPEC.md` **MUST** list:
- Which `Credential` kinds are acceptable and with what scopes.
- Whether the Provider needs network access at `initialize` time.
- Any other prerequisites (e.g., "the Google Provider additionally reads a `region` env var").

If any precondition fails during `initialize`, the Registry does not bind the Provider; subsequent lookups return `MissingFacility` with the reason surfaced.

#### R8 — Concurrency contract

Fidius requires `Send + Sync` on the trait and generates `&self` vtable entries. Therefore **by construction**:
- Concurrent Operations on a single Provider handle are safe.
- The host may hold multiple in-flight calls through one handle.

`SPEC.md` may add constraints on top of this — e.g., "no ordering guaranteed between `complete_task` and a subsequent `list_tasks`; read-after-write consistency is best-effort per the upstream API" — but cannot weaken the safety guarantees fidius enforces.

#### R9 — Drift detection

Handled by fidius automatically. A Provider compiled against trait version `N` with method signatures `S` carries an FNV-1a hash of `S` in its descriptor. At load time the host computes the same hash from its copy of the trait and refuses the plugin on mismatch (`LoadError::InterfaceHashMismatch`).

**Implication:** changes to required-method signatures are automatically breaking. Additive minor changes are only possible via fidius's optional-method `capabilities` bitfield — see the versioning section below.

No hand-written `schema.json` exists. No manual schema-vs-trait drift test is needed. Fidius owns this entirely.

#### R10 — Conformance test kit

A set of property/golden tests any compliant Provider **MUST** pass, exported from the Facility's module. Covers:
- Success path for every Operation (including `initialize`).
- Each baseline error path (`AUTH_EXPIRED`, `RATE_LIMITED`, `NOT_FOUND`, `PRECONDITION_FAILED`, `NETWORK`).
- Round-trip serialization of every domain type.
- `OPERATIONS` const method-name set matches the trait's method set exactly.
- `PluginError.code → FacilityError` mapping for every code listed in the Facility's mapping table.

Providers invoke the conformance kit in their own CI against a mocked or fixture-driven Provider instance. A Provider that hasn't run the conformance tests green for a Facility **MUST NOT** claim that Facility in its manifest.

#### R11 — Reference Provider

Before a Facility graduates from `draft` to `stable`, at least one complete Provider implementation must exist in-tree (typically in a `crates/arawn-provider-<name>/` crate). The reference Provider:
- Proves the Definition can be implemented ("if nothing can, the Definition is broken").
- Acts as a worked example for third-party Provider authors.
- Targets the Facility's integration tests.

### Who may author a Facility Definition

Facilities are **in-tree only**. Plugins cannot introduce new Facility types — only new Providers for existing Facilities.

Rationale: Facilities are the stable surface the agent's reasoning and the user-facing tool catalog depend on. If each plugin could invent its own Facility name, an arawn user would have no predictable answer to "does arawn support messaging?" without enumerating plugin manifests. The Facility catalogue is deliberately small and governed.

A new Facility is introduced by:
1. Opening an ADR under `ARAWN-A-*` describing the class of activity, why the existing Facilities don't cover it, and the proposed Definition sketch.
2. On ADR acceptance, landing the full Definition (R1–R11) plus a reference Provider.
3. Publishing the Facility at fidius `version = 1` with the changelog entry.

### Facility versioning

Versions are integers (`1`, `2`, …) declared via `#[fidius::plugin_interface(version = N)]`.

- **Major bump (`N` → `N+1`)** — any change to a required-method signature, any change to a shared domain type's shape, any baseline error-taxonomy change. Fidius's `interface_hash` mismatch enforces this mechanically — prior plugins cannot load. Requires ADR.
- **Additive changes within a major** — new *optional* methods declared via fidius's capability bits. Existing plugins continue to load; newer plugins may advertise additional capabilities. These don't bump `version = N` but do require a changelog entry and conformance-kit update. The host calls `has_capability(bit)` before invoking an optional method; if unset, it returns `FacilityError::NotImplemented`.
- **R6 mapping-table extensions** — adding a new `PluginError.code` to the table doesn't bump the version; it's a pure conformance-test change.

Fidius carries the interface version in the plugin descriptor. The Registry refuses to bind Providers compiled against a different major than the host supports, with a clear error naming both versions.

### What a Facility Definition looks like on disk

```
crates/arawn-facility/
├── src/
│   ├── lib.rs                — re-exports, FacilityError, OperationMeta, Effect, Idempotency
│   ├── registry.rs           — FacilityRegistry, Binding, Slot
│   ├── credential.rs         — Credential handle type
│   └── facilities/
│       ├── todo_list_management/
│       │   ├── mod.rs        — `#[fidius::plugin_interface] trait TodoListManagement`
│       │   ├── types.rs      — domain types used by the trait
│       │   ├── operations.rs — static OPERATIONS const (bridge until fidius #[method_meta])
│       │   ├── SPEC.md       — R2/R3/R6-mapping-table/R7/R8 prose
│       │   └── conformance.rs — R10 test kit
│       ├── calendar_scheduling/
│       │   └── ...
│       ├── external_messaging/
│       │   └── ...
│       └── push_notification/
│           └── ...
```

Every Facility is a subdirectory with exactly this layout. `SPEC.md` is not optional; its absence fails a lint.

### Governance summary

| Change | Requires ADR | Requires conformance re-run | Requires major bump |
|---|---|---|---|
| New Facility | Yes | N/A | N/A (starts at version 1) |
| New **optional** method (new fidius capability bit) | No | Yes | No |
| New optional field on domain type | No | Yes | No |
| New error code in R6 mapping table | No | Yes | No |
| Rename / remove any method | Yes | Yes | Yes (interface_hash mismatch) |
| Change required-method signature | Yes | Yes | Yes (interface_hash mismatch) |
| Required field added to domain type | Yes | Yes | Yes |
| Remove `FacilityError` variant | Yes | Yes | Yes |
| Change Operation's Effect or Idempotency class | Yes | Yes | Yes (semantic break) |

## Provider Requirements

*To be drafted — see "Open Sections" below.*

## Initial Facility Catalogue

The Facility System ships with four Facilities defined in-tree. Each maps to one of the four traits landed by ARAWN-T-0179 (to be renamed).

| Facility | Short description | Reference Provider |
|---|---|---|
| `TodoListManagement` | Create/list/update/complete tasks on an external task list. | Google Tasks |
| `CalendarScheduling` | Read/write calendar events; compute free/busy. | Google Calendar |
| `ExternalMessaging` | Send and read messages on an external communication service. | Slack |
| `PushNotification` | Interrupt the user's attention with a short notification. | Slack (initially) |

These names are load-bearing in documentation, config, and code. Changes require an ADR.

## Open Sections (drafting order)

1. ~~Facility Definition Requirements~~ — drafted.
2. **Provider Requirements** — what a Provider must supply to claim a Facility: manifest fields, `initialize` implementation guidance, error-code conventions, conformance CI integration, packaging layout.
3. **Registry and Binding Semantics** — how Bindings are resolved at startup, plugin discovery, interface-version and hash-mismatch handling, hot-reload behaviour, what `arawn setup <provider>` writes.
4. **Security Contract** — exactly what a Provider can and cannot observe about the host (token scope, filesystem, other Providers). Interplay with the sandbox and sensitive-paths deny list.

*The "Wire Protocol" section from an earlier draft has been removed — fidius owns the wire layer (ABI, serialization, drift detection) and we do not define a parallel protocol.*

## Changelog

| Date | Change | Rationale |
|------|--------|-----------|
| 2026-04-17 | Initial draft — ontology locked. | First pass; ontology agreed via discussion. |
| 2026-04-17 | Fold fidius in as the mechanism; rewrite R1–R11; drop R9 wire-schema and Wire Protocol open section. | Fidius provides ABI-stable trait→dylib plumbing with automatic drift detection; hand-written schema document was redundant. Kept governance, ontology, and host-side responsibilities (R2/R3/R4-metadata/R6-mapping/R7-initialize/R10/R11). |