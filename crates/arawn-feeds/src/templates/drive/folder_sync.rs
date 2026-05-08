//! `drive/folder-sync` — rsync-style mirror of a Drive folder onto
//! local disk. One-way pull: remote is the source of truth; the
//! local mirror gets new/updated files written, deleted files
//! removed, and unchanged files skipped.
//!
//! Required param:
//! - `folder: string` — Drive folder id, the literal `"root"`, or a
//!   slash-delimited path under "My Drive" (e.g. `"Reports/2026"`).
//!   Resolved to a folder id at registration time so a typo fails
//!   fast.
//!
//! Disk layout:
//!
//! ```text
//! drive/folder-sync/<feed_id>/
//!   ├── meta.json             # cursor: { files: { <file_id>: { token, path } } }
//!   ├── <subfolder>/
//!   │   └── <file>            # native bytes
//!   └── <file>
//! ```
//!
//! Storage model:
//!
//! - One body per remote file at the same relative path Drive uses.
//! - Google native types (Docs/Sheets/Slides/Drawings) are exported
//!   per [`crate::clients::export_for`]; the export's filename gets a
//!   matching extension (`.md`, `.csv`, etc.) so file managers can
//!   open them.
//! - Unsupported native types (forms, sites, scripts) are skipped
//!   with a warn-level log; they have no usable export.
//! - Unchanged files (token matches the cursor) are skipped — no
//!   download, no rewrite.
//! - Files present locally but absent from the remote walk are
//!   deleted on the way out (mirror semantics).
//!
//! The cursor is keyed by Drive `file_id`, so renames and moves are
//! handled correctly: when a file's path changes, we delete the old
//! local path and write the new one in the same run.
//!
//! Cycle protection: Drive folders form a DAG (a file can have
//! multiple parents), but for a sync the user picked, treating it as
//! a tree is fine. A small visited-set prevents infinite recursion if
//! the API ever surprises us.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::common::{change_token, is_under, sanitize_path_component};
use crate::clients::{DriveFeedClient, DriveFile, export_for, is_unsupported_google_native};
use crate::error::FeedError;
use crate::template::{FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct FolderSyncTemplate;

const NAME: &str = "drive/folder-sync";
/// Cap recursion to keep a misbehaving folder graph from spinning
/// forever. Real folder trees rarely exceed ~10 levels.
const MAX_DEPTH: usize = 32;

#[derive(Debug, Default, Serialize, Deserialize)]
struct Cursor {
    /// Map of remote file_id → on-disk state. Keys are Drive ids;
    /// values track the last-seen change token + the relative path
    /// we wrote the file to.
    files: BTreeMap<String, FileEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FileEntry {
    /// The change-detection token from the last successful write.
    /// Same value for the same content; templates compare equality.
    token: String,
    /// Relative path under feed_dir/, with platform-native separators.
    path: String,
}

#[async_trait]
impl FeedTemplate for FolderSyncTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        let folder = params
            .0
            .get("folder")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::InvalidParams("missing required param: folder".into()))?;
        if folder.trim().is_empty() {
            return Err(FeedError::InvalidParams("folder must not be empty".into()));
        }
        Ok(())
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: "0 * * * *".into(), // hourly
            initial_cursor: json!({ "files": {} }),
        }
    }

    async fn run(
        &self,
        ctx: &TemplateCtx,
        params: &TemplateParams,
        feed_dir: &Path,
        cursor: &Value,
    ) -> Result<RunOutcome, FeedError> {
        let started = Instant::now();
        let drive = ctx.clients().drive().ok_or_else(|| {
            FeedError::Auth("google drive integration not connected".into())
        })?;

        let folder_spec = params
            .0
            .get("folder")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::InvalidParams("missing folder".into()))?;
        let folder_id = drive.resolve_folder(folder_spec).await?;

        let prior: Cursor = serde_json::from_value(cursor.clone()).unwrap_or_default();

        // ── 1. Walk remote tree, collect remote_files keyed by id ──
        let mut remote: BTreeMap<String, RemoteFile> = BTreeMap::new();
        let mut visited: HashSet<String> = HashSet::new();
        walk(
            drive.clone(),
            &folder_id,
            PathBuf::new(),
            0,
            &mut remote,
            &mut visited,
        )
        .await?;

        // ── 2. Diff against cursor; download new/changed files ──
        let mut new_files: BTreeMap<String, FileEntry> = BTreeMap::new();
        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut skipped_unsupported: u64 = 0;

        for (id, remote_file) in &remote {
            if remote_file.file.is_folder {
                continue;
            }
            if is_unsupported_google_native(&remote_file.file.mime_type) {
                tracing::warn!(
                    target: "arawn::feeds",
                    %id,
                    mime = %remote_file.file.mime_type,
                    name = %remote_file.file.name,
                    "skipping drive native type with no export mapping"
                );
                skipped_unsupported += 1;
                continue;
            }

            let token = change_token(
                remote_file.file.md5_checksum.as_deref(),
                remote_file.file.modified_time.as_deref(),
            );
            let target_rel = remote_file.relative_path.clone();
            let target_abs = feed_dir.join(&target_rel);

            let prior_entry = prior.files.get(id);
            let unchanged = prior_entry.is_some_and(|p| p.token == token && p.path == target_rel);

            if !unchanged {
                // If the file moved, delete the old path before
                // writing the new one.
                if let Some(p) = prior_entry
                    && p.path != target_rel
                {
                    let old_abs = feed_dir.join(&p.path);
                    safe_remove_file(feed_dir, &old_abs)?;
                }

                if let Some(parent) = target_abs.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        FeedError::Storage(format!(
                            "create {}: {e}",
                            parent.display()
                        ))
                    })?;
                }

                let export = export_for(&remote_file.file.mime_type).map(|(m, _)| m);
                let bytes = drive.download(id, export).await?;
                if !is_under(feed_dir, &target_abs) {
                    return Err(FeedError::Storage(format!(
                        "refusing to write outside feed_dir: {}",
                        target_abs.display()
                    )));
                }
                atomic_write(&target_abs, &bytes)?;
                total_items += 1;
                total_bytes += bytes.len() as u64;
            }

            new_files.insert(
                id.clone(),
                FileEntry {
                    token,
                    path: target_rel,
                },
            );
        }

        // ── 3. Reap local files no longer present remotely ─────────
        let mut deletions: u64 = 0;
        for (id, entry) in &prior.files {
            if !new_files.contains_key(id) {
                let abs = feed_dir.join(&entry.path);
                if abs.exists() {
                    safe_remove_file(feed_dir, &abs)?;
                    deletions += 1;
                }
            }
        }
        // Best-effort: prune now-empty subdirs left by deletions.
        // Walk top-down and remove any empty dir under feed_dir.
        prune_empty_dirs(feed_dir);

        let new_cursor = json!({ "files": new_files });
        let status = match (total_items, deletions) {
            (0, 0) => "no-new-items".to_string(),
            _ => "ok".to_string(),
        };

        if skipped_unsupported > 0 {
            tracing::info!(
                target: "arawn::feeds",
                feed = NAME,
                skipped_unsupported,
                "folder-sync skipped unsupported google native files"
            );
        }

        Ok(RunOutcome {
            cursor: new_cursor,
            summary: RunSummary {
                items_written: total_items,
                bytes_written: total_bytes,
                duration: started.elapsed(),
            },
            status,
        })
    }
}

#[derive(Debug, Clone)]
struct RemoteFile {
    file: DriveFile,
    /// Relative path under feed_dir/. Uses native path separators.
    relative_path: String,
}

/// Recursively walk a Drive folder, collecting every file (not
/// folder) into `out` keyed by Drive id.
fn walk<'a>(
    drive: Arc<dyn DriveFeedClient>,
    folder_id: &'a str,
    rel_prefix: PathBuf,
    depth: usize,
    out: &'a mut BTreeMap<String, RemoteFile>,
    visited: &'a mut HashSet<String>,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<(), FeedError>> + Send + 'a>,
> {
    let folder_id = folder_id.to_string();
    Box::pin(async move {
        if depth >= MAX_DEPTH {
            tracing::warn!(
                target: "arawn::feeds",
                folder_id = %folder_id,
                "folder-sync max depth reached, stopping recursion"
            );
            return Ok(());
        }
        if !visited.insert(folder_id.clone()) {
            return Ok(()); // already walked
        }
        let children = drive.list_folder_children(&folder_id).await?;
        for child in children {
            let safe_name = sanitize_path_component(&child.name);
            let mut here = rel_prefix.clone();
            here.push(&safe_name);
            if child.is_folder {
                walk(drive.clone(), &child.id, here, depth + 1, out, visited).await?;
            } else {
                let mut path = here;
                if let Some((_, ext)) = export_for(&child.mime_type) {
                    let with_ext = format!(
                        "{}.{ext}",
                        path.file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default()
                    );
                    path.set_file_name(with_ext);
                }
                out.insert(
                    child.id.clone(),
                    RemoteFile {
                        file: child,
                        relative_path: path.to_string_lossy().to_string(),
                    },
                );
            }
        }
        Ok(())
    })
}

fn atomic_write(path: &Path, body: &[u8]) -> Result<(), FeedError> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, body)
        .map_err(|e| FeedError::Storage(format!("write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| FeedError::Storage(format!("rename {}: {e}", path.display())))?;
    Ok(())
}

fn safe_remove_file(feed_dir: &Path, path: &Path) -> Result<(), FeedError> {
    if !is_under(feed_dir, path) {
        return Err(FeedError::Storage(format!(
            "refusing to remove outside feed_dir: {}",
            path.display()
        )));
    }
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| {
            FeedError::Storage(format!("remove {}: {e}", path.display()))
        })?;
    }
    Ok(())
}

fn prune_empty_dirs(root: &Path) {
    // Iterate post-order: remove children first, then parents.
    let entries = match std::fs::read_dir(root) {
        Ok(it) => it,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            prune_empty_dirs(&p);
            if let Ok(mut it) = std::fs::read_dir(&p)
                && it.next().is_none()
            {
                let _ = std::fs::remove_dir(&p);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_requires_folder() {
        assert!(FolderSyncTemplate
            .validate(&TemplateParams::default())
            .is_err());
        let p = TemplateParams(json!({ "folder": "" }));
        assert!(FolderSyncTemplate.validate(&p).is_err());
        let p = TemplateParams(json!({ "folder": "root" }));
        FolderSyncTemplate.validate(&p).unwrap();
    }

    #[test]
    fn defaults_use_hourly_cadence() {
        let d = FolderSyncTemplate.defaults(&TemplateParams::default());
        assert_eq!(d.cadence, "0 * * * *");
    }
}
