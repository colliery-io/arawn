//! `filesystem/folder` — watch a local folder, emit a signal per
//! text-file change (created / modified / deleted).
//!
//! Unlike the provider-backed templates, this one needs no
//! [`FeedClients`](crate::clients::FeedClients) — it reads the local
//! filesystem directly. On each cadence fire it walks the configured
//! `root`, builds a `(mtime, size)` fingerprint per included file, and
//! diffs that map against the cursor it persisted last run to detect
//! what was created / modified / deleted since.
//!
//! Intended for raw text drops (meeting transcripts, notes), **not**
//! for code trees — hence the default excludes for `.git`, `target`,
//! `node_modules`, etc.
//!
//! ## Task split
//!
//! - **T-A (this file)** lands the param/cursor types, registration
//!   metadata, and synchronous `validate()`. [`FilesystemFeedTemplate::run`]
//!   is `unimplemented!()`.
//! - **T-B (ARAWN-T-0418)** fills `run()` with the scan-and-diff core.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

use async_trait::async_trait;
use chrono::Utc;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use walkdir::WalkDir;

use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

/// Default cadence for a filesystem feed.
///
/// The runtime enforces a 15-minute cadence floor ([`crate::cadence::MIN_CADENCE`])
/// and cron expressions are minute-granularity, so a sub-minute "watch"
/// latency is not representable on this surface. We default to the
/// floor — the fastest the runtime allows.
const DEFAULT_CADENCE: &str = "*/15 * * * *";

/// Parameters for the `filesystem/folder` template.
///
/// `root` is the absolute folder to watch. `recursive` controls whether
/// subdirectories are descended. `include` / `exclude` are glob patterns
/// (matched against the path relative to `root`); a file is emitted when
/// it matches at least one `include` and no `exclude`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemFeedParams {
    pub root: PathBuf,
    #[serde(default = "default_recursive")]
    pub recursive: bool,
    #[serde(default = "default_include")]
    pub include: Vec<String>,
    #[serde(default = "default_exclude")]
    pub exclude: Vec<String>,
}

fn default_recursive() -> bool {
    true
}

fn default_include() -> Vec<String> {
    vec!["**/*".to_string()]
}

fn default_exclude() -> Vec<String> {
    [
        ".git/**",
        "target/**",
        "node_modules/**",
        ".venv/**",
        "__pycache__/**",
        ".DS_Store",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

impl Default for FilesystemFeedParams {
    fn default() -> Self {
        Self {
            root: PathBuf::new(),
            recursive: default_recursive(),
            include: default_include(),
            exclude: default_exclude(),
        }
    }
}

/// Per-file fingerprint used to detect change without reading contents.
///
/// `(mtime, size)` is cheap to compute and good enough for transcript
/// drops — an editor save bumps mtime, a content change bumps size or
/// mtime. Pathological in-place edits that preserve both are not
/// detected, which is an accepted tradeoff for the polling model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFingerprint {
    /// Unix mtime in seconds.
    pub mtime: i64,
    /// File size in bytes.
    pub size: u64,
}

/// Cursor persisted between runs: the fingerprint map from the previous
/// scan. `BTreeMap` (not `HashMap`) so the serialized cursor is
/// deterministic and diff-friendly on disk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilesystemFeedCursor {
    #[serde(default)]
    pub files: BTreeMap<PathBuf, FileFingerprint>,
}

/// The `filesystem/folder` template.
pub struct FilesystemFeedTemplate;

#[async_trait]
impl FeedTemplate for FilesystemFeedTemplate {
    fn name(&self) -> &'static str {
        "filesystem/folder"
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        let parsed: FilesystemFeedParams = serde_json::from_value(params.as_value().clone())
            .map_err(|e| FeedError::InvalidParams(format!("filesystem/folder params: {e}")))?;
        validate_params(&parsed)
    }

    fn param_schema(&self) -> Vec<crate::param_schema::ParamSpec> {
        use crate::param_schema::{ParamKind, ParamSpec};
        vec![
            ParamSpec::required(
                "root",
                "Folder to watch",
                ParamKind::Path,
                "Absolute path to the folder to watch. Must be a real directory at least \
                 three path components deep.",
            ),
            ParamSpec::optional(
                "recursive",
                "Recursive",
                ParamKind::Bool,
                serde_json::json!(true),
                "Descend into subdirectories.",
            ),
            ParamSpec::optional(
                "include",
                "Include globs",
                ParamKind::List,
                serde_json::json!(["**/*"]),
                "Glob patterns a file must match (relative to root).",
            ),
            ParamSpec::optional(
                "exclude",
                "Exclude globs",
                ParamKind::List,
                serde_json::json!([
                    ".git/**",
                    "target/**",
                    "node_modules/**",
                    ".venv/**",
                    "__pycache__/**",
                    ".DS_Store"
                ]),
                "Glob patterns that exclude matches; exclude wins over include.",
            ),
        ]
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: DEFAULT_CADENCE.to_string(),
            initial_cursor: serde_json::json!({ "files": {} }),
        }
    }

    async fn run(
        &self,
        _ctx: &TemplateCtx,
        params: &TemplateParams,
        feed_dir: &Path,
        cursor: &Value,
    ) -> Result<RunOutcome, FeedError> {
        let started = Instant::now();

        let parsed: FilesystemFeedParams = serde_json::from_value(params.as_value().clone())
            .map_err(|e| FeedError::InvalidParams(format!("filesystem/folder params: {e}")))?;
        let prev = parse_cursor(cursor);

        let include = build_matcher(&parsed.include)?;
        let exclude = build_matcher(&parsed.exclude)?;
        let curr = scan(&parsed.root, parsed.recursive, &include, &exclude)?;

        let signals = diff(&parsed.root, &prev, &curr);

        // Append one JSONL line per change. Append-only so the signal
        // log is an immutable record of every event the feed observed.
        let mut bytes_written: u64 = 0;
        if !signals.is_empty() {
            let log_path = feed_dir.join("signals.jsonl");
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .map_err(|e| FeedError::Storage(format!("open {}: {e}", log_path.display())))?;
            for s in &signals {
                let line = serde_json::to_string(s)
                    .map_err(|e| FeedError::Storage(format!("serialize signal: {e}")))?;
                let body = format!("{line}\n");
                f.write_all(body.as_bytes()).map_err(|e| {
                    FeedError::Storage(format!("write {}: {e}", log_path.display()))
                })?;
                bytes_written += body.len() as u64;
            }
        }

        let new_cursor = serde_json::to_value(FilesystemFeedCursor { files: curr })
            .map_err(|e| FeedError::Storage(format!("serialize cursor: {e}")))?;
        let status = if signals.is_empty() {
            "no-changes".to_string()
        } else {
            "ok".to_string()
        };

        Ok(RunOutcome {
            cursor: new_cursor,
            summary: RunSummary {
                items_written: signals.len() as u64,
                bytes_written,
                duration: started.elapsed(),
            },
            status,
        })
    }
}

/// Parse a persisted cursor into its fingerprint map. A `null` or
/// malformed cursor (e.g. first run) degrades to an empty map, which
/// makes every matching file look newly `created` — the documented
/// "auto-index on first registration" behavior.
fn parse_cursor(cursor: &Value) -> BTreeMap<PathBuf, FileFingerprint> {
    serde_json::from_value::<FilesystemFeedCursor>(cursor.clone())
        .map(|c| c.files)
        .unwrap_or_default()
}

/// Compile a list of glob patterns into a [`GlobSet`]. An empty pattern
/// list builds an empty set that matches nothing — for `include` that
/// means "emit nothing", for `exclude` it means "exclude nothing".
fn build_matcher(patterns: &[String]) -> Result<GlobSet, FeedError> {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        builder.add(
            Glob::new(p)
                .map_err(|e| FeedError::InvalidParams(format!("malformed glob '{p}': {e}")))?,
        );
    }
    builder
        .build()
        .map_err(|e| FeedError::InvalidParams(format!("glob set build failed: {e}")))
}

/// Walk `root` and build the current fingerprint map for every file
/// that matches `include` and not `exclude` (matched against the path
/// relative to `root`). `exclude` always wins. Keys are absolute paths.
fn scan(
    root: &Path,
    recursive: bool,
    include: &GlobSet,
    exclude: &GlobSet,
) -> Result<BTreeMap<PathBuf, FileFingerprint>, FeedError> {
    let mut curr = BTreeMap::new();

    let mut walker = WalkDir::new(root);
    if !recursive {
        // depth 0 is the root dir itself; depth 1 is its immediate
        // children. Capping at 1 keeps the walk to the top level.
        walker = walker.max_depth(1);
    }

    for entry in walker {
        let entry =
            entry.map_err(|e| FeedError::Storage(format!("walk {}: {e}", root.display())))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        if !include.is_match(rel) || exclude.is_match(rel) {
            continue;
        }
        let meta = entry
            .metadata()
            .map_err(|e| FeedError::Storage(format!("stat {}: {e}", entry.path().display())))?;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        curr.insert(
            entry.path().to_path_buf(),
            FileFingerprint {
                mtime,
                size: meta.len(),
            },
        );
    }

    Ok(curr)
}

/// Diff the previous fingerprint map against the current one and emit
/// one signal per change: `created` for new paths, `modified` when the
/// fingerprint changed, `deleted` for paths gone from the current scan.
/// Unchanged paths emit nothing.
fn diff(
    root: &Path,
    prev: &BTreeMap<PathBuf, FileFingerprint>,
    curr: &BTreeMap<PathBuf, FileFingerprint>,
) -> Vec<Value> {
    let mut signals = Vec::new();

    for (path, fp) in curr {
        match prev.get(path) {
            None => signals.push(signal(root, path, "created", Some(fp))),
            Some(old) if old != fp => signals.push(signal(root, path, "modified", Some(fp))),
            _ => {} // unchanged
        }
    }
    for path in prev.keys() {
        if !curr.contains_key(path) {
            signals.push(signal(root, path, "deleted", None));
        }
    }

    signals
}

/// Build one signal record in the documented shape. `size_bytes` and
/// `mtime` are `null` for `deleted` events (no fingerprint to report).
fn signal(root: &Path, path: &Path, event: &str, fp: Option<&FileFingerprint>) -> Value {
    let rel_path = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    json!({
        "source": root.to_string_lossy(),
        "path": path.to_string_lossy(),
        "rel_path": rel_path,
        "event": event,
        "ts": Utc::now().to_rfc3339(),
        "size_bytes": fp.map(|f| f.size),
        "mtime": fp.map(|f| f.mtime),
    })
}

/// Synchronous, no-IO-beyond-stat validation of filesystem params.
///
/// Rejects, in order: a relative `root`, a `root` with fewer than three
/// path components (guards against watching `/`, `/Users`, etc.), a
/// non-existent or inaccessible `root`, a `root` that isn't a directory,
/// and any malformed glob in `include` / `exclude`.
fn validate_params(p: &FilesystemFeedParams) -> Result<(), FeedError> {
    if !p.root.is_absolute() {
        return Err(FeedError::InvalidParams(format!(
            "root must be an absolute path, got '{}'",
            p.root.display()
        )));
    }

    // Component count includes the root-dir component, so `/Users/foo`
    // is 3 and `/Users` is 2. Refusing < 3 keeps a feed from blanketing
    // a whole home or volume root.
    if p.root.components().count() < 3 {
        return Err(FeedError::InvalidParams(format!(
            "root '{}' is too broad; watch a specific folder at least two levels deep",
            p.root.display()
        )));
    }

    let meta = std::fs::metadata(&p.root).map_err(|e| {
        FeedError::InvalidParams(format!("root '{}' is not accessible: {e}", p.root.display()))
    })?;
    if !meta.is_dir() {
        return Err(FeedError::InvalidParams(format!(
            "root '{}' is not a directory",
            p.root.display()
        )));
    }

    for g in p.include.iter().chain(p.exclude.iter()) {
        globset::Glob::new(g)
            .map_err(|e| FeedError::InvalidParams(format!("malformed glob '{g}': {e}")))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params_for(root: PathBuf) -> FilesystemFeedParams {
        FilesystemFeedParams {
            root,
            ..Default::default()
        }
    }

    #[test]
    fn defaults_match_documented_shape() {
        let p = FilesystemFeedParams::default();
        assert!(p.recursive);
        assert_eq!(p.include, vec!["**/*".to_string()]);
        assert_eq!(p.exclude.len(), 6);
        assert!(p.exclude.contains(&".git/**".to_string()));
        assert!(p.exclude.contains(&".DS_Store".to_string()));
    }

    #[test]
    fn params_round_trip_through_serde() {
        let p = params_for(PathBuf::from("/Users/me/notes"));
        let v = serde_json::to_value(&p).unwrap();
        let back: FilesystemFeedParams = serde_json::from_value(v).unwrap();
        assert_eq!(back.root, p.root);
        assert_eq!(back.recursive, p.recursive);
        assert_eq!(back.include, p.include);
        assert_eq!(back.exclude, p.exclude);
    }

    #[test]
    fn params_apply_defaults_when_only_root_given() {
        let v = serde_json::json!({ "root": "/Users/me/notes" });
        let p: FilesystemFeedParams = serde_json::from_value(v).unwrap();
        assert!(p.recursive);
        assert_eq!(p.include, default_include());
        assert_eq!(p.exclude, default_exclude());
    }

    #[test]
    fn cursor_round_trips() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("a.txt"),
            FileFingerprint {
                mtime: 1234,
                size: 56,
            },
        );
        let cursor = FilesystemFeedCursor { files };
        let v = serde_json::to_value(&cursor).unwrap();
        let back: FilesystemFeedCursor = serde_json::from_value(v).unwrap();
        assert_eq!(back.files.len(), 1);
        assert_eq!(
            back.files.get(&PathBuf::from("a.txt")).unwrap().mtime,
            1234
        );
    }

    #[test]
    fn defaults_use_the_cadence_floor() {
        let t = FilesystemFeedTemplate;
        let d = t.defaults(&TemplateParams::default());
        crate::cadence::validate_cadence(&d.cadence).expect("default cadence must satisfy floor");
    }

    #[test]
    fn validate_accepts_a_real_deep_directory() {
        let dir = tempfile::tempdir().unwrap();
        // tempdir lives under the system temp root, which is comfortably
        // more than 3 components deep on macOS/Linux.
        let p = params_for(dir.path().to_path_buf());
        validate_params(&p).expect("a real deep dir should validate");
    }

    #[test]
    fn validate_rejects_relative_root() {
        let p = params_for(PathBuf::from("relative/notes"));
        assert!(matches!(
            validate_params(&p),
            Err(FeedError::InvalidParams(_))
        ));
    }

    #[test]
    fn validate_rejects_too_shallow_root() {
        // `/tmp` exists and is a dir but is only 2 components deep.
        let p = params_for(PathBuf::from("/tmp"));
        assert!(matches!(
            validate_params(&p),
            Err(FeedError::InvalidParams(_))
        ));
    }

    #[test]
    fn validate_rejects_nonexistent_root() {
        let p = params_for(PathBuf::from("/Users/nobody/does/not/exist/xyzzy"));
        assert!(matches!(
            validate_params(&p),
            Err(FeedError::InvalidParams(_))
        ));
    }

    #[test]
    fn validate_rejects_non_directory_root() {
        let f = tempfile::NamedTempFile::new().unwrap();
        let p = params_for(f.path().to_path_buf());
        assert!(matches!(
            validate_params(&p),
            Err(FeedError::InvalidParams(_))
        ));
    }

    #[test]
    fn validate_rejects_malformed_include_glob() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = params_for(dir.path().to_path_buf());
        p.include = vec!["[".to_string()];
        assert!(matches!(
            validate_params(&p),
            Err(FeedError::InvalidParams(_))
        ));
    }

    #[test]
    fn validate_rejects_malformed_exclude_glob() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = params_for(dir.path().to_path_buf());
        p.exclude = vec!["[".to_string()];
        assert!(matches!(
            validate_params(&p),
            Err(FeedError::InvalidParams(_))
        ));
    }

    // ─── T-B: scan-and-diff core ─────────────────────────────────────

    fn write_file(dir: &Path, rel: &str, contents: &str) {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, contents).unwrap();
    }

    fn matchers(include: &[&str], exclude: &[&str]) -> (GlobSet, GlobSet) {
        let inc = build_matcher(&include.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap();
        let exc = build_matcher(&exclude.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap();
        (inc, exc)
    }

    fn default_matchers() -> (GlobSet, GlobSet) {
        (
            build_matcher(&default_include()).unwrap(),
            build_matcher(&default_exclude()).unwrap(),
        )
    }

    #[test]
    fn empty_cursor_emits_created_for_each_match() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "a.txt", "a");
        write_file(dir.path(), "b.txt", "b");
        let (inc, exc) = default_matchers();
        let curr = scan(dir.path(), true, &inc, &exc).unwrap();
        let signals = diff(dir.path(), &BTreeMap::new(), &curr);
        assert_eq!(signals.len(), 2);
        assert!(signals.iter().all(|s| s["event"] == "created"));
    }

    #[test]
    fn unchanged_file_emits_nothing() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "a.txt", "a");
        let (inc, exc) = default_matchers();
        let curr = scan(dir.path(), true, &inc, &exc).unwrap();
        let signals = diff(dir.path(), &curr, &curr);
        assert!(signals.is_empty());
    }

    #[test]
    fn modified_file_emits_modified() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "a.txt", "a");
        let (inc, exc) = default_matchers();
        let first = scan(dir.path(), true, &inc, &exc).unwrap();
        // Grow the file so the (mtime, size) fingerprint differs even
        // within the same mtime-second.
        write_file(dir.path(), "a.txt", "aaaaaaaaaa");
        let second = scan(dir.path(), true, &inc, &exc).unwrap();
        let signals = diff(dir.path(), &first, &second);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0]["event"], "modified");
    }

    #[test]
    fn deleted_file_emits_deleted() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "a.txt", "a");
        let (inc, exc) = default_matchers();
        let first = scan(dir.path(), true, &inc, &exc).unwrap();
        std::fs::remove_file(dir.path().join("a.txt")).unwrap();
        let second = scan(dir.path(), true, &inc, &exc).unwrap();
        let signals = diff(dir.path(), &first, &second);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0]["event"], "deleted");
        assert!(signals[0]["size_bytes"].is_null());
        assert!(signals[0]["mtime"].is_null());
    }

    #[test]
    fn exclude_glob_skips_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "keep.txt", "k");
        write_file(dir.path(), "target/skip.txt", "s");
        let (inc, exc) = matchers(&["**/*"], &["target/**"]);
        let curr = scan(dir.path(), true, &inc, &exc).unwrap();
        assert_eq!(curr.len(), 1);
        assert!(curr.keys().next().unwrap().ends_with("keep.txt"));
    }

    #[test]
    fn recursive_false_ignores_subdirs() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "top.txt", "t");
        write_file(dir.path(), "sub/nested.txt", "n");
        let (inc, exc) = matchers(&["**/*"], &[]);
        let curr = scan(dir.path(), false, &inc, &exc).unwrap();
        assert_eq!(curr.len(), 1);
        assert!(curr.keys().next().unwrap().ends_with("top.txt"));
    }

    #[test]
    fn cursor_diff_round_trip_through_serde() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "a.txt", "a");
        let (inc, exc) = default_matchers();
        let curr = scan(dir.path(), true, &inc, &exc).unwrap();
        let as_cursor = serde_json::to_value(FilesystemFeedCursor {
            files: curr.clone(),
        })
        .unwrap();
        let back = parse_cursor(&as_cursor);
        assert_eq!(back, curr);
    }

    #[tokio::test]
    async fn run_writes_signal_jsonl_and_advances_cursor() {
        let watched = tempfile::tempdir().unwrap();
        write_file(watched.path(), "note.txt", "hello");
        let feed_dir = tempfile::tempdir().unwrap();

        let template = FilesystemFeedTemplate;
        let params = TemplateParams::new(
            serde_json::to_value(params_for(watched.path().to_path_buf())).unwrap(),
        );
        let ctx = TemplateCtx::noop();

        let outcome = template
            .run(&ctx, &params, feed_dir.path(), &Value::Null)
            .await
            .unwrap();
        assert_eq!(outcome.summary.items_written, 1);
        assert_eq!(outcome.status, "ok");

        let log = std::fs::read_to_string(feed_dir.path().join("signals.jsonl")).unwrap();
        let v: Value = serde_json::from_str(log.lines().next().unwrap()).unwrap();
        for k in [
            "source",
            "path",
            "rel_path",
            "event",
            "ts",
            "size_bytes",
            "mtime",
        ] {
            assert!(v.get(k).is_some(), "signal missing key '{k}'");
        }
        assert_eq!(v["event"], "created");
        assert_eq!(v["rel_path"], "note.txt");

        // Second run against the advanced cursor sees no changes.
        let outcome2 = template
            .run(&ctx, &params, feed_dir.path(), &outcome.cursor)
            .await
            .unwrap();
        assert_eq!(outcome2.summary.items_written, 0);
        assert_eq!(outcome2.status, "no-changes");
    }
}
