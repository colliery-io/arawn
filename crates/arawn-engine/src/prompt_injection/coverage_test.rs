//! Static-coverage test: every inbound-text boundary in the engine
//! must funnel through the prompt-injection guard.
//!
//! This is a brittle-by-design test: if a new inbound boundary lands
//! without a `prompt_injection::enforce` call, CI breaks. Updating
//! it is the explicit step a contributor takes when they add a new
//! boundary.
//!
//! Update the `BOUNDARIES` list when you add a new inbound boundary.
//! Each entry is `(path relative to workspace root, expected needle)`.

#[cfg(test)]
mod static_coverage {
    use std::path::PathBuf;

    /// Paths the guard *must* appear in. If you add a new inbound
    /// boundary, add the file here and wire the call.
    const BOUNDARIES: &[&str] = &[
        "crates/arawn-engine/src/tools/web_fetch.rs",
        "crates/arawn-engine/src/tools/feed_search.rs",
    ];

    fn workspace_root() -> PathBuf {
        // CARGO_MANIFEST_DIR is set per crate; walk up two levels to
        // the workspace root (`crates/arawn-engine/` → workspace).
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        crate_dir.parent().unwrap().parent().unwrap().to_path_buf()
    }

    #[test]
    fn every_inbound_boundary_calls_the_guard() {
        let root = workspace_root();
        for path in BOUNDARIES {
            let full = root.join(path);
            let source = std::fs::read_to_string(&full)
                .unwrap_or_else(|e| panic!("could not read {}: {e}", full.display()));
            assert!(
                source.contains("prompt_injection::enforce"),
                "{path} is in the inbound-boundary list but does not call \
                 prompt_injection::enforce(). Either wire the guard or remove \
                 the file from BOUNDARIES with a justification."
            );
        }
    }
}
