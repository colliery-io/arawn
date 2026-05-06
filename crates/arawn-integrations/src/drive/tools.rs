//! Engine tools wrapping Google Drive.
//!
//! Seven tools land in v1 (read + write):
//! - `drive_search` — query by Google's Drive query syntax
//! - `drive_list` — list root or a folder
//! - `drive_get_metadata` — full file metadata
//! - `drive_read` — content (Google Docs/Sheets/Slides via export, others raw)
//! - `drive_upload` — create a new file
//! - `drive_update` — overwrite content
//! - `drive_delete` — trash (recoverable) — does not permadelete

use std::io::Cursor;
use std::sync::Arc;

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use google_drive3::api::File as DriveFile;
use serde::Serialize;
use serde_json::{Value, json};

use super::integration::GoogleDriveIntegration;

fn integ_err(e: crate::IntegrationError) -> ToolError {
    ToolError::ExecutionFailed(e.user_message())
}

fn google_err(stage: &str, e: google_drive3::Error) -> ToolError {
    ToolError::ExecutionFailed(format!("Drive {stage}: {e}"))
}

/// Compact file row used by list / search / get-metadata. Drops fields
/// that aren't useful for the agent's reasoning (etag, headRevisionId, etc.).
#[derive(Debug, Clone, Serialize)]
struct FileSummary {
    id: Option<String>,
    name: Option<String>,
    mime_type: Option<String>,
    size: Option<String>,
    modified_time: Option<String>,
    web_view_link: Option<String>,
    /// Set on get_metadata, omitted on list/search.
    #[serde(skip_serializing_if = "Option::is_none")]
    parents: Option<Vec<String>>,
    /// Email addresses of the file's owners.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    owners: Vec<String>,
    /// Drive's recoverable-delete flag — if true, the file is in trash.
    #[serde(skip_serializing_if = "Option::is_none")]
    trashed: Option<bool>,
}

fn summarize_file(f: &DriveFile, include_parents: bool) -> FileSummary {
    FileSummary {
        id: f.id.clone(),
        name: f.name.clone(),
        mime_type: f.mime_type.clone(),
        size: f.size.as_ref().map(|s| s.to_string()),
        modified_time: f.modified_time.map(|t| t.to_rfc3339()),
        web_view_link: f.web_view_link.clone(),
        parents: if include_parents {
            f.parents.clone()
        } else {
            None
        },
        owners: f
            .owners
            .as_ref()
            .map(|os| os.iter().filter_map(|o| o.email_address.clone()).collect())
            .unwrap_or_default(),
        trashed: f.trashed,
    }
}

/// Standard projection passed to `fields` so we get the same shape across
/// search/list/get. Trailing parens scope each FileList vs File response.
const FILE_FIELDS_LIST: &str = "nextPageToken,files(id,name,mimeType,size,modifiedTime,webViewLink,owners,trashed)";
const FILE_FIELDS_ONE: &str = "id,name,mimeType,size,modifiedTime,webViewLink,owners,trashed,parents";

/// Cap returned content for `drive_read` so a 50MB binary doesn't fill the
/// LLM's tool result. Configurable per-call up to this ceiling.
const DRIVE_READ_DEFAULT_MAX_BYTES: usize = 1_000_000; // 1 MB
const DRIVE_READ_HARD_MAX_BYTES: usize = 5_000_000; // 5 MB

// ─── /drive_search ────────────────────────────────────────────────────────

pub struct DriveSearchTool {
    integration: Arc<GoogleDriveIntegration>,
}

impl DriveSearchTool {
    pub fn new(integration: Arc<GoogleDriveIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for DriveSearchTool {
    fn name(&self) -> &str {
        "drive_search"
    }
    fn description(&self) -> &str {
        "Search Google Drive using Drive's query syntax. Examples: \
         `name contains 'budget'`, `mimeType = 'application/pdf'`, \
         `modifiedTime > '2026-01-01T00:00:00'`, `'<folder_id>' in parents`. \
         Combine with `and` / `or`. Returns id, name, mime_type, size, \
         modified_time, web_view_link, owners. Use drive_get_metadata \
         for full metadata or drive_read for content. Excludes trashed \
         files by default (the query is `and`-joined with `trashed=false`)."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Drive query string. See https://developers.google.com/drive/api/guides/search-files"
                },
                "page_size": {
                    "type": "integer",
                    "description": "Max results per page (default 50, max 100)",
                    "minimum": 1,
                    "maximum": 100
                },
                "page_token": {
                    "type": "string",
                    "description": "Continuation token from a previous response"
                },
                "include_trashed": {
                    "type": "boolean",
                    "description": "Include trashed files (default false)"
                }
            },
            "required": ["query"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let user_query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'query'".into()))?
            .to_string();
        let page_size = params
            .get("page_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(50)
            .min(100) as i32;
        let include_trashed = params
            .get("include_trashed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let page_token = params.get("page_token").and_then(|v| v.as_str());

        let final_query = if include_trashed {
            user_query
        } else {
            format!("({user_query}) and trashed = false")
        };

        let hub = self.integration.hub().map_err(integ_err)?;
        let mut req = hub
            .files()
            .list()
            .q(&final_query)
            .page_size(page_size)
            .param("fields", FILE_FIELDS_LIST);
        if let Some(token) = page_token {
            req = req.page_token(token);
        }
        let (_, resp) = req.doit().await.map_err(|e| google_err("files.list", e))?;
        let files: Vec<FileSummary> = resp
            .files
            .unwrap_or_default()
            .iter()
            .map(|f| summarize_file(f, false))
            .collect();
        let payload = json!({
            "files": files,
            "next_page_token": resp.next_page_token,
        });
        Ok(ToolOutput::success(payload.to_string()))
    }
}

// ─── /drive_list ──────────────────────────────────────────────────────────

pub struct DriveListTool {
    integration: Arc<GoogleDriveIntegration>,
}

impl DriveListTool {
    pub fn new(integration: Arc<GoogleDriveIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for DriveListTool {
    fn name(&self) -> &str {
        "drive_list"
    }
    fn description(&self) -> &str {
        "List the contents of a Drive folder. With no `folder_id`, lists the \
         user's My Drive root. Returns the same fields as drive_search. \
         Excludes trashed files by default."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "folder_id": {
                    "type": "string",
                    "description": "Folder file id. Omit to list root."
                },
                "page_size": {
                    "type": "integer",
                    "description": "Max results per page (default 50, max 100)",
                    "minimum": 1,
                    "maximum": 100
                },
                "page_token": {
                    "type": "string",
                    "description": "Continuation token from a previous response"
                }
            }
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let folder_id = params
            .get("folder_id")
            .and_then(|v| v.as_str())
            .unwrap_or("root");
        let page_size = params
            .get("page_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(50)
            .min(100) as i32;
        let page_token = params.get("page_token").and_then(|v| v.as_str());

        let query = format!("'{folder_id}' in parents and trashed = false");

        let hub = self.integration.hub().map_err(integ_err)?;
        let mut req = hub
            .files()
            .list()
            .q(&query)
            .page_size(page_size)
            .param("fields", FILE_FIELDS_LIST);
        if let Some(token) = page_token {
            req = req.page_token(token);
        }
        let (_, resp) = req.doit().await.map_err(|e| google_err("files.list", e))?;
        let files: Vec<FileSummary> = resp
            .files
            .unwrap_or_default()
            .iter()
            .map(|f| summarize_file(f, false))
            .collect();
        let payload = json!({
            "files": files,
            "next_page_token": resp.next_page_token,
        });
        Ok(ToolOutput::success(payload.to_string()))
    }
}

// ─── /drive_get_metadata ──────────────────────────────────────────────────

pub struct DriveGetMetadataTool {
    integration: Arc<GoogleDriveIntegration>,
}

impl DriveGetMetadataTool {
    pub fn new(integration: Arc<GoogleDriveIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for DriveGetMetadataTool {
    fn name(&self) -> &str {
        "drive_get_metadata"
    }
    fn description(&self) -> &str {
        "Get full metadata for a single Drive file by id. Returns id, \
         name, mime_type, size, modified_time, web_view_link, owners, \
         parents, trashed."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_id": { "type": "string", "description": "Drive file id" }
            },
            "required": ["file_id"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let file_id = params
            .get("file_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'file_id'".into()))?;

        let hub = self.integration.hub().map_err(integ_err)?;
        let (_, file) = hub
            .files()
            .get(file_id)
            .param("fields", FILE_FIELDS_ONE)
            .doit()
            .await
            .map_err(|e| google_err("files.get", e))?;
        let summary = summarize_file(&file, true);
        Ok(ToolOutput::success(serde_json::to_string(&summary).unwrap()))
    }
}

// ─── /drive_read ──────────────────────────────────────────────────────────

pub struct DriveReadTool {
    integration: Arc<GoogleDriveIntegration>,
}

impl DriveReadTool {
    pub fn new(integration: Arc<GoogleDriveIntegration>) -> Self {
        Self { integration }
    }
}

/// Pick the export format for Google's native types. For everything else
/// we download the raw bytes (handled in execute).
fn export_mime_for(google_mime: &str) -> Option<&'static str> {
    match google_mime {
        "application/vnd.google-apps.document" => Some("text/markdown"),
        "application/vnd.google-apps.spreadsheet" => Some("text/csv"),
        "application/vnd.google-apps.presentation" => Some("text/plain"),
        "application/vnd.google-apps.drawing" => Some("image/png"),
        // No sensible export for forms / sites / scripts; tool will error
        // and tell the LLM to ask the user to convert manually.
        _ => None,
    }
}

#[async_trait]
impl Tool for DriveReadTool {
    fn name(&self) -> &str {
        "drive_read"
    }
    fn description(&self) -> &str {
        "Read the content of a Drive file. Google Docs are exported as \
         markdown, Sheets as CSV, Slides as plain text. Other types \
         (PDF, images, plain text) are downloaded raw — text-like \
         types decoded as UTF-8, binary returned as base64. Caps \
         response at 1MB by default (configurable up to 5MB)."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_id": { "type": "string", "description": "Drive file id" },
                "max_bytes": {
                    "type": "integer",
                    "description": "Cap bytes returned (default 1MB, max 5MB)",
                    "minimum": 1,
                    "maximum": DRIVE_READ_HARD_MAX_BYTES
                }
            },
            "required": ["file_id"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let file_id = params
            .get("file_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'file_id'".into()))?
            .to_string();
        let max_bytes = params
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .map(|n| (n as usize).min(DRIVE_READ_HARD_MAX_BYTES))
            .unwrap_or(DRIVE_READ_DEFAULT_MAX_BYTES);

        let hub = self.integration.hub().map_err(integ_err)?;

        // First, get metadata to learn the mime type.
        let (_, file) = hub
            .files()
            .get(&file_id)
            .param("fields", "id,name,mimeType,size")
            .doit()
            .await
            .map_err(|e| google_err("files.get", e))?;
        let mime = file.mime_type.clone().unwrap_or_default();

        // Dispatch: Google native → export (returns just Response, no
        // parsed body), otherwise → download raw via alt=media (returns
        // tuple but the second element is File::default()).
        let (bytes, response_mime) = if let Some(export_mime) = export_mime_for(&mime) {
            let resp = hub
                .files()
                .export(&file_id, export_mime)
                .doit()
                .await
                .map_err(|e| google_err("files.export", e))?;
            let body = http_body_util::BodyExt::collect(resp.into_body())
                .await
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("Drive export body read: {e}"))
                })?
                .to_bytes();
            (body.to_vec(), export_mime.to_string())
        } else if mime.starts_with("application/vnd.google-apps.") {
            // Native Google type with no export mapping (forms, sites, scripts).
            return Err(ToolError::ExecutionFailed(format!(
                "Cannot export Google native type '{mime}'. Open the file \
                 in your browser via web_view_link and convert/copy content manually."
            )));
        } else {
            let (resp, _) = hub
                .files()
                .get(&file_id)
                .param("alt", "media")
                .doit()
                .await
                .map_err(|e| google_err("files.get media", e))?;
            let body = http_body_util::BodyExt::collect(resp.into_body())
                .await
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("Drive download body read: {e}"))
                })?
                .to_bytes();
            (body.to_vec(), mime.clone())
        };

        let truncated = bytes.len() > max_bytes;
        let trimmed = if truncated {
            bytes[..max_bytes].to_vec()
        } else {
            bytes
        };

        // Try UTF-8 for text-like mime types; fall back to base64.
        let is_texty = response_mime.starts_with("text/")
            || response_mime == "application/json"
            || response_mime == "application/xml";
        let payload = if is_texty
            && let Ok(s) = String::from_utf8(trimmed.clone())
        {
            json!({
                "file_id": file_id,
                "name": file.name,
                "mime_type": response_mime,
                "encoding": "utf-8",
                "content": s,
                "truncated": truncated,
            })
        } else {
            json!({
                "file_id": file_id,
                "name": file.name,
                "mime_type": response_mime,
                "encoding": "base64",
                "content": B64.encode(&trimmed),
                "truncated": truncated,
            })
        };

        Ok(ToolOutput::success(payload.to_string()))
    }
}

// ─── /drive_upload ────────────────────────────────────────────────────────

pub struct DriveUploadTool {
    integration: Arc<GoogleDriveIntegration>,
}

impl DriveUploadTool {
    pub fn new(integration: Arc<GoogleDriveIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for DriveUploadTool {
    fn name(&self) -> &str {
        "drive_upload"
    }
    fn description(&self) -> &str {
        "Upload a new file to Drive. `content` is the file body; for \
         binary types, base64-encode it and set `encoding: 'base64'` \
         (default 'utf-8'). `mime_type` defaults to text/plain. \
         `parent_folder_id` defaults to the user's root."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::Other
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "File name (e.g. 'meeting-notes.md')" },
                "content": { "type": "string", "description": "File body — utf-8 text or base64" },
                "mime_type": {
                    "type": "string",
                    "description": "Content MIME type (default text/plain)"
                },
                "encoding": {
                    "type": "string",
                    "enum": ["utf-8", "base64"],
                    "description": "How `content` is encoded (default utf-8)"
                },
                "parent_folder_id": {
                    "type": "string",
                    "description": "Drive folder id (omit for root)"
                }
            },
            "required": ["name", "content"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'name'".into()))?
            .to_string();
        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'content'".into()))?;
        let mime_type = params
            .get("mime_type")
            .and_then(|v| v.as_str())
            .unwrap_or("text/plain")
            .to_string();
        let encoding = params
            .get("encoding")
            .and_then(|v| v.as_str())
            .unwrap_or("utf-8");
        let parent = params
            .get("parent_folder_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let bytes = match encoding {
            "base64" => B64
                .decode(content)
                .map_err(|e| ToolError::ExecutionFailed(format!("base64 decode: {e}")))?,
            _ => content.as_bytes().to_vec(),
        };

        let metadata = DriveFile {
            name: Some(name.clone()),
            parents: parent.map(|p| vec![p]),
            ..Default::default()
        };

        let mime: mime::Mime = mime_type
            .parse()
            .map_err(|e| ToolError::ExecutionFailed(format!("invalid mime_type: {e}")))?;
        let hub = self.integration.hub().map_err(integ_err)?;
        let (_, file) = hub
            .files()
            .create(metadata)
            .param("fields", FILE_FIELDS_ONE)
            .upload(Cursor::new(bytes), mime)
            .await
            .map_err(|e| google_err("files.create", e))?;

        let summary = summarize_file(&file, true);
        Ok(ToolOutput::success(serde_json::to_string(&summary).unwrap()))
    }
}

// ─── /drive_update ────────────────────────────────────────────────────────

pub struct DriveUpdateTool {
    integration: Arc<GoogleDriveIntegration>,
}

impl DriveUpdateTool {
    pub fn new(integration: Arc<GoogleDriveIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for DriveUpdateTool {
    fn name(&self) -> &str {
        "drive_update"
    }
    fn description(&self) -> &str {
        "Overwrite the content of an existing Drive file. Preserves \
         metadata (name, parents, etc.). Use drive_get_metadata first \
         if you need to confirm the right file. `encoding` defaults to \
         utf-8; pass 'base64' for binary content."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::Other
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_id": { "type": "string", "description": "Drive file id" },
                "content": { "type": "string", "description": "New file body" },
                "mime_type": {
                    "type": "string",
                    "description": "Content MIME type (default text/plain)"
                },
                "encoding": {
                    "type": "string",
                    "enum": ["utf-8", "base64"],
                    "description": "How `content` is encoded (default utf-8)"
                }
            },
            "required": ["file_id", "content"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let file_id = params
            .get("file_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'file_id'".into()))?
            .to_string();
        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'content'".into()))?;
        let mime_type = params
            .get("mime_type")
            .and_then(|v| v.as_str())
            .unwrap_or("text/plain")
            .to_string();
        let encoding = params
            .get("encoding")
            .and_then(|v| v.as_str())
            .unwrap_or("utf-8");

        let bytes = match encoding {
            "base64" => B64
                .decode(content)
                .map_err(|e| ToolError::ExecutionFailed(format!("base64 decode: {e}")))?,
            _ => content.as_bytes().to_vec(),
        };

        let mime: mime::Mime = mime_type
            .parse()
            .map_err(|e| ToolError::ExecutionFailed(format!("invalid mime_type: {e}")))?;
        let hub = self.integration.hub().map_err(integ_err)?;
        let (_, file) = hub
            .files()
            .update(DriveFile::default(), &file_id)
            .param("fields", FILE_FIELDS_ONE)
            .upload(Cursor::new(bytes), mime)
            .await
            .map_err(|e| google_err("files.update", e))?;

        let summary = summarize_file(&file, true);
        Ok(ToolOutput::success(serde_json::to_string(&summary).unwrap()))
    }
}

// ─── /drive_delete ────────────────────────────────────────────────────────

pub struct DriveDeleteTool {
    integration: Arc<GoogleDriveIntegration>,
}

impl DriveDeleteTool {
    pub fn new(integration: Arc<GoogleDriveIntegration>) -> Self {
        Self { integration }
    }
}

#[async_trait]
impl Tool for DriveDeleteTool {
    fn name(&self) -> &str {
        "drive_delete"
    }
    fn description(&self) -> &str {
        "Move a Drive file to trash. Recoverable for 30 days from the \
         Drive UI. Does NOT permanently delete — Drive's API has a \
         separate `files.delete` for that, which we deliberately don't \
         expose because the agent shouldn't have the ability to \
         permadelete user data."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Web
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::FileWrite
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_id": { "type": "string", "description": "Drive file id" }
            },
            "required": ["file_id"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        let file_id = params
            .get("file_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'file_id'".into()))?
            .to_string();

        let hub = self.integration.hub().map_err(integ_err)?;
        let metadata = DriveFile {
            trashed: Some(true),
            ..Default::default()
        };
        let (_, file) = hub
            .files()
            .update(metadata, &file_id)
            .param("fields", FILE_FIELDS_ONE)
            .doit_without_upload()
            .await
            .map_err(|e| google_err("files.update (trash)", e))?;
        Ok(ToolOutput::success(
            json!({
                "trashed": true,
                "file": summarize_file(&file, true),
            })
            .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_mime_dispatch_covers_known_google_types() {
        assert_eq!(
            export_mime_for("application/vnd.google-apps.document"),
            Some("text/markdown")
        );
        assert_eq!(
            export_mime_for("application/vnd.google-apps.spreadsheet"),
            Some("text/csv")
        );
        assert_eq!(
            export_mime_for("application/vnd.google-apps.presentation"),
            Some("text/plain")
        );
        // Non-Google types fall through to None.
        assert_eq!(export_mime_for("application/pdf"), None);
        assert_eq!(export_mime_for("text/plain"), None);
    }

    #[test]
    fn summarize_file_extracts_owner_emails() {
        use google_drive3::api::User;
        let mut owner = User::default();
        owner.email_address = Some("alice@example.com".into());
        let mut f = DriveFile::default();
        f.id = Some("abc".into());
        f.name = Some("notes.md".into());
        f.mime_type = Some("text/markdown".into());
        f.owners = Some(vec![owner]);
        let s = summarize_file(&f, false);
        assert_eq!(s.id.as_deref(), Some("abc"));
        assert_eq!(s.name.as_deref(), Some("notes.md"));
        assert_eq!(s.owners, vec!["alice@example.com".to_string()]);
        assert!(s.parents.is_none()); // include_parents=false
    }

    #[test]
    fn summarize_file_includes_parents_when_requested() {
        let mut f = DriveFile::default();
        f.parents = Some(vec!["folder1".into()]);
        let s = summarize_file(&f, true);
        assert_eq!(s.parents.as_deref(), Some(&["folder1".to_string()][..]));
    }
}
