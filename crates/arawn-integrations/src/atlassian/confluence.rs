//! Confluence tools — search, get page, create, update, list spaces.

use std::collections::HashSet;
use std::sync::Arc;

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::client::AtlassianClient;
use super::integration::AtlassianIntegration;

fn integ_err(e: crate::IntegrationError) -> ToolError {
    ToolError::ExecutionFailed(e.user_message())
}

fn check_scopes(
    integration: &AtlassianIntegration,
    required: &[&str],
) -> Result<(), ToolError> {
    let granted: HashSet<String> = integration.granted_scopes().map_err(integ_err)?;
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|s| !granted.contains(*s))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(ToolError::ExecutionFailed(format!(
            "Missing Atlassian scope(s): {}. Add to your Atlassian app, \
             then run /connect atlassian to refresh.",
            missing.join(", ")
        )))
    }
}

fn site_param(params: &Value) -> Option<&str> {
    params.get("site").and_then(|v| v.as_str())
}

// ─── Markdown ↔ Confluence storage format ─────────────────────────────────
//
// Confluence stores pages in its own XML-flavored "storage format" (HTML
// with custom XML namespaces). For v1 we handle the 80% case:
// paragraphs, headers, lists, code blocks, and inline emphasis. Anything
// outside that round-trips as a paragraph block.

/// Wrap a markdown body into a Confluence storage-format string. Naive
/// converter: line-based, handles `# heading`, `- item`, ` ```lang` code
/// blocks, and paragraphs. Inline `**bold**` / `*italic*` / `` `code` ``
/// are escaped into `<strong>` / `<em>` / `<code>`.
fn markdown_to_storage(md: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    let mut code_buf = String::new();
    let mut in_list = false;

    let flush_list = |out: &mut String, in_list: &mut bool| {
        if *in_list {
            out.push_str("</ul>");
            *in_list = false;
        }
    };

    for line in md.lines() {
        if let Some(rest) = line.strip_prefix("```") {
            if in_code {
                // Closing fence.
                out.push_str(&format!(
                    "<ac:structured-macro ac:name=\"code\"><ac:plain-text-body><![CDATA[{}]]></ac:plain-text-body></ac:structured-macro>",
                    code_buf.trim_end_matches('\n')
                ));
                code_buf.clear();
                in_code = false;
            } else {
                flush_list(&mut out, &mut in_list);
                in_code = true;
                let _ = rest; // language hint — not used by Confluence's basic code macro
            }
            continue;
        }
        if in_code {
            code_buf.push_str(line);
            code_buf.push('\n');
            continue;
        }

        if let Some(stripped) = line.strip_prefix("# ") {
            flush_list(&mut out, &mut in_list);
            out.push_str(&format!("<h1>{}</h1>", inline_md_to_storage(stripped)));
        } else if let Some(stripped) = line.strip_prefix("## ") {
            flush_list(&mut out, &mut in_list);
            out.push_str(&format!("<h2>{}</h2>", inline_md_to_storage(stripped)));
        } else if let Some(stripped) = line.strip_prefix("### ") {
            flush_list(&mut out, &mut in_list);
            out.push_str(&format!("<h3>{}</h3>", inline_md_to_storage(stripped)));
        } else if let Some(stripped) = line.strip_prefix("- ") {
            if !in_list {
                out.push_str("<ul>");
                in_list = true;
            }
            out.push_str(&format!("<li>{}</li>", inline_md_to_storage(stripped)));
        } else if line.trim().is_empty() {
            flush_list(&mut out, &mut in_list);
        } else {
            flush_list(&mut out, &mut in_list);
            out.push_str(&format!("<p>{}</p>", inline_md_to_storage(line)));
        }
    }
    flush_list(&mut out, &mut in_list);
    if in_code {
        // Unclosed code fence — render what we have as a code block.
        out.push_str(&format!(
            "<ac:structured-macro ac:name=\"code\"><ac:plain-text-body><![CDATA[{}]]></ac:plain-text-body></ac:structured-macro>",
            code_buf.trim_end_matches('\n')
        ));
    }
    out
}

/// Apply inline markdown (bold/italic/code) to a text fragment, escaping
/// XML-unsafe characters first.
fn inline_md_to_storage(s: &str) -> String {
    let escaped = xml_escape(s);
    apply_inline(&escaped)
}

fn apply_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                let (inner, found) = take_until(&mut chars, "**");
                if found {
                    out.push_str("<strong>");
                    out.push_str(&inner);
                    out.push_str("</strong>");
                } else {
                    out.push_str("**");
                    out.push_str(&inner);
                }
            }
            '*' => {
                let (inner, found) = take_until(&mut chars, "*");
                if found {
                    out.push_str("<em>");
                    out.push_str(&inner);
                    out.push_str("</em>");
                } else {
                    out.push('*');
                    out.push_str(&inner);
                }
            }
            '`' => {
                let (inner, found) = take_until(&mut chars, "`");
                if found {
                    out.push_str("<code>");
                    out.push_str(&inner);
                    out.push_str("</code>");
                } else {
                    out.push('`');
                    out.push_str(&inner);
                }
            }
            other => out.push(other),
        }
    }
    out
}

fn take_until(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    delim: &str,
) -> (String, bool) {
    let mut buf = String::new();
    let bytes: Vec<char> = delim.chars().collect();
    let n = bytes.len();
    let mut window: Vec<char> = Vec::with_capacity(n);
    for c in chars.by_ref() {
        buf.push(c);
        window.push(c);
        if window.len() > n {
            window.remove(0);
        }
        if window.len() == n && window == bytes {
            // Strip the delimiter from buf
            buf.truncate(buf.len() - n);
            return (buf, true);
        }
    }
    (buf, false)
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// Strip Confluence storage-format tags into rough markdown. Lossy — for
/// agent-side reading, not round-trip-perfect editing.
fn storage_to_markdown(storage: &str) -> String {
    let mut out = String::with_capacity(storage.len());
    let mut chars = storage.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            // Skip to matching '>'
            let mut tag = String::new();
            for c2 in chars.by_ref() {
                if c2 == '>' {
                    break;
                }
                tag.push(c2);
            }
            // Translate a few known tags into markdown.
            let lower = tag.to_ascii_lowercase();
            if lower == "p" || lower == "/p" || lower == "br" || lower == "br/" {
                out.push('\n');
            } else if lower == "h1" {
                out.push_str("\n# ");
            } else if lower == "h2" {
                out.push_str("\n## ");
            } else if lower == "h3" {
                out.push_str("\n### ");
            } else if lower == "ul" || lower == "/ul" {
                out.push('\n');
            } else if lower == "li" {
                out.push_str("- ");
            } else if lower == "/li" {
                out.push('\n');
            } else if lower == "strong" || lower == "/strong" {
                out.push_str("**");
            } else if lower == "em" || lower == "/em" {
                out.push('*');
            } else if lower == "code" || lower == "/code" {
                out.push('`');
            }
            // All other tags drop silently.
            continue;
        }
        out.push(c);
    }
    // Unescape entities.
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

// ─── Response shapes ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SearchResp {
    #[serde(default)]
    results: Vec<RawSearchResult>,
}

#[derive(Debug, Deserialize)]
struct RawSearchResult {
    title: Option<String>,
    #[serde(rename = "_links", default)]
    links: serde_json::Map<String, Value>,
    #[serde(default)]
    content: Option<RawContentRef>,
}

#[derive(Debug, Deserialize)]
struct RawContentRef {
    id: String,
    #[serde(rename = "type")]
    kind: Option<String>,
    space: Option<RawSpaceRef>,
}

#[derive(Debug, Deserialize)]
struct RawSpaceRef {
    key: Option<String>,
}

#[derive(Debug, Serialize)]
struct SearchHit {
    id: Option<String>,
    title: Option<String>,
    kind: Option<String>,
    space_key: Option<String>,
    url: Option<String>,
}

// Confluence v2 page shape: `{ id, status, title, spaceId, parentId,
// version: {number, message?}, body: {storage: {value, representation}},
// _links: {webui, base, edit-ui, ...} }`. Notably no `space` block (only
// `spaceId`); we resolve the space key separately if the agent needs it.
#[derive(Debug, Deserialize)]
struct PageDetailRaw {
    id: String,
    title: Option<String>,
    #[serde(rename = "spaceId", default)]
    space_id: Option<String>,
    body: Option<RawBody>,
    version: Option<RawVersion>,
    #[serde(rename = "_links", default)]
    links: serde_json::Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct RawBody {
    storage: Option<RawBodyContent>,
}

#[derive(Debug, Deserialize)]
struct RawBodyContent {
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawVersion {
    number: Option<u64>,
}

#[derive(Debug, Serialize)]
struct PageSummary {
    id: String,
    title: Option<String>,
    kind: Option<String>,
    space_key: Option<String>,
    space_name: Option<String>,
    body_markdown: Option<String>,
    body_storage: Option<String>,
    version: Option<u64>,
    url: Option<String>,
}

// Confluence v2 spaces shape: `{ results: [{id, key, name, type, ...}],
// _links: { next? } }`. id is now numeric (string-encoded), key is the
// short alpha key the agent already uses.
#[derive(Debug, Deserialize)]
struct SpacesResp {
    #[serde(default)]
    results: Vec<RawSpace>,
}

#[derive(Debug, Deserialize)]
struct RawSpace {
    id: String,
    key: String,
    name: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
}

// ─── /confluence_search ───────────────────────────────────────────────────

const CQL_SEARCH_BASE: &str = "Search Confluence with CQL (Confluence Query Language). \
    Examples: `text ~ 'arawn'`, `space = ENG AND lastmodified >= now('-7d')`, \
    `title = 'Onboarding'`. Returns hits with id, title, kind, space_key, url. \
    Use confluence_get_page to fetch full content for a specific id.";
const CQL_SEARCH_SCOPES: &[&str] = &["read:confluence-content.all"];

pub struct ConfluenceSearchTool {
    integration: Arc<AtlassianIntegration>,
    description: String,
}

impl ConfluenceSearchTool {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self {
            integration,
            description: format!(
                "{CQL_SEARCH_BASE}\n\nRequires Atlassian scope(s): {}.",
                CQL_SEARCH_SCOPES.join(", ")
            ),
        }
    }
}

#[async_trait]
impl Tool for ConfluenceSearchTool {
    fn name(&self) -> &str {
        "confluence_search"
    }
    fn description(&self) -> &str {
        &self.description
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
                "cql": { "type": "string", "description": "CQL query" },
                "limit": { "type": "integer", "description": "Max results (default 25, max 100)", "minimum": 1, "maximum": 100 },
                "site": { "type": "string", "description": "Atlassian site (optional)" }
            },
            "required": ["cql"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        check_scopes(&self.integration, CQL_SEARCH_SCOPES)?;
        let cql = params
            .get("cql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'cql'".into()))?
            .to_string();
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(25)
            .min(100);
        let site = site_param(&params);

        let client = AtlassianClient::new(Arc::clone(&self.integration));
        // CQL search has no v2 equivalent yet; v1 /search remains
        // functional per Atlassian's deprecation table.
        let resp: SearchResp = client
            .confluence_v1_get(
                "/search",
                site,
                &[("cql", cql), ("limit", limit.to_string())],
            )
            .await
            .map_err(integ_err)?;

        let hits: Vec<SearchHit> = resp
            .results
            .iter()
            .map(|r| SearchHit {
                id: r.content.as_ref().map(|c| c.id.clone()),
                title: r.title.clone(),
                kind: r.content.as_ref().and_then(|c| c.kind.clone()),
                space_key: r
                    .content
                    .as_ref()
                    .and_then(|c| c.space.as_ref())
                    .and_then(|s| s.key.clone()),
                url: r
                    .links
                    .get("webui")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            })
            .collect();

        Ok(ToolOutput::success(json!({"results": hits}).to_string()))
    }
}

// ─── /confluence_get_page ─────────────────────────────────────────────────

const CONFLUENCE_GET_PAGE_BASE: &str = "Fetch a Confluence page by id. Returns title, \
    space, body in markdown (lossy conversion from Confluence storage \
    format), version number, and the web URL. Set raw=true to get the \
    raw storage-format XML body alongside the markdown.";
const CONFLUENCE_GET_PAGE_SCOPES: &[&str] = &["read:confluence-content.all"];

pub struct ConfluenceGetPageTool {
    integration: Arc<AtlassianIntegration>,
    description: String,
}

impl ConfluenceGetPageTool {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self {
            integration,
            description: format!(
                "{CONFLUENCE_GET_PAGE_BASE}\n\nRequires Atlassian scope(s): {}.",
                CONFLUENCE_GET_PAGE_SCOPES.join(", ")
            ),
        }
    }
}

#[async_trait]
impl Tool for ConfluenceGetPageTool {
    fn name(&self) -> &str {
        "confluence_get_page"
    }
    fn description(&self) -> &str {
        &self.description
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
                "page_id": { "type": "string" },
                "raw": { "type": "boolean", "description": "Also return raw storage-format body" },
                "site": { "type": "string", "description": "Atlassian site (optional)" }
            },
            "required": ["page_id"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        check_scopes(&self.integration, CONFLUENCE_GET_PAGE_SCOPES)?;
        let page_id = params
            .get("page_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'page_id'".into()))?
            .to_string();
        let want_raw = params.get("raw").and_then(|v| v.as_bool()).unwrap_or(false);
        let site = site_param(&params);

        let client = AtlassianClient::new(Arc::clone(&self.integration));
        // v2: GET /pages/{id}?body-format=storage. Body shape is
        // {value, representation}; no nested space object — we resolve
        // the space key separately if needed.
        let page: PageDetailRaw = client
            .confluence_get(
                &format!("/pages/{page_id}"),
                site,
                &[("body-format", "storage".into())],
            )
            .await
            .map_err(integ_err)?;

        let storage = page
            .body
            .as_ref()
            .and_then(|b| b.storage.as_ref())
            .and_then(|s| s.value.clone());

        // Resolve spaceId → key (best-effort; one extra round-trip).
        let space_key = if let Some(ref sid) = page.space_id {
            client
                .confluence_get::<SpacesResp>("/spaces", site, &[])
                .await
                .ok()
                .and_then(|r| r.results.into_iter().find(|s| &s.id == sid)).map(|s| s.key)
        } else {
            None
        };

        let summary = PageSummary {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: Some("page".to_string()),
            space_key,
            space_name: None,
            body_markdown: storage.as_deref().map(storage_to_markdown),
            body_storage: if want_raw { storage } else { None },
            version: page.version.and_then(|v| v.number),
            url: page
                .links
                .get("webui")
                .and_then(|v| v.as_str())
                .map(String::from),
        };

        Ok(ToolOutput::success(serde_json::to_string(&summary).unwrap()))
    }
}

// ─── /confluence_create_page ──────────────────────────────────────────────

const CONFLUENCE_CREATE_PAGE_BASE: &str = "Create a new Confluence page. `body_markdown` \
    is auto-converted to Confluence storage format (handles paragraphs, \
    headers, lists, code blocks, **bold**, *italic*, `code`). For pages with \
    a parent (sub-page), pass parent_id.";
const CONFLUENCE_CREATE_PAGE_SCOPES: &[&str] = &["write:confluence-content"];

pub struct ConfluenceCreatePageTool {
    integration: Arc<AtlassianIntegration>,
    description: String,
}

impl ConfluenceCreatePageTool {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self {
            integration,
            description: format!(
                "{CONFLUENCE_CREATE_PAGE_BASE}\n\nRequires Atlassian scope(s): {}.",
                CONFLUENCE_CREATE_PAGE_SCOPES.join(", ")
            ),
        }
    }
}

#[async_trait]
impl Tool for ConfluenceCreatePageTool {
    fn name(&self) -> &str {
        "confluence_create_page"
    }
    fn description(&self) -> &str {
        &self.description
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
                "space_key": { "type": "string", "description": "Space key (e.g. 'ENG')" },
                "title": { "type": "string" },
                "body_markdown": { "type": "string" },
                "parent_id": { "type": "string", "description": "Parent page id (optional)" },
                "site": { "type": "string", "description": "Atlassian site (optional)" }
            },
            "required": ["space_key", "title", "body_markdown"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        check_scopes(&self.integration, CONFLUENCE_CREATE_PAGE_SCOPES)?;
        let space_key = params
            .get("space_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'space_key'".into()))?;
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'title'".into()))?;
        let body_md = params
            .get("body_markdown")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'body_markdown'".into()))?;
        let parent_id = params.get("parent_id").and_then(|v| v.as_str());
        let site = site_param(&params);

        let storage = markdown_to_storage(body_md);

        let client = AtlassianClient::new(Arc::clone(&self.integration));

        // v2 wants spaceId, not space.key. One extra GET to resolve.
        let spaces: SpacesResp = client
            .confluence_get("/spaces", site, &[])
            .await
            .map_err(integ_err)?;
        let space_id = spaces
            .results
            .into_iter()
            .find(|s| s.key == space_key)
            .map(|s| s.id)
            .ok_or_else(|| {
                ToolError::ExecutionFailed(format!(
                    "no Confluence space with key '{space_key}' on this site"
                ))
            })?;

        let mut body = json!({
            "spaceId": space_id,
            "status": "current",
            "title": title,
            "body": {
                "representation": "storage",
                "value": storage,
            },
        });
        if let Some(pid) = parent_id {
            body["parentId"] = json!(pid);
        }

        let resp: Value = client
            .confluence_post("/pages", site, &body)
            .await
            .map_err(integ_err)?;
        Ok(ToolOutput::success(resp.to_string()))
    }
}

// ─── /confluence_update_page ──────────────────────────────────────────────

const CONFLUENCE_UPDATE_PAGE_BASE: &str = "Update an existing Confluence page. \
    Confluence requires the new version number to be one higher than the \
    current — this tool fetches the current version automatically and \
    increments. body_markdown is converted to storage format the same way \
    as confluence_create_page.";
const CONFLUENCE_UPDATE_PAGE_SCOPES: &[&str] = &["write:confluence-content"];

pub struct ConfluenceUpdatePageTool {
    integration: Arc<AtlassianIntegration>,
    description: String,
}

impl ConfluenceUpdatePageTool {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self {
            integration,
            description: format!(
                "{CONFLUENCE_UPDATE_PAGE_BASE}\n\nRequires Atlassian scope(s): {}.",
                CONFLUENCE_UPDATE_PAGE_SCOPES.join(", ")
            ),
        }
    }
}

#[async_trait]
impl Tool for ConfluenceUpdatePageTool {
    fn name(&self) -> &str {
        "confluence_update_page"
    }
    fn description(&self) -> &str {
        &self.description
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
                "page_id": { "type": "string" },
                "title": { "type": "string", "description": "New title (required by Confluence; pass current to keep unchanged)" },
                "body_markdown": { "type": "string" },
                "site": { "type": "string", "description": "Atlassian site (optional)" }
            },
            "required": ["page_id", "title", "body_markdown"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        check_scopes(&self.integration, CONFLUENCE_UPDATE_PAGE_SCOPES)?;
        let page_id = params
            .get("page_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'page_id'".into()))?
            .to_string();
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'title'".into()))?;
        let body_md = params
            .get("body_markdown")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'body_markdown'".into()))?;
        let site = site_param(&params);

        let client = AtlassianClient::new(Arc::clone(&self.integration));
        // v2: GET /pages/{id} for the current version number.
        let current: PageDetailRaw = client
            .confluence_get(&format!("/pages/{page_id}"), site, &[])
            .await
            .map_err(integ_err)?;
        let current_version = current.version.and_then(|v| v.number).unwrap_or(1);

        let storage = markdown_to_storage(body_md);
        let body = json!({
            "id": page_id,
            "status": "current",
            "title": title,
            "version": { "number": current_version + 1 },
            "body": {
                "representation": "storage",
                "value": storage,
            },
        });

        let resp: Value = client
            .confluence_put(&format!("/pages/{page_id}"), site, &body)
            .await
            .map_err(integ_err)?;
        Ok(ToolOutput::success(resp.to_string()))
    }
}

// ─── /confluence_list_spaces ──────────────────────────────────────────────

const CONFLUENCE_LIST_SPACES_BASE: &str = "List Confluence spaces the user can access. \
    Returns key, name, type. Use the space_key in confluence_create_page or \
    in CQL queries (`space = ENG`).";
const CONFLUENCE_LIST_SPACES_SCOPES: &[&str] = &["read:confluence-space.summary"];

pub struct ConfluenceListSpacesTool {
    integration: Arc<AtlassianIntegration>,
    description: String,
}

impl ConfluenceListSpacesTool {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self {
            integration,
            description: format!(
                "{CONFLUENCE_LIST_SPACES_BASE}\n\nRequires Atlassian scope(s): {}.",
                CONFLUENCE_LIST_SPACES_SCOPES.join(", ")
            ),
        }
    }
}

#[derive(Debug, Serialize)]
struct SpaceSummary {
    id: String,
    key: String,
    name: Option<String>,
    kind: Option<String>,
}

#[async_trait]
impl Tool for ConfluenceListSpacesTool {
    fn name(&self) -> &str {
        "confluence_list_spaces"
    }
    fn description(&self) -> &str {
        &self.description
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
                "site": { "type": "string", "description": "Atlassian site (optional)" }
            }
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        check_scopes(&self.integration, CONFLUENCE_LIST_SPACES_SCOPES)?;
        let site = site_param(&params);
        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let resp: SpacesResp = client
            .confluence_get("/spaces", site, &[])
            .await
            .map_err(integ_err)?;
        let summaries: Vec<SpaceSummary> = resp
            .results
            .into_iter()
            .map(|s| SpaceSummary {
                id: s.id,
                key: s.key,
                name: s.name,
                kind: s.kind,
            })
            .collect();
        Ok(ToolOutput::success(
            json!({"spaces": summaries}).to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_paragraphs_become_p_tags() {
        let md = "Hello world.\n\nSecond paragraph.";
        let s = markdown_to_storage(md);
        assert!(s.contains("<p>Hello world.</p>"));
        assert!(s.contains("<p>Second paragraph.</p>"));
    }

    #[test]
    fn markdown_headers_become_hN_tags() {
        let s = markdown_to_storage("# Big\n## Mid\n### Small");
        assert!(s.contains("<h1>Big</h1>"));
        assert!(s.contains("<h2>Mid</h2>"));
        assert!(s.contains("<h3>Small</h3>"));
    }

    #[test]
    fn markdown_lists_round_through_ul() {
        let s = markdown_to_storage("- alpha\n- bravo");
        assert!(s.contains("<ul><li>alpha</li><li>bravo</li></ul>"));
    }

    #[test]
    fn markdown_inline_emphasis() {
        let s = markdown_to_storage("Make it **bold** and *italic* with `code`.");
        assert!(s.contains("<strong>bold</strong>"));
        assert!(s.contains("<em>italic</em>"));
        assert!(s.contains("<code>code</code>"));
    }

    #[test]
    fn markdown_code_block_uses_confluence_macro() {
        let s = markdown_to_storage("```rust\nfn main() {}\n```");
        assert!(s.contains("ac:name=\"code\""));
        assert!(s.contains("fn main()"));
    }

    #[test]
    fn xml_escape_handles_lt_gt_amp() {
        let s = markdown_to_storage("a < b & c > d");
        assert!(s.contains("&lt;"));
        assert!(s.contains("&gt;"));
        assert!(s.contains("&amp;"));
    }

    #[test]
    fn storage_to_markdown_strips_basic_tags() {
        let storage = "<h1>Title</h1><p>Hello <strong>world</strong>.</p>";
        let md = storage_to_markdown(storage);
        assert!(md.contains("# Title"));
        assert!(md.contains("**world**"));
    }
}
