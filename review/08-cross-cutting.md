# Cross-Cutting Analysis

## Summary

Across all seven review lenses, three dominant architectural themes emerge:

1. **The engine crate is a gravity well.** `arawn-engine` (~21,000 lines) absorbs responsibilities that should be distributed, creating unnecessary coupling that manifests as legibility problems (LEG-003, LEG-004), evolvability friction (EVO-01, EVO-02, EVO-03), inflated compile times (EVO-03), and security surface confusion (the permission system, tool registry, and sandbox all live in one crate with different trust models).

2. **The permission/sandbox system has layered bypass paths.** Session grants override deny rules (COR-002/SEC-005), background shell commands skip the sandbox (SEC-004), grep/glob bypass path restrictions entirely (SEC-001), sandbox failure falls back to unsandboxed execution silently (SEC-006), and `set_permission_mode` can globally disable permissions with no authentication (SEC-007 + SEC-002). Individually each finding is a contained issue; together they form a pattern where every layer of the security model has a silent escape hatch.

3. **The service abstraction is half-built.** The `ArawnService` trait covers 7 of 16 RPC methods (API-002/EVO-05), the WebSocket protocol has no versioning or schema (API-004), error codes flatten structured information (API-003), and return types are untyped `serde_json::Value` for half the methods (API-006). The gap between the trait's promise ("Future: RemoteService") and the reality (9 methods bypass it entirely) means the service layer is neither a clean abstraction nor an honest direct implementation.

## Cross-Lens Findings

### XC-01: Tool Name Casing Mismatch Is Both a Legibility Bug and a Correctness Bug
**Lenses**: Legibility, Correctness
**Findings**: LEG-001, LEG-009, LEG-010, COR-007

The tool name casing inconsistency (LEG-001) is not merely a style issue. LEG-009 identifies that the `TASK_TOOLS` filter constant uses PascalCase names (`"TaskCreate"`, `"TaskUpdate"`, etc.) while the actual tools register with snake_case names (`"task_create"`, etc.). LEG-010 finds the same for `AGENT_TOOLS` (`"Agent"` vs `"agent"`). COR-007 notes that `filter_tools_for_context` may hide necessary tools from the LLM.

These are the same root problem: the naming inconsistency produces silent functional failures in the keyword-based tool filtering. After the first 2 messages in a session, the LLM will never be offered task or agent tools via keyword matching -- only through the "previously used" fallback or the first-turn "all tools" path. This degrades tool availability in longer sessions without any visible error.

**Severity adjustment**: LEG-009 and LEG-010 should be treated as **High** severity, not Major/Minor -- they are silent correctness bugs that degrade LLM capability in a way that is invisible to both the user and the developer.

---

### XC-02: JSONL Fragility Spans Correctness, Performance, Operability, and Evolvability
**Lenses**: Correctness, Performance, Operability, Evolvability
**Findings**: COR-008, PERF-03, OPS-06, EVO-07

The JSONL message format is a single design decision with four different symptoms:

- **COR-008**: No `fsync` after append means crash can produce partial lines
- **PERF-03**: Full file parse on load despite `load_compacted()` discarding everything before the last Summary -- pure waste I/O
- **OPS-06**: A partial line makes the entire session unloadable (no skip-bad-lines recovery)
- **EVO-07**: No version marker or migration path means any change to the `Message` enum breaks existing sessions

The root cause is that JSONL was chosen as a simple append-only format but never gained the reliability mechanisms that an append-only log needs: write barriers, corruption recovery, versioning, and efficient seeking. The format works fine on the happy path but has no resilience layer.

**Recommendation**: Address this holistically rather than piecemeal. A single effort should: (1) add skip-bad-lines recovery in `load()`, (2) record the last-Summary byte offset in SQLite for efficient seeking, and (3) add a version header line for future migration support. This solves COR-008, PERF-03, OPS-06, and EVO-07 in one pass.

---

### XC-03: Permission Bypass Chain
**Lenses**: Correctness, Security, Operability
**Findings**: COR-002, SEC-001, SEC-004, SEC-005, SEC-006, SEC-007, SEC-002

Multiple findings across correctness and security describe different ways the permission/sandbox system can be circumvented:

1. **SEC-001**: grep/glob have no path restrictions and are auto-allowed as read-only tools -- full-disk search
2. **SEC-004**: `run_in_background: true` skips the OS-level sandbox entirely
3. **SEC-006**: Sandbox failure silently falls back to unsandboxed execution
4. **COR-002/SEC-005**: Session grants override explicit deny rules
5. **SEC-007 + SEC-002**: Any local process can call `set_permission_mode(bypass)` with no authentication, globally disabling permissions

Each finding was rated Medium or High individually. In combination, they form a layered bypass chain where an attacker (prompt injection or malicious local process) has multiple independent paths to escape restrictions. The grep/glob path (SEC-001) is the most concerning because it requires zero special conditions -- it works in Default permission mode with no user interaction.

**Severity adjustment**: SEC-001 should be considered the highest priority security fix because it is exploitable in the default configuration with no user interaction, whereas the other bypass paths require either specific conditions (background mode, sandbox failure) or local process access.

---

### XC-04: Incomplete `ArawnService` Trait Is Both an API Design and Evolvability Problem
**Lenses**: API Design, Evolvability
**Findings**: API-002, EVO-05, API-003, API-006

The `ArawnService` trait problem appears in both the API and evolvability reviews but from different angles:

- **API-002**: The trait covers 7 of 16 RPC methods, making it a misleading abstraction
- **EVO-05**: A hypothetical `RemoteService` would miss half the functionality
- **API-003**: Error codes flatten `ServiceError` variants to a generic `"service_error"`, losing structured information -- this is made worse by the methods that bypass the trait and may not even return `ServiceError`
- **API-006**: Five methods return `serde_json::Value` instead of typed responses, and all five are among the methods that bypass the trait

The pattern is clear: methods added after the initial trait design bypassed it rather than extending it, accumulating technical debt. The `Value`-returning methods, the ad-hoc error codes, and the trait incompleteness are all symptoms of the same root cause -- the service layer was designed for an initial scope and never updated as functionality grew.

---

### XC-05: Monolithic Orchestration Functions Span Legibility, Evolvability, and Operability
**Lenses**: Legibility, Evolvability, Operability
**Findings**: LEG-003, LEG-004, EVO-09, OPS-02, OPS-03

The `main.rs` serve block (~350 lines of imperative setup) and `LocalService::send_message` (~220 lines) are flagged by legibility (hard to navigate), evolvability (high change cost for new subsystems), and operability (OPS-02: errors silently swallowed in the spawned task, OPS-03: no span context for request tracing).

The lack of intermediate abstractions makes it difficult to:
- Add span-based tracing (OPS-03) because there are no natural function boundaries to annotate with `#[instrument]`
- Handle errors correctly (OPS-02) because the error handling is inlined in a 100-line closure
- Add new subsystems (EVO-09) because insertion points are not obvious

Refactoring these two functions would simultaneously improve three review lenses.

---

### XC-06: Cancellation Gap Spans Correctness, Operability, and API Design
**Lenses**: Correctness, Operability, API Design
**Findings**: OPS-10, COR-001 (related)

The unimplemented `cancel()` (OPS-10) has implications beyond operability:
- The TUI sends Ctrl-C, the server responds with `{"status": "cancelled"}`, but nothing happens -- **false acknowledgment** is an API contract violation
- Without cancellation, there is no way to stop a misbehaving engine loop that is consuming LLM credits
- The absence of `CancellationToken` also means there is no mechanism for graceful shutdown (OPS-01) to stop in-flight requests

This is also related to COR-001 (no concurrent session guard) -- if cancellation worked, it could serve as an implicit guard against concurrent sends to the same session (cancel the old one before starting a new one).

## Root Causes

### RC-1: `arawn-engine` Accumulated Responsibilities Without Decomposition

**Symptoms**: EVO-01, EVO-02, EVO-03, EVO-06, LEG-005, LEG-006

The engine crate grew organically to house tools, permissions, hooks, plugins, skills, compaction, and the query loop. This single crate is the source of:
- The upward dependency from `arawn-mcp` (EVO-02) -- MCP needs the `Tool` trait which lives in the engine
- The TUI depending on engine internals (EVO-03) -- `ModalPrompt` and `PermissionMode` live in the engine
- Dual plugin system confusion (LEG-005) -- both systems live in the same crate with similar names
- Permission system using string-based categorization instead of the `Tool` trait (EVO-06) -- both live in the same crate but don't interact cleanly

**Resolution**: Extract an `arawn-tool` interface crate (`Tool`, `ToolOutput`, `ToolRegistry`, `ToolContext`) and move `ModalPrompt`/`PermissionMode` to `arawn-service`. This is the single highest-leverage structural change -- it would fix EVO-02, EVO-03, and create a clean boundary for the permission system.

### RC-2: No Data Durability Layer for JSONL

**Symptoms**: COR-008, PERF-03, OPS-06, EVO-07, COR-004

The JSONL format was chosen for simplicity (append-only, human-readable) but was never given the durability guarantees that persistent data needs. The promotion inconsistency (COR-004) is also a manifestation: SQLite and JSONL are updated non-atomically because they are separate systems with no shared transaction boundary.

### RC-3: Service Layer Did Not Evolve with Feature Growth

**Symptoms**: API-002, EVO-05, API-003, API-006, API-001, EVO-04

The `ArawnService` trait and WebSocket RPC layer were designed early for a smaller feature set. As memory, permissions, workflows, commands, and inventory were added, they were wired directly into `LocalService` and the WebSocket match block without updating the trait or establishing consistent patterns. This produced: an incomplete trait, hand-rolled dispatch boilerplate, inconsistent naming, untyped return values, and flattened error codes.

### RC-4: Security Model Designed for Happy Path

**Symptoms**: SEC-001, SEC-004, SEC-005, SEC-006, SEC-007, SEC-002

The sandbox, permissions, and path restrictions were designed for the common case (LLM uses tools as intended) but each has an escape path for edge cases (background mode, sandbox failure, read-only tools, session grants). The missing authentication (SEC-002) was reasonable for "localhost only" but the `set_permission_mode` RPC (SEC-007) makes it a force multiplier for any bypass.

## Tensions

### T-1: Security vs. Usability in Read-Only Tools (SEC-001 vs. Tool Design)

Grep and glob are read-only and auto-allowed for good reason -- asking permission for every grep would make the tool unusable. But the absence of path restrictions means they can read anything on disk. The tension is real: path-restricting read-only tools would add friction (permission prompts or errors when the LLM searches outside the workstream), but the current state allows full-disk search via prompt injection.

**Assessment**: The tradeoff was likely not conscious. A middle ground exists: restrict to the workstream root by default, allow `ctx.allowed_paths`, and auto-expand to the user's home directory only with explicit configuration. This preserves usability for normal use while limiting prompt injection damage.

### T-2: Evolvability vs. Current Simplicity in the Engine Crate (EVO-01)

Splitting `arawn-engine` into multiple crates would improve compile times, dependency management, and clean abstraction boundaries. But it would also add crate management overhead (more `Cargo.toml` files, more re-exports, more version coordination) for a project with a small team. The engine's current structure works -- the code is internally modular with clear module boundaries.

**Assessment**: The tension is resolved by extracting only what is needed: an `arawn-tool` interface crate. This is the minimum extraction that fixes the concrete dependency problems (EVO-02, EVO-03) without over-decomposing. The hooks, permissions, and skills modules can stay in the engine until they need to be independently consumed.

### T-3: Performance vs. Durability in JSONL (PERF-06 vs. COR-008)

Adding `fsync` after each JSONL append (fixing COR-008) would add measurable latency per tool call (10-20ms on SSD, much more on spinning disk). The current no-sync approach is fast but risks data loss.

**Assessment**: The better solution avoids this tension entirely: add corruption recovery in `load()` (skip bad lines) rather than preventing corruption via `fsync`. This preserves write performance while making the system resilient to the failure mode. Batch multiple messages into a single write-then-sync at turn boundaries for a middle ground.

### T-4: Operability vs. Complexity in Graceful Shutdown (OPS-01)

Implementing full graceful shutdown (signal handling, cancellation tokens, task draining, workflow shutdown) is a significant engineering effort for a single-user local tool that is typically stopped with Ctrl-C.

**Assessment**: The tension is real but the partial-JSONL-write failure mode (OPS-06) makes it worth addressing at a basic level. A minimal approach: handle SIGINT/SIGTERM, cancel in-flight engine tasks (which also fixes OPS-10), and let background tasks complete with a timeout. Full workflow draining can come later.

## Systemic Patterns

### SP-1: Silent Degradation as a Design Pattern

Multiple subsystems silently degrade rather than surfacing failures:
- `parse_arguments()` silently returns `{}` on malformed JSON (COR-003)
- JSONL persistence errors are logged but not surfaced to the user (OPS-02)
- Session stats update errors are completely silenced with `let _ =` (OPS-02)
- Sandbox failure falls back to unsandboxed execution with only a log warning (SEC-006)
- Config hot-reload logs new values but doesn't actually apply LLM/engine changes (OPS-07)
- `cancel()` returns success but does nothing (OPS-10)
- `TASK_TOOLS`/`AGENT_TOOLS` filter silently fails to match (LEG-009, LEG-010)

This is a consistent pattern: when something fails or is incomplete, the system continues as if nothing happened. For a user-facing tool, silent degradation is worse than an error -- the user builds trust based on apparent success, then discovers data loss or unexpected behavior later. The JSONL persistence case (OPS-02) is the clearest example: the user sees a successful response but their conversation was not saved.

**Recommendation**: Establish a project convention: failures that affect data integrity or security must be surfaced to the user, even if the system can continue. Use `EngineEvent::Error` or a new `EngineEvent::Warning` variant for non-fatal but user-visible problems.

### SP-2: String-Typed Dispatch Throughout

Multiple subsystems use string matching for dispatch or categorization:
- Tool names are plain strings matched in `CORE_TOOLS`, `TASK_TOOLS`, etc. (LEG-009)
- Permission `tool_category()` maps tool name strings to categories (EVO-06)
- RPC method dispatch is a string match block (EVO-04, LEG-012)
- Hook event matching uses string event names
- Skill invocation uses string skill names

This pattern means: no compile-time verification of name consistency, no IDE-assisted navigation, and high risk of silent mismatches (as demonstrated by LEG-009/LEG-010). In Rust, enum-based dispatch or trait-based registration would catch these errors at compile time.

### SP-3: Dual Systems Without Migration Paths

The codebase has accumulated parallel implementations without deprecation or migration:
- Legacy WASM plugins and new-style directory plugins (LEG-005, EVO-08)
- `ToolResult` (private) and `ToolOutput` (public) -- same struct, different names (LEG-002)
- `ArawnService` trait methods and direct `LocalService` methods (API-002)
- SQLite migrations (Refinery) for `arawn-storage` and programmatic table creation for `arawn-memory`

Each pair represents a system that evolved without removing or bridging the old version. This creates maintenance cost and contributor confusion.

### SP-4: The "God Context" Pattern

Several key types accumulate responsibilities by being the single bag of state passed through a subsystem:
- `ToolContext` carries 10 fields, most tools use 1-3 (API-007)
- `LocalService` is an 856-line struct bridging 10+ subsystems (EVO, LEG-004)
- `QueryEngine` is constructed with ~10 `with_*` calls for optional components

This is a natural consequence of Rust's ownership model (you need to pass everything the function might need), but it signals that the interfaces could be narrower. The risk is that adding a new field to `ToolContext` or `LocalService` is easy, while splitting them requires significant refactoring -- so they grow monotonically.

## Severity Adjustments

| Finding | Original Severity | Adjusted Severity | Rationale |
|---------|-------------------|-------------------|-----------|
| LEG-009 | Major | **High** | Not a naming style issue -- it is a silent correctness bug that prevents keyword-based tool filtering from ever matching task tools. Combined with COR-007, this degrades LLM capability in longer sessions. |
| LEG-010 | Minor | **Medium** | Same class of bug as LEG-009 for agent tools. The agent tool keyword filter silently never matches. |
| SEC-001 | High | **High (confirmed)** | Cross-referenced with the permission bypass chain (XC-03), this is the easiest-to-exploit path: requires no special conditions, works in default config, and enables full-disk information disclosure via prompt injection. |
| COR-002/SEC-005 | Medium | **High** | In isolation, session grants bypassing deny is a design quirk. Combined with SEC-002 (no auth) and SEC-007 (remote permission mode change), it means deny rules are not a reliable security boundary under any conditions. |
| OPS-02 | Medium | **High** | Silent data loss in a personal assistant's conversation history is a trust-breaking failure. Combined with OPS-06 (partial JSONL = unrecoverable session) and OPS-01 (no graceful shutdown), the risk of losing conversation data is higher than each finding suggests alone. |
| OPS-10 | Medium | **High** | False acknowledgment of cancellation is an API contract violation that wastes LLM credits and erodes user trust. Combined with OPS-01 (no graceful shutdown) and COR-001 (no concurrent session guard), the lack of cancellation is a blocker for multiple operational improvements. |
| API-007 | Low | **Low (confirmed)** | The "God context" pattern is real but acceptable at current scale. It becomes a problem only if the tool API is opened to third-party plugin authors. |
| EVO-07 | Medium | **Medium (confirmed)** | The JSONL migration risk is real but mitigated by the fact that the `Message` enum has been stable. Worth addressing proactively before it becomes an emergency. |
