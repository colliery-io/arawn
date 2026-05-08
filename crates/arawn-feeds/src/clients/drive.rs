//! Drive — what feeds need from Google Drive, plus the production
//! adapter over `arawn-integrations` + `google-drive3`.
//!
//! Templates depend on the [`DriveFeedClient`] trait. Tests fake it
//! externally; production wires [`RealDriveClient`], which reuses the
//! same `GoogleDriveIntegration` (and persisted token) the existing
//! Drive tools use.

use std::sync::Arc;

use arawn_integrations::drive::GoogleDriveIntegration;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::FeedError;

/// One file as feeds care about it. Subset of Google's File resource —
/// only fields the templates actually read. Kept serializable so
/// `drive/recent` can write it verbatim to disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    /// RFC3339; Drive's canonical "last modified" value.
    pub modified_time: Option<String>,
    /// Drive's content checksum for binary uploads. `None` for Google
    /// native types (Docs/Sheets/Slides) — those use `modified_time`
    /// for change detection instead.
    pub md5_checksum: Option<String>,
    /// Parent folder ids — a Drive file can have multiple parents
    /// (shortcut/multi-parent). Templates pick the first for path
    /// reconstruction.
    pub parents: Vec<String>,
    pub size: Option<i64>,
    /// True if this is a folder (mime_type ==
    /// `application/vnd.google-apps.folder`). Pre-computed for
    /// readability.
    pub is_folder: bool,
}

const MIME_FOLDER: &str = "application/vnd.google-apps.folder";

impl DriveFile {
    pub fn folder_mime() -> &'static str {
        MIME_FOLDER
    }
}

/// What feeds need from Drive.
#[async_trait]
pub trait DriveFeedClient: Send + Sync {
    /// Resolve a folder spec — either a Drive folder id, the literal
    /// `"root"`, or a slash-delimited path under "My Drive" — to its
    /// folder id. Used at registration time so we can fail fast on a
    /// bad `folder` param.
    async fn resolve_folder(&self, path_or_id: &str) -> Result<String, FeedError>;

    /// List immediate (non-recursive) children of `folder_id`. Caller
    /// recurses; this keeps the trait simple and lets templates own
    /// walk semantics (cycle detection, depth caps). Excludes trashed.
    async fn list_folder_children(&self, folder_id: &str) -> Result<Vec<DriveFile>, FeedError>;

    /// List files modified after `since`, capped at `max_results`.
    /// Used by `drive/recent`. Drive returns most-recent-first.
    async fn list_modified_since(
        &self,
        since: DateTime<Utc>,
        max_results: u32,
    ) -> Result<Vec<DriveFile>, FeedError>;

    /// Download a file's bytes. For Google native types, the caller
    /// has already mapped to an `export_mime`; otherwise pass `None`
    /// for raw alt=media download.
    async fn download(
        &self,
        file_id: &str,
        export_mime: Option<&str>,
    ) -> Result<Vec<u8>, FeedError>;
}

/// Pick the export mime + filename suffix for Google native types.
/// Re-uses the policy from `arawn-integrations::drive::tools::drive_read`.
/// `None` means "raw alt=media download" (or unsupported native type
/// — caller decides whether that's an error or a skip).
pub fn export_for(mime: &str) -> Option<(&'static str, &'static str)> {
    match mime {
        "application/vnd.google-apps.document" => Some(("text/markdown", "md")),
        "application/vnd.google-apps.spreadsheet" => Some(("text/csv", "csv")),
        "application/vnd.google-apps.presentation" => Some(("text/plain", "txt")),
        "application/vnd.google-apps.drawing" => Some(("image/png", "png")),
        _ => None,
    }
}

/// True if `mime` is a Google native type with no export mapping
/// (forms, sites, scripts) — callers skip these to avoid noise.
pub fn is_unsupported_google_native(mime: &str) -> bool {
    mime.starts_with("application/vnd.google-apps.")
        && mime != MIME_FOLDER
        && export_for(mime).is_none()
}

// ─── Production adapter ──────────────────────────────────────────────

const FIELDS_LIST: &str =
    "nextPageToken,files(id,name,mimeType,size,modifiedTime,md5Checksum,parents,trashed)";
const FIELDS_ONE: &str =
    "id,name,mimeType,size,modifiedTime,md5Checksum,parents,trashed";

pub struct RealDriveClient {
    integration: Arc<GoogleDriveIntegration>,
}

impl RealDriveClient {
    pub fn new(integration: Arc<GoogleDriveIntegration>) -> Self {
        Self { integration }
    }
}

fn integ_err(e: arawn_integrations::IntegrationError) -> FeedError {
    use arawn_integrations::IntegrationError;
    match e {
        IntegrationError::NotConnected(msg) => FeedError::Auth(msg),
        other => FeedError::Provider(other.user_message()),
    }
}

fn google_err(op: &str, msg: String) -> FeedError {
    if msg.contains("rateLimitExceeded") || msg.contains("userRateLimitExceeded") {
        FeedError::RateLimited { retry_after: None }
    } else if msg.contains("invalid_grant")
        || msg.contains("token_expired")
        || msg.contains("unauthorized_client")
    {
        FeedError::Auth(format!("{op}: {msg}"))
    } else {
        FeedError::Provider(format!("{op}: {msg}"))
    }
}

fn from_api(f: google_drive3::api::File) -> DriveFile {
    let mime = f.mime_type.clone().unwrap_or_default();
    DriveFile {
        id: f.id.unwrap_or_default(),
        name: f.name.unwrap_or_default(),
        is_folder: mime == MIME_FOLDER,
        mime_type: mime,
        modified_time: f.modified_time.map(|t| t.to_rfc3339()),
        md5_checksum: f.md5_checksum,
        parents: f.parents.unwrap_or_default(),
        size: f.size,
    }
}

#[async_trait]
impl DriveFeedClient for RealDriveClient {
    async fn resolve_folder(&self, path_or_id: &str) -> Result<String, FeedError> {
        // Plain "root" or anything that parses as a Drive id (no
        // slashes, no spaces) — treat as id directly.
        if path_or_id == "root" {
            return Ok("root".into());
        }
        if !path_or_id.contains('/') {
            // Could be a literal id; verify by fetching metadata.
            let hub = self.integration.hub().map_err(integ_err)?;
            let (_, file) = hub
                .files()
                .get(path_or_id)
                .param("fields", FIELDS_ONE)
                .doit()
                .await
                .map_err(|e| google_err("files.get(folder)", e.to_string()))?;
            if file.mime_type.as_deref() != Some(MIME_FOLDER) {
                return Err(FeedError::InvalidParams(format!(
                    "'{path_or_id}' resolves to a file, not a folder"
                )));
            }
            return Ok(file.id.unwrap_or_default());
        }

        // Slash-delimited path under My Drive: walk one segment at a
        // time. Each segment becomes a `name = '<seg>' and '<parent>'
        // in parents and mimeType = '<folder-mime>'` query.
        let hub = self.integration.hub().map_err(integ_err)?;
        let mut current = "root".to_string();
        for segment in path_or_id.split('/').filter(|s| !s.is_empty()) {
            let escaped = segment.replace('\'', "\\'");
            let q = format!(
                "name = '{escaped}' and '{current}' in parents and \
                 mimeType = '{MIME_FOLDER}' and trashed = false"
            );
            let (_, resp) = hub
                .files()
                .list()
                .q(&q)
                .param("fields", FIELDS_LIST)
                .page_size(2)
                .doit()
                .await
                .map_err(|e| google_err("files.list(resolve)", e.to_string()))?;
            let mut iter = resp.files.unwrap_or_default().into_iter();
            let first = iter.next().ok_or_else(|| {
                FeedError::InvalidParams(format!(
                    "no folder named '{segment}' under id '{current}'"
                ))
            })?;
            current = first.id.unwrap_or_default();
        }
        Ok(current)
    }

    async fn list_folder_children(&self, folder_id: &str) -> Result<Vec<DriveFile>, FeedError> {
        let hub = self.integration.hub().map_err(integ_err)?;
        let q = format!("'{folder_id}' in parents and trashed = false");
        let mut all: Vec<DriveFile> = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut req = hub
                .files()
                .list()
                .q(&q)
                .param("fields", FIELDS_LIST)
                .page_size(200);
            if let Some(t) = page_token.as_deref() {
                req = req.page_token(t);
            }
            let (_, resp) = req
                .doit()
                .await
                .map_err(|e| google_err("files.list(children)", e.to_string()))?;
            for f in resp.files.unwrap_or_default() {
                all.push(from_api(f));
            }
            match resp.next_page_token {
                Some(t) if !t.is_empty() => page_token = Some(t),
                _ => break,
            }
        }
        Ok(all)
    }

    async fn list_modified_since(
        &self,
        since: DateTime<Utc>,
        max_results: u32,
    ) -> Result<Vec<DriveFile>, FeedError> {
        let hub = self.integration.hub().map_err(integ_err)?;
        let q = format!(
            "modifiedTime > '{}' and trashed = false",
            since.to_rfc3339()
        );
        let (_, resp) = hub
            .files()
            .list()
            .q(&q)
            .order_by("modifiedTime desc")
            .page_size(max_results.min(1000) as i32)
            .param("fields", FIELDS_LIST)
            .doit()
            .await
            .map_err(|e| google_err("files.list(modified)", e.to_string()))?;
        Ok(resp
            .files
            .unwrap_or_default()
            .into_iter()
            .map(from_api)
            .collect())
    }

    async fn download(
        &self,
        file_id: &str,
        export_mime: Option<&str>,
    ) -> Result<Vec<u8>, FeedError> {
        let hub = self.integration.hub().map_err(integ_err)?;
        let bytes = match export_mime {
            Some(mime) => {
                let resp = hub
                    .files()
                    .export(file_id, mime)
                    .doit()
                    .await
                    .map_err(|e| google_err("files.export", e.to_string()))?;
                http_body_util::BodyExt::collect(resp.into_body())
                    .await
                    .map_err(|e| FeedError::Provider(format!("export body read: {e}")))?
                    .to_bytes()
                    .to_vec()
            }
            None => {
                let (resp, _) = hub
                    .files()
                    .get(file_id)
                    .param("alt", "media")
                    .doit()
                    .await
                    .map_err(|e| google_err("files.get(media)", e.to_string()))?;
                http_body_util::BodyExt::collect(resp.into_body())
                    .await
                    .map_err(|e| FeedError::Provider(format!("media body read: {e}")))?
                    .to_bytes()
                    .to_vec()
            }
        };
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_for_covers_known_natives() {
        assert_eq!(
            export_for("application/vnd.google-apps.document"),
            Some(("text/markdown", "md"))
        );
        assert_eq!(
            export_for("application/vnd.google-apps.spreadsheet"),
            Some(("text/csv", "csv"))
        );
        assert!(export_for("application/pdf").is_none());
    }

    #[test]
    fn unsupported_native_excludes_folders_and_known_exports() {
        assert!(!is_unsupported_google_native(MIME_FOLDER));
        assert!(!is_unsupported_google_native(
            "application/vnd.google-apps.document"
        ));
        assert!(is_unsupported_google_native(
            "application/vnd.google-apps.form"
        ));
        assert!(!is_unsupported_google_native("text/plain"));
    }
}
