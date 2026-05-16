---
id: arawn-ceremonies-crate-scaffold
level: task
title: "arawn-ceremonies crate scaffold + Ceremony plugin trait + plugin registry"
short_code: "ARAWN-T-0279"
created_at: 2026-05-15T23:44:44.415010+00:00
updated_at: 2026-05-16T00:06:40.793915+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# arawn-ceremonies crate scaffold + plugin trait

## Goal
New `crates/arawn-ceremonies` crate. Defines the `Ceremony` plugin trait and a `PluginRegistry` that the cloacina workflow runner (T-0281) dispatches into. Nothing ceremony-specific — pure infrastructure.

## Reference
I-0043 Plugin Contract section. Trait shape:

```rust
trait Ceremony: Send + Sync {
    fn kind(&self) -> &'static str;
    fn period_key(&self, now: DateTime<Utc>) -> String;
    fn default_schedule(&self) -> CronSchedule;
    async fn gather(&self, ctx: &CeremonyCtx) -> Result<GatheredFacts, CeremonyError>;
    async fn compose(&self, ctx: &CeremonyCtx, facts: GatheredFacts) -> Result<Vec<NewItem>, CeremonyError>;
    fn interactive_actions(&self) -> &[InteractiveAction] { &[] }
    fn patterns(&self) -> Option<&dyn PatternDetector> { None }
}
```

## Acceptance
- New crate at `crates/arawn-ceremonies` registered in the workspace.
- `Ceremony` trait + `CeremonyError` + `NewItem` + `GatheredFacts` + `CeremonyCtx` + `InteractiveAction` placeholders.
- `PluginRegistry { register, get_by_kind, all }` with `Arc<dyn Ceremony>` storage.
- 4–6 unit tests: register/get/iterate + duplicate-kind rejection.
- Empty `lib.rs` re-exports keep the public surface clean.

## Out of scope
Cron loop (T-0281), schema (T-0280), the gather/compose pipeline (T-0282), RPC (T-0283), any concrete plugin.

## Notes
The trait is `async` on `gather` + `compose` because both need DB + LLM. `interactive_actions` is sync because it returns static metadata.
## Status Updates

**2026-05-16 — implementation landed.**

- New `crates/arawn-ceremonies` registered in the workspace.
- Module layout:
  - `error.rs` — `CeremonyError` with `MissingCitation`, `DuplicateKind`, `InvalidTabletState`, `InsufficientHistory`, `Storage`, `Llm`, `Other` variants. Constructor helpers for the load-bearing ones.
  - `types.rs` — `TabletStatus { Open|Reviewed|Unreviewed|Archived }`, `ItemKind` enum mirroring the schema, `GatheredFacts { payload, gathered_at }`, `DetectedPattern { iso_week, pattern_key, magnitude, payload }`.
  - `plugin.rs` — the `Ceremony` trait + supporting types: `CronSchedule { expression, timezone }`, `InteractiveAction`, `ComposedItem` (citation REQUIRED at the type level via field), `UserItem` (no citation field), `NewItem` enum routing between the two write paths, `CeremonyCtx` trait (write_pattern_row exposed; T-0282 wires the rest), `PatternDetector` trait.
  - `registry.rs` — `PluginRegistry` with register/get/all/len/is_empty. Backed by `Arc<RwLock<HashMap<String, Arc<dyn Ceremony>>>>` so cloning the registry shares the same storage. Duplicate-kind registration returns `Err`. 6 unit tests.
- The two-write-path contract is enforced at the **type level**: `NewItem::Composed(ComposedItem)` has a non-optional `citation_id` field; `NewItem::User(UserItem)` has no such field. T-0282 implements the matching dispatch but the contract can't be violated by a plugin even before T-0282 lands.
- Workspace check clean; 6 tests pass.

Next: T-0280 (schema migration).