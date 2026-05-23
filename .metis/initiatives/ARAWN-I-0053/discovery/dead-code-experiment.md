# Dead-code validation experiment — results

*Run 2026-05-21 during ARAWN-I-0053 discovery phase. All 15 `#[allow(dead_code)]` / `#[allow(unused_*)]` / `#[allow(deprecated)]` sites were stripped at once; the workspace was then rebuilt under three profiles and warnings captured. Tree was reverted with `git checkout -- crates/` after.*

## Procedure

1. Confirmed clean baseline: `cargo build --workspace --release` ✅, `cargo check --workspace` ✅.
2. Used a small Python script to comment out all 15 allow attributes with `// EXPERIMENT-STRIPPED: was #[allow(...)]` markers across 12 files.
3. Ran three profiles, captured `warning: ... never (used|read|constructed)` lines:
   - `cargo check --workspace`
   - `cargo build --workspace --release`
   - `cargo test --workspace --no-run`
4. Reverted via `git checkout -- crates/`. Verified zero `EXPERIMENT-STRIPPED` markers remained.

## Important caveat: broken integration test contaminated test-profile output

`crates/arawn-ceremonies/tests/brief_pipeline.rs:59` fails to compile (missing `period_window` trait method on `ScriptedPlugin`). When this happens, cargo aborts lint analysis for the rest of `arawn-ceremonies`'s test targets, producing false-positive "never used" warnings on items that ARE used by inline tests within `engine.rs`. Specifically:

- `begin` (engine.rs:505) — IS used at engine.rs:830 (inline test), but appeared as "never used" in the test build.
- `commit` (engine.rs:516) — IS used at engine.rs:840 (inline test), but appeared as "never used" in the test build.

`rollback` (engine.rs:527) was independently verified as having zero callers anywhere in the workspace via grep — it is truly dead.

## Results — Symbol-by-symbol

| Site | Symbol | Debug check | Release build | Test --no-run | Final verdict |
|---|---|---|---|---|---|
| `ceremonies/engine.rs:504` | `fn begin` | "never used" | "never used" | "never used" (false positive — see caveat) | **Keep but convert to `#[cfg(test)]`** — really used by inline tests. |
| `ceremonies/engine.rs:515` | `fn commit` | "never used" | "never used" | "never used" (false positive — see caveat) | **Keep but convert to `#[cfg(test)]`** — really used by inline tests. |
| `ceremonies/engine.rs:526` | `fn rollback` | "never used" | "never used" | "never used" | **Truly dead — delete.** |
| `ceremonies/service.rs:910` | `fn status_str` | "never used" | "never used" | "never used" | **Truly dead — delete.** |
| `engine/background.rs:122` | `handle` field | "never read" | "never read" | "never read" | **Keep + improve comment.** Field is held for Drop semantics; Rust's lint can't see Drop usage as a read. The `#[allow(dead_code)]` is legitimately necessary. |
| `engine/plan.rs:32` | `stripped_rules` field | "never read" | "never read" | "never read" | **Truly dead — delete.** Speculative future-proofing that was never written to. |
| `engine/system_prompt.rs:206` | `PromptSection` (annotation on the struct) | the *field* `name` is "never read" | same | same | **Spurious — the struct is alive, only the `name` field is dead.** Drop the field rather than the allow. |
| `engine/tools/ceremony.rs:366` | `fn _add_item_unused` | (no warning — `_` prefix exempted) | (no warning) | (no warning) | **Allow is redundant — `_` prefix already exempts it.** Drop the allow; the function itself can stay as a forward-reservation if `AddItemRequest` import is needed. |
| `engine/tools/steward.rs:380` | `fn _unused` | (no warning) | (no warning) | (no warning) | **Allow is redundant.** Function should also be deleted because its only purpose is to fake-use `resolve_workstream`, which is itself dead (see below). |
| `engine/tools/steward.rs:84` | `fn resolve_workstream` | "never used" | "never used" | "never used" | **Truly dead — delete** (alongside `_unused`). |
| `extractor/cot.rs:629` | `fn push_classify` | (no warning; in `#[cfg(test)]`) | (no warning) | "never used" | **Test-only dead — delete.** A test-internal builder method nobody calls. |
| `feeds/clients/atlassian.rs:525` | `#[allow(deprecated)]` on call to `get_all_projects` | (no deprecation warning) | (no deprecation warning) | (no deprecation warning) | **Annotation likely redundant.** Either the upstream's `#[deprecated]` doesn't reach this call site, or the warning system isn't running deprecation lints in this build mode. Operator can drop the annotation and re-verify. |
| `feeds/store.rs:176` | `pub use serde_json::Value as JsonValue` | (no warning — `pub` re-exports aren't dead-code) | (no warning) | (no warning) | **Allow is redundant.** No `arawn_feeds::JsonValue` callers anywhere. The whole alias should be deleted as cruft. |
| `feeds/store.rs:178` | `fn _value_marker` | (no warning — `_` prefix exempted) | (no warning) | (no warning) | **Allow is redundant.** And the function itself is pointless once `JsonValue` is removed. |
| `feeds/templates/github/repo_mirror.rs:374` | `fn _force_use_traits` | (no warning — `_` prefix exempted) | (no warning) | (no warning) | **Allow is redundant.** Verify whether the trait imports it "forces" are needed by real code in this file; if so, the function itself can go. |
| `steward/dust.rs:309` | `fn _ts` | (no warning — `_` prefix exempted) | (no warning) | (no warning) | **Allow is redundant.** Confirm `chrono::Utc::now()` is used by real code in this file; if so, `_ts` can be deleted with no other changes. |

## Cross-cutting observations from the experiment

1. **6 of 15 allows decorate `_`-prefixed symbols where Rust's lint already exempts the name.** These allows were never load-bearing.
2. **2 of 15 are on `pub use` re-exports** (`JsonValue`, the engine `as ToolContext` alias). These don't fire dead-code warnings — the allows were preemptive but unnecessary.
3. **1 of 15 (`background.rs:122`) is genuinely needed.** The "field held for Drop semantics" pattern is a Rust limitation.
4. **2 of 15 (`begin`, `commit` in ceremonies/engine.rs) were red herrings caused by a broken test.** They are real uses in tests; the lint just couldn't see them due to compilation abort.
5. **1 of 15 (`#[allow(deprecated)]` on atlassian) didn't trigger any deprecation warning** when stripped, suggesting the upstream crate's `#[deprecated]` may not propagate at that call site under the current rustc / lint configuration. Worth a focused follow-up before assuming the annotation is needed.
6. **3 of 15 are truly dead symbols** (`status_str`, `rollback`, `resolve_workstream`, plus the speculative `stripped_rules` field).

## Reproducibility

The strip script (Python 3) is preserved at the foot of this file. Run from the repo root with:

```bash
/usr/bin/python3 -c "$(cat <<'EOF'
import re, pathlib
pattern = re.compile(r'^(\s*)#\[allow\((dead_code|unused_imports|unused_variables|unused_must_use|unused|deprecated)\)\]\s*$', re.M)
count = 0
for path in pathlib.Path('crates').rglob('*.rs'):
    content = path.read_text()
    if not pattern.search(content): continue
    new = pattern.sub(r'\1// EXPERIMENT-STRIPPED: was #[allow(\2)]', content)
    if new != content:
        path.write_text(new)
        count += len(pattern.findall(content))
print(f'Stripped {count} allow(...) attributes.')
EOF
)"
```

Then build three profiles and capture warnings. Revert with `git checkout -- crates/`.
