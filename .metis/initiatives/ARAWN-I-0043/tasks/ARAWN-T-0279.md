---
---
id: arawn-ceremonies-crate-scaffold
level: task
title: "arawn-ceremonies crate scaffold + Ceremony plugin trait + plugin registry"
short_code: "ARAWN-T-0279"
created_at: 2026-05-15T23:44:44.415010+00:00
updated_at: 2026-05-15T23:44:44.415010+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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
