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
//!
//! And two for doc comments (COLLIERY-I-0533):
//! - `drive_list_open_comments` — the open threads for me
//! - `drive_reply_to_comment` — reply to a thread; never resolves it

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use google_drive3::api::File as DriveFile;
use serde::Serialize;
use serde_json::{Value, json};

use super::comments::{self, CommentsApi, DocComment, HubCommentsApi};
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
const FILE_FIELDS_LIST: &str =
    "nextPageToken,files(id,name,mimeType,size,modifiedTime,webViewLink,owners,trashed)";
const FILE_FIELDS_ONE: &str =
    "id,name,mimeType,size,modifiedTime,webViewLink,owners,trashed,parents";

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
        ToolCategory::Drive
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
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
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
        ToolCategory::Drive
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
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
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
        ToolCategory::Drive
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
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
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
        Ok(ToolOutput::success(
            serde_json::to_string(&summary).unwrap(),
        ))
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
        ToolCategory::Drive
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
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
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
                .map_err(|e| ToolError::ExecutionFailed(format!("Drive export body read: {e}")))?
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
                .map_err(|e| ToolError::ExecutionFailed(format!("Drive download body read: {e}")))?
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
        let payload = if is_texty && let Ok(s) = String::from_utf8(trimmed.clone()) {
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
        ToolCategory::Drive
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
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
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
        Ok(ToolOutput::success(
            serde_json::to_string(&summary).unwrap(),
        ))
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
        ToolCategory::Drive
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
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
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
        Ok(ToolOutput::success(
            serde_json::to_string(&summary).unwrap(),
        ))
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
        ToolCategory::Drive
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
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
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

// ─── /drive_list_open_comments ────────────────────────────────────────────

/// One open thread as the comment tools give it.
fn comment_summary(
    file_id: &str,
    doc_title: &str,
    link: Option<String>,
    c: &DocComment,
    reason: Option<&str>,
) -> Value {
    let last = c.last_reply();
    json!({
        "file_id": file_id,
        "comment_id": c.id,
        "doc_title": doc_title,
        "reason": reason,
        "author": c.author,
        "content": c.content,
        "quoted": c.quoted,
        "reply_count": c.live_replies().count(),
        "last_reply_author": last.and_then(|r| r.author.as_deref()),
        "modified_time": c.modified_time,
        "link": link,
    })
}

/// The `drive_comments` rows, newest first, in the shape of
/// [`comment_summary`]. `file_id` keeps one doc's threads. Shared with
/// the UAT tool.
pub(crate) fn open_comments_from_projection(
    data_dir: &Path,
    file_id: Option<&str>,
    limit: usize,
) -> Result<Vec<Value>, ToolError> {
    use arawn_projections::drive_comments::FEED_TYPE;
    let store = arawn_projections::ProjectionStore::open(&data_dir.join("projections.db"))
        .map_err(|e| ToolError::ExecutionFailed(format!("open projections: {e}")))?;
    store
        .ensure_feed_type(FEED_TYPE)
        .map_err(|e| ToolError::ExecutionFailed(format!("ensure schema: {e}")))?;
    let conn = store.conn().lock().unwrap();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT source_ts, metadata FROM {FEED_TYPE} \
             WHERE ?1 IS NULL OR json_extract(metadata, '$.file_id') = ?1 \
             ORDER BY source_ts DESC LIMIT ?2"
        ))
        .map_err(|e| ToolError::ExecutionFailed(format!("prepare: {e}")))?;
    let rows = stmt
        .query_map(rusqlite::params![file_id, limit as i64], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| ToolError::ExecutionFailed(format!("query: {e}")))?;
    let mut out = Vec::new();
    for r in rows {
        let (ts, meta) = r.map_err(|e| ToolError::ExecutionFailed(format!("row: {e}")))?;
        let m: Value = serde_json::from_str(&meta).unwrap_or(Value::Null);
        out.push(json!({
            "file_id": m["file_id"],
            "comment_id": m["comment_id"],
            "doc_title": m["doc_title"],
            "reason": m["reason"],
            "author": m["author"],
            "content": m["content"],
            "quoted": m["quoted"],
            "reply_count": m["reply_count"],
            "last_reply_author": m["last_reply_author"],
            "modified_time": ts,
            "link": m["link"],
        }));
    }
    Ok(out)
}

pub(crate) const LIST_OPEN_COMMENTS_DESCRIPTION: &str = "List the open Google Docs, Sheets and Slides comment threads that are for me: \
     assigned to me, mentioning me, or on a doc I own and not answered by me. Reads \
     what the drive/comments feed has stored, newest first. With `file_id`, asks \
     Drive now for that doc's open threads (each with `reason`, null when the \
     thread is not for me). Gives doc title, author, text, reason and a link that \
     opens the doc on the comment.";

pub(crate) fn list_open_comments_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "file_id": {
                "type": "string",
                "description": "One doc's id: ask Drive now instead of the stored list"
            },
            "limit": {
                "type": "integer",
                "description": "Max threads (default 20, max 100)",
                "minimum": 1,
                "maximum": 100
            }
        }
    })
}

pub(crate) fn limit_param(params: &Value) -> usize {
    params
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(20)
        .clamp(1, 100) as usize
}

pub struct DriveListOpenCommentsTool {
    api: Arc<dyn CommentsApi>,
    data_dir: PathBuf,
}

impl DriveListOpenCommentsTool {
    /// `data_dir` holds `projections.db`.
    pub fn new(integration: Arc<GoogleDriveIntegration>, data_dir: PathBuf) -> Self {
        Self::with_api(Arc::new(HubCommentsApi::new(integration)), data_dir)
    }

    pub fn with_api(api: Arc<dyn CommentsApi>, data_dir: PathBuf) -> Self {
        Self { api, data_dir }
    }

    pub(crate) async fn run(&self, params: &Value) -> Result<Value, ToolError> {
        let limit = limit_param(params);
        let Some(file_id) = params.get("file_id").and_then(Value::as_str) else {
            let out = open_comments_from_projection(&self.data_dir, None, limit)?;
            return Ok(json!({ "comments": out, "source": "feed" }));
        };
        let me = self.api.about_me().await.map_err(integ_err)?;
        let doc = self.api.doc_meta(file_id).await.map_err(integ_err)?;
        let mut all = comments::list_comments(self.api.as_ref(), file_id, None)
            .await
            .map_err(integ_err)?;
        all.retain(|c| !c.resolved && !c.deleted);
        all.sort_by(|a, b| b.modified_time.cmp(&a.modified_time));
        let out: Vec<Value> = all
            .iter()
            .take(limit)
            .map(|c| {
                let link = doc
                    .web_view_link
                    .as_deref()
                    .map(|l| comments::comment_link(l, &c.id));
                let reason = comments::for_me(c, &me, doc.owned_by_me).map(|r| r.as_str());
                comment_summary(file_id, &doc.name, link, c, reason)
            })
            .collect();
        Ok(json!({ "comments": out, "source": "drive" }))
    }
}

#[async_trait]
impl Tool for DriveListOpenCommentsTool {
    fn name(&self) -> &str {
        "drive_list_open_comments"
    }
    fn description(&self) -> &str {
        LIST_OPEN_COMMENTS_DESCRIPTION
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Drive
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        list_open_comments_schema()
    }
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let v = self.run(&params).await?;
        Ok(ToolOutput::success(v.to_string()))
    }
}

// ─── /drive_reply_to_comment ──────────────────────────────────────────────

pub struct DriveReplyToCommentTool {
    api: Arc<dyn CommentsApi>,
}

impl DriveReplyToCommentTool {
    pub fn new(integration: Arc<GoogleDriveIntegration>) -> Self {
        Self::with_api(Arc::new(HubCommentsApi::new(integration)))
    }

    pub fn with_api(api: Arc<dyn CommentsApi>) -> Self {
        Self { api }
    }

    pub(crate) async fn run(&self, params: &Value) -> Result<Value, ToolError> {
        let arg = |k: &str| {
            params
                .get(k)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| ToolError::ExecutionFailed(format!("missing or empty '{k}'")))
        };
        let file_id = arg("file_id")?;
        let comment_id = arg("comment_id")?;
        let body = arg("body")?;
        // Content only: no `action`, so the thread is not resolved.
        let request = google_drive3::api::Reply {
            content: Some(body.to_string()),
            ..Default::default()
        };
        let reply = self
            .api
            .create_reply(file_id, comment_id, request, comments::REPLY_FIELDS)
            .await
            .map_err(integ_err)?;
        // The reply is posted; a failed link lookup only loses the link.
        let link = match self.api.doc_meta(file_id).await {
            Ok(doc) => doc
                .web_view_link
                .map(|l| comments::comment_link(&l, comment_id)),
            Err(e) => {
                tracing::warn!(file_id, error = %e, "drive reply: no link");
                None
            }
        };
        Ok(json!({
            "reply_id": reply.id,
            "content": reply.content,
            "created_time": reply.created_time.map(|t| t.to_rfc3339()),
            "link": link,
        }))
    }
}

#[async_trait]
impl Tool for DriveReplyToCommentTool {
    fn name(&self) -> &str {
        "drive_reply_to_comment"
    }
    fn description(&self) -> &str {
        "Reply to a comment thread on a Google Doc, Sheet or Slides file, as me. Takes \
         `file_id` and `comment_id` (from drive_list_open_comments) and the reply \
         `body` (plain text). Does not resolve the thread: resolving stays with the \
         people in it. Gives back the reply id and a link to the thread."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Drive
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::Other
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_id": { "type": "string", "description": "Drive file id" },
                "comment_id": { "type": "string", "description": "Comment thread id" },
                "body": { "type": "string", "description": "Reply text (plain)" }
            },
            "required": ["file_id", "comment_id", "body"]
        })
    }
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let v = self.run(&params).await?;
        Ok(ToolOutput::success(v.to_string()))
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

    // ─── doc comment tools ───────────────────────────────────────────

    use crate::IntegrationError;
    use google_drive3::api::{Comment, CommentList, Reply};
    use std::sync::Mutex;

    /// Fake Drive comment calls. Records each call; `replies.create`
    /// keeps the request body as JSON.
    #[derive(Default)]
    struct FakeApi {
        calls: Mutex<Vec<String>>,
        reply_requests: Mutex<Vec<(String, String, Value, String)>>,
        comments: Vec<Comment>,
    }

    #[async_trait]
    impl CommentsApi for FakeApi {
        async fn about_me(&self) -> Result<String, IntegrationError> {
            self.calls.lock().unwrap().push("about.get".into());
            Ok("me@example.com".into())
        }
        async fn doc_meta(&self, file_id: &str) -> Result<comments::DocMeta, IntegrationError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("files.get {file_id}"));
            Ok(comments::DocMeta {
                id: file_id.into(),
                name: "Q3 plan".into(),
                mime_type: "application/vnd.google-apps.document".into(),
                web_view_link: Some(format!("https://docs.google.com/document/d/{file_id}/edit")),
                owned_by_me: false,
            })
        }
        async fn list_comments_page(
            &self,
            file_id: &str,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<CommentList, IntegrationError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("comments.list {file_id}"));
            Ok(CommentList {
                comments: Some(self.comments.clone()),
                ..Default::default()
            })
        }
        async fn create_reply(
            &self,
            file_id: &str,
            comment_id: &str,
            request: Reply,
            fields: &str,
        ) -> Result<Reply, IntegrationError> {
            self.calls.lock().unwrap().push("replies.create".into());
            self.reply_requests.lock().unwrap().push((
                file_id.into(),
                comment_id.into(),
                serde_json::to_value(&request).unwrap(),
                fields.into(),
            ));
            Ok(serde_json::from_value(json!({
                "id": "R9", "content": request.content,
                "createdTime": "2026-10-07T08:00:00Z"
            }))
            .unwrap())
        }
    }

    #[tokio::test]
    async fn reply_posts_the_body_to_the_thread_and_does_not_resolve() {
        let api = Arc::new(FakeApi::default());
        let tool = DriveReplyToCommentTool::with_api(api.clone());
        let out = tool
            .run(&json!({ "file_id": "F1", "comment_id": "C1", "body": "Confirmed, 4 hires." }))
            .await
            .unwrap();

        let reqs = api.reply_requests.lock().unwrap().clone();
        assert_eq!(reqs.len(), 1);
        let (file_id, comment_id, body, fields) = &reqs[0];
        assert_eq!((file_id.as_str(), comment_id.as_str()), ("F1", "C1"));
        assert_eq!(body["content"], "Confirmed, 4 hires.");
        assert!(body["action"].is_null(), "no resolve action: {body}");
        assert_eq!(fields, "id,content,createdTime");

        assert_eq!(out["reply_id"], "R9");
        assert_eq!(
            out["link"],
            "https://docs.google.com/document/d/F1/edit?disco=C1"
        );
    }

    #[tokio::test]
    async fn reply_refuses_an_empty_body_without_calling_drive() {
        let api = Arc::new(FakeApi::default());
        let tool = DriveReplyToCommentTool::with_api(api.clone());
        for body in [json!(""), json!("   "), Value::Null] {
            let err = tool
                .run(&json!({ "file_id": "F1", "comment_id": "C1", "body": body }))
                .await
                .unwrap_err();
            assert!(err.to_string().contains("body"), "{err}");
        }
        assert!(api.calls.lock().unwrap().is_empty());
    }

    fn seeded_data_dir() -> tempfile::TempDir {
        use arawn_projections::drive_comments::DriveCommentProjection;
        let tmp = tempfile::tempdir().unwrap();
        let store =
            arawn_projections::ProjectionStore::open(&tmp.path().join("projections.db")).unwrap();
        let row = |n: u32, file: &str, hours_ago: i64| DriveCommentProjection {
            id: format!("dc-{n}"),
            feed_id: "drive-comments".into(),
            source_id: format!("{file}/C{n}"),
            source_ts: chrono::Utc::now() - chrono::Duration::hours(hours_ago),
            file_id: file.into(),
            comment_id: format!("C{n}"),
            doc_title: format!("Doc {file}"),
            link: Some(format!(
                "https://docs.google.com/document/d/{file}/edit?disco=C{n}"
            )),
            reason: "mentioned".into(),
            author: Some("Robin".into()),
            content: format!("Thread {n}"),
            quoted: None,
            reply_count: 0,
            last_reply_author: None,
            last_reply: None,
        };
        store
            .write_batch(&[row(1, "F1", 5), row(2, "F2", 1), row(3, "F1", 3)])
            .unwrap();
        tmp
    }

    #[tokio::test]
    async fn list_reads_the_projection_rows_newest_first() {
        let tmp = seeded_data_dir();
        let api = Arc::new(FakeApi::default());
        let tool = DriveListOpenCommentsTool::with_api(api.clone(), tmp.path().to_path_buf());
        let out = tool.run(&json!({ "limit": 2 })).await.unwrap();
        assert_eq!(out["source"], "feed");
        let ids: Vec<&str> = out["comments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["comment_id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["C2", "C3"]);
        assert_eq!(out["comments"][0]["content"], "Thread 2");
        assert_eq!(out["comments"][0]["reason"], "mentioned");
        assert!(api.calls.lock().unwrap().is_empty(), "no Drive call");
    }

    #[tokio::test]
    async fn list_with_file_id_asks_drive_for_the_open_threads() {
        let tmp = tempfile::tempdir().unwrap();
        let comment = |id: &str, extra: Value| {
            let mut v = json!({
                "id": id, "content": format!("Thread {id}"), "resolved": false,
                "modifiedTime": "2026-10-05T10:00:00Z",
                "author": {"displayName": "Robin", "me": false}
            });
            v.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            serde_json::from_value::<Comment>(v).unwrap()
        };
        let api = Arc::new(FakeApi {
            comments: vec![
                comment(
                    "A",
                    json!({ "mentionedEmailAddresses": ["me@example.com"] }),
                ),
                comment("B", json!({ "resolved": true })),
                comment("C", json!({})),
            ],
            ..Default::default()
        });
        let tool = DriveListOpenCommentsTool::with_api(api.clone(), tmp.path().to_path_buf());
        let out = tool.run(&json!({ "file_id": "F1" })).await.unwrap();
        assert_eq!(out["source"], "drive");
        let got: Vec<(Value, Value)> = out["comments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| (c["comment_id"].clone(), c["reason"].clone()))
            .collect();
        assert_eq!(
            got,
            [(json!("A"), json!("mentioned")), (json!("C"), Value::Null)]
        );
        assert_eq!(
            out["comments"][0]["link"],
            "https://docs.google.com/document/d/F1/edit?disco=A"
        );
    }

    #[test]
    fn comment_tools_are_drive_tools_with_the_right_permissions() {
        let tmp = tempfile::tempdir().unwrap();
        let api: Arc<dyn CommentsApi> = Arc::new(FakeApi::default());
        let list = DriveListOpenCommentsTool::with_api(api.clone(), tmp.path().to_path_buf());
        let reply = DriveReplyToCommentTool::with_api(api);
        assert_eq!(list.category(), ToolCategory::Drive);
        assert_eq!(reply.category(), ToolCategory::Drive);
        assert_eq!(list.permission_category(), PermissionCategory::ReadOnly);
        assert_eq!(reply.permission_category(), PermissionCategory::Other);
        assert!(reply.description().contains("Does not resolve"));
    }

    #[test]
    fn summarize_file_includes_parents_when_requested() {
        let mut f = DriveFile::default();
        f.parents = Some(vec!["folder1".into()]);
        let s = summarize_file(&f, true);
        assert_eq!(s.parents.as_deref(), Some(&["folder1".to_string()][..]));
    }
}
