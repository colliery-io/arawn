//! Jira tools — search, get, create, update, comment, transition.
//!
//! Uses the [`jira_v3_openapi`] generated client (auto-generated from
//! Atlassian's official OpenAPI spec). The hand-rolled v3 calls in
//! `client.rs` were tracking deprecated endpoints; the generated client
//! follows API moves with each `cargo update`.

use std::collections::HashMap;
use std::sync::Arc;

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use jira_v3_openapi::apis::issue_comments_api;
use jira_v3_openapi::apis::issue_search_api;
use jira_v3_openapi::apis::issues_api;
use jira_v3_openapi::models::{
    Comment, IssueBean, IssueUpdateDetails, SearchAndReconcileRequestBean,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::adf::md_to_adf;
use super::client::AtlassianClient;
use super::integration::AtlassianIntegration;

fn integ_err(e: crate::IntegrationError) -> ToolError {
    ToolError::ExecutionFailed(e.user_message())
}

fn check_scopes(
    integration: &AtlassianIntegration,
    required: &[&str],
) -> Result<(), ToolError> {
    let granted: std::collections::HashSet<String> = integration
        .granted_scopes()
        .map_err(integ_err)?;
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|s| !granted.contains(*s))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(ToolError::ExecutionFailed(format!(
            "Missing Atlassian scope(s): {}. Add to your Atlassian app \
             (developer.atlassian.com/console/myapps), then run /connect \
             atlassian to refresh the token.",
            missing.join(", ")
        )))
    }
}

fn site_param(params: &Value) -> Option<&str> {
    params.get("site").and_then(|v| v.as_str())
}

/// Map an `openapi::Error<E>` (from the generated client) into our common
/// IntegrationError → ToolError pipeline. `Error::ResponseError` carries
/// the HTTP status + body so we can surface useful context to the agent.
fn openapi_err<E: std::fmt::Debug>(e: jira_v3_openapi::apis::Error<E>) -> ToolError {
    use jira_v3_openapi::apis::Error;
    let msg = match e {
        Error::ResponseError(r) => format!("HTTP {}: {}", r.status, r.content),
        other => format!("{other}"),
    };
    integ_err(crate::IntegrationError::Provider(msg))
}

/// Some Jira write endpoints (transitions, edit-without-return) respond
/// with HTTP 204 No Content. The generated client tries to JSON-parse
/// the empty body and fails with `EOF while parsing a value at line 1
/// column 0`. Treat that specific error as success.
fn tolerate_empty_body<E: std::fmt::Debug>(
    e: jira_v3_openapi::apis::Error<E>,
) -> Result<(), ToolError> {
    use jira_v3_openapi::apis::Error;
    if let Error::Serde(ref se) = e
        && se.classify() == serde_json::error::Category::Eof
    {
        return Ok(());
    }
    Err(openapi_err(e))
}

// IssueBean.fields is a HashMap; the existing extractors work with
// serde_json::Map. Adapter so we don't have to rewrite the field-pluck
// helpers.
fn fields_map(issue: &IssueBean) -> Map<String, Value> {
    issue
        .fields
        .clone()
        .map(|h| h.into_iter().collect::<Map<String, Value>>())
        .unwrap_or_default()
}

// ─── Response shapes ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct IssueSummary {
    key: String,
    summary: Option<String>,
    status: Option<String>,
    issue_type: Option<String>,
    priority: Option<String>,
    assignee: Option<String>,
    reporter: Option<String>,
    updated: Option<String>,
}

fn summarize_issue(key: &str, fields: &Map<String, Value>) -> IssueSummary {
    IssueSummary {
        key: key.to_string(),
        summary: fields.get("summary").and_then(|v| v.as_str()).map(String::from),
        status: fields
            .get("status")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from),
        issue_type: fields
            .get("issuetype")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from),
        priority: fields
            .get("priority")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from),
        assignee: fields
            .get("assignee")
            .and_then(|v| v.get("displayName"))
            .and_then(|v| v.as_str())
            .map(String::from),
        reporter: fields
            .get("reporter")
            .and_then(|v| v.get("displayName"))
            .and_then(|v| v.as_str())
            .map(String::from),
        updated: fields.get("updated").and_then(|v| v.as_str()).map(String::from),
    }
}

#[derive(Debug, Serialize)]
struct IssueDetail {
    key: String,
    summary: Option<String>,
    status: Option<String>,
    issue_type: Option<String>,
    priority: Option<String>,
    assignee: Option<String>,
    reporter: Option<String>,
    description: Option<String>,
    updated: Option<String>,
    created: Option<String>,
    comments: Vec<CommentSummary>,
    available_transitions: Vec<TransitionSummary>,
}

#[derive(Debug, Serialize)]
struct CommentSummary {
    id: String,
    author: Option<String>,
    body: Option<String>,
    created: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TransitionSummary {
    id: String,
    name: String,
    /// The status the issue moves to if this transition is taken.
    to: Option<String>,
}

// Wrap text — interpreted as markdown — into ADF. Plain text passes
// through cleanly; if the agent provides markdown structure (headings,
// lists, **bold**, `code`, links) it's preserved.
fn adf_from_markdown(text: &str) -> Value {
    md_to_adf(text)
}

// ─── /jira_search ─────────────────────────────────────────────────────────

const JIRA_SEARCH_BASE: &str = "Search Jira issues using JQL (Jira Query Language). \
    Examples: `project = ENG AND status = 'In Progress'`, \
    `assignee = currentUser() AND updated >= -7d`, \
    `text ~ 'login bug'`. Returns issues with key, summary, status, \
    type, priority, assignee, updated. The `site` arg picks among connected \
    Atlassian sites (e.g. 'acme.atlassian.net'); omit for default site. \
    \n\nIMPORTANT: Atlassian rejects unbounded JQL like bare `order by created desc`. \
    Always include a search restriction such as `project = ...`, \
    `assignee = currentUser()`, `updated >= -30d`, or `text ~ '...'`.";
const JIRA_SEARCH_SCOPES: &[&str] = &["read:jira-work"];

pub struct JiraSearchTool {
    integration: Arc<AtlassianIntegration>,
    description: String,
}

impl JiraSearchTool {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self {
            integration,
            description: format!(
                "{JIRA_SEARCH_BASE}\n\nRequires Atlassian scope(s): {}.",
                JIRA_SEARCH_SCOPES.join(", ")
            ),
        }
    }
}

#[async_trait]
impl Tool for JiraSearchTool {
    fn name(&self) -> &str {
        "jira_search"
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
                "jql": { "type": "string", "description": "JQL query" },
                "max_results": {
                    "type": "integer",
                    "description": "Max issues (default 25, max 100)",
                    "minimum": 1,
                    "maximum": 100
                },
                "site": { "type": "string", "description": "Atlassian site URL or name (optional)" }
            },
            "required": ["jql"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        check_scopes(&self.integration, JIRA_SEARCH_SCOPES)?;
        let jql = params
            .get("jql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'jql'".into()))?
            .to_string();
        let max = params
            .get("max_results")
            .and_then(|v| v.as_i64())
            .unwrap_or(25)
            .clamp(1, 100) as i32;
        let site = site_param(&params);

        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let cfg = client.jira_config(site).await.map_err(integ_err)?;

        let req = SearchAndReconcileRequestBean {
            jql: Some(jql),
            max_results: Some(max),
            fields: Some(vec![
                "summary".into(),
                "status".into(),
                "issuetype".into(),
                "priority".into(),
                "assignee".into(),
                "reporter".into(),
                "updated".into(),
            ]),
            ..Default::default()
        };
        let results = issue_search_api::search_and_reconsile_issues_using_jql_post(&cfg, req)
            .await
            .map_err(openapi_err)?;
        let issues = results.issues.unwrap_or_default();
        let summaries: Vec<IssueSummary> = issues
            .iter()
            .map(|b| {
                let key = b.key.clone().unwrap_or_default();
                summarize_issue(&key, &fields_map(b))
            })
            .collect();
        Ok(ToolOutput::success(
            json!({
                "issues": summaries,
                "next_page_token": results.next_page_token,
                "is_last": results.is_last,
            })
            .to_string(),
        ))
    }
}

// ─── /jira_get_issue ──────────────────────────────────────────────────────

const JIRA_GET_ISSUE_BASE: &str = "Get full details for a Jira issue, including \
    description, comments, and the list of available transitions \
    (e.g. 'In Progress', 'Done') the agent can apply via jira_transition_issue.";
const JIRA_GET_ISSUE_SCOPES: &[&str] = &["read:jira-work"];

pub struct JiraGetIssueTool {
    integration: Arc<AtlassianIntegration>,
    description: String,
}

impl JiraGetIssueTool {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self {
            integration,
            description: format!(
                "{JIRA_GET_ISSUE_BASE}\n\nRequires Atlassian scope(s): {}.",
                JIRA_GET_ISSUE_SCOPES.join(", ")
            ),
        }
    }
}

#[async_trait]
impl Tool for JiraGetIssueTool {
    fn name(&self) -> &str {
        "jira_get_issue"
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
                "key": { "type": "string", "description": "Issue key, e.g. 'ENG-123'" },
                "site": { "type": "string", "description": "Atlassian site (optional)" }
            },
            "required": ["key"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        check_scopes(&self.integration, JIRA_GET_ISSUE_SCOPES)?;
        let key = params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'key'".into()))?
            .to_string();
        let site = site_param(&params);

        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let cfg = client.jira_config(site).await.map_err(integ_err)?;

        let issue = issues_api::get_issue(
            &cfg,
            &key,
            Some(vec![
                "summary".into(),
                "status".into(),
                "issuetype".into(),
                "priority".into(),
                "assignee".into(),
                "reporter".into(),
                "description".into(),
                "created".into(),
                "updated".into(),
                "comment".into(),
            ]),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(openapi_err)?;

        let transitions = issues_api::get_transitions(&cfg, &key, None, None, None, None, None)
            .await
            .map_err(openapi_err)?;

        let f = fields_map(&issue);
        let comments = f
            .get("comment")
            .and_then(|v| v.get("comments"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|c| CommentSummary {
                        id: c.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                        author: c
                            .get("author")
                            .and_then(|v| v.get("displayName"))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        body: c.get("body").map(|v| {
                            if let Some(s) = v.as_str() {
                                s.to_string()
                            } else {
                                serde_json::to_string(v).unwrap_or_default()
                            }
                        }),
                        created: c.get("created").and_then(|v| v.as_str()).map(String::from),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let detail = IssueDetail {
            key: issue.key.clone().unwrap_or_else(|| key.clone()),
            summary: f.get("summary").and_then(|v| v.as_str()).map(String::from),
            status: f
                .get("status")
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .map(String::from),
            issue_type: f
                .get("issuetype")
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .map(String::from),
            priority: f
                .get("priority")
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .map(String::from),
            assignee: f
                .get("assignee")
                .and_then(|v| v.get("displayName"))
                .and_then(|v| v.as_str())
                .map(String::from),
            reporter: f
                .get("reporter")
                .and_then(|v| v.get("displayName"))
                .and_then(|v| v.as_str())
                .map(String::from),
            description: f.get("description").map(|v| {
                if let Some(s) = v.as_str() {
                    s.to_string()
                } else {
                    serde_json::to_string(v).unwrap_or_default()
                }
            }),
            created: f.get("created").and_then(|v| v.as_str()).map(String::from),
            updated: f.get("updated").and_then(|v| v.as_str()).map(String::from),
            comments,
            available_transitions: transitions
                .transitions
                .unwrap_or_default()
                .into_iter()
                .map(|t| TransitionSummary {
                    id: t.id.unwrap_or_default(),
                    name: t.name.unwrap_or_default(),
                    to: t.to.and_then(|to| to.name),
                })
                .collect(),
        };

        Ok(ToolOutput::success(serde_json::to_string(&detail).unwrap()))
    }
}

// ─── /jira_create_issue ───────────────────────────────────────────────────

const JIRA_CREATE_ISSUE_BASE: &str = "Create a new Jira issue in the given project. \
    Returns the new issue's key. `description` accepts markdown (headings, \
    lists, **bold**, *italic*, `code`, links) — converted to Atlassian \
    Document Format automatically. `issue_type` is the type name (e.g. \
    'Task', 'Bug', 'Story').";
const JIRA_CREATE_ISSUE_SCOPES: &[&str] = &["write:jira-work"];

pub struct JiraCreateIssueTool {
    integration: Arc<AtlassianIntegration>,
    description: String,
}

impl JiraCreateIssueTool {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self {
            integration,
            description: format!(
                "{JIRA_CREATE_ISSUE_BASE}\n\nRequires Atlassian scope(s): {}.",
                JIRA_CREATE_ISSUE_SCOPES.join(", ")
            ),
        }
    }
}

#[async_trait]
impl Tool for JiraCreateIssueTool {
    fn name(&self) -> &str {
        "jira_create_issue"
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
                "project_key": { "type": "string", "description": "Project key, e.g. 'ENG'" },
                "summary": { "type": "string" },
                "description": { "type": "string" },
                "issue_type": { "type": "string", "description": "e.g. 'Task', 'Bug', 'Story'" },
                "site": { "type": "string", "description": "Atlassian site (optional)" }
            },
            "required": ["project_key", "summary", "issue_type"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        check_scopes(&self.integration, JIRA_CREATE_ISSUE_SCOPES)?;
        let project_key = params
            .get("project_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'project_key'".into()))?;
        let summary = params
            .get("summary")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'summary'".into()))?;
        let issue_type = params
            .get("issue_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'issue_type'".into()))?;
        let description = params.get("description").and_then(|v| v.as_str());
        let site = site_param(&params);

        let mut fields: HashMap<String, Value> = HashMap::new();
        fields.insert("project".into(), json!({ "key": project_key }));
        fields.insert("summary".into(), json!(summary));
        fields.insert("issuetype".into(), json!({ "name": issue_type }));
        if let Some(desc) = description {
            fields.insert("description".into(), adf_from_markdown(desc));
        }

        let details = IssueUpdateDetails {
            fields: Some(fields),
            ..Default::default()
        };

        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let cfg = client.jira_config(site).await.map_err(integ_err)?;
        let created = issues_api::create_issue(&cfg, details, None)
            .await
            .map_err(openapi_err)?;
        Ok(ToolOutput::success(
            json!({
                "id": created.id,
                "key": created.key,
                "self": created.param_self,
            })
            .to_string(),
        ))
    }
}

// ─── /jira_update_issue ───────────────────────────────────────────────────

const JIRA_UPDATE_ISSUE_BASE: &str = "Update fields on an existing Jira issue. Pass \
    `fields` as a JSON object using Jira field names. Examples: \
    `{\"summary\": \"new title\"}`, `{\"priority\": {\"name\": \"High\"}}`, \
    `{\"labels\": [\"uat\", \"arawn\"]}`. Markdown values for `description` \
    or `environment` are converted to Atlassian Document Format \
    automatically — pass plain markdown, not raw ADF JSON. For workflow \
    status changes use jira_transition_issue instead.";
const JIRA_UPDATE_ISSUE_SCOPES: &[&str] = &["write:jira-work"];

pub struct JiraUpdateIssueTool {
    integration: Arc<AtlassianIntegration>,
    description: String,
}

impl JiraUpdateIssueTool {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self {
            integration,
            description: format!(
                "{JIRA_UPDATE_ISSUE_BASE}\n\nRequires Atlassian scope(s): {}.",
                JIRA_UPDATE_ISSUE_SCOPES.join(", ")
            ),
        }
    }
}

#[async_trait]
impl Tool for JiraUpdateIssueTool {
    fn name(&self) -> &str {
        "jira_update_issue"
    }
    fn description(&self) -> &str {
        &self.description
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
                "key": { "type": "string", "description": "Issue key, e.g. 'ENG-123'" },
                "fields": { "type": "object", "description": "Field name → new value map" },
                "site": { "type": "string", "description": "Atlassian site (optional)" }
            },
            "required": ["key", "fields"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        check_scopes(&self.integration, JIRA_UPDATE_ISSUE_SCOPES)?;
        let key = params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'key'".into()))?
            .to_string();
        let fields_value = params
            .get("fields")
            .and_then(|v| v.as_object())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'fields' (object)".into()))?
            .clone();
        let site = site_param(&params);

        // Jira v3 requires ADF (Atlassian Document Format) for rich-text
        // fields. Auto-wrap plain strings the agent may have passed for
        // `description` / `environment`; leave structured ADF blocks alone.
        let mut fields_map: HashMap<String, Value> = HashMap::new();
        for (name, value) in fields_value {
            let promoted = if matches!(name.as_str(), "description" | "environment") {
                if let Some(s) = value.as_str() {
                    adf_from_markdown(s)
                } else {
                    value
                }
            } else {
                value
            };
            fields_map.insert(name, promoted);
        }
        let details = IssueUpdateDetails {
            fields: Some(fields_map),
            ..Default::default()
        };

        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let cfg = client.jira_config(site).await.map_err(integ_err)?;
        // `return_issue=true` makes the endpoint respond with a JSON
        // body. Without it Atlassian returns 204 No Content and the
        // generated client errors trying to parse an empty body.
        issues_api::edit_issue(
            &cfg,
            &key,
            details,
            None,
            None,
            None,
            Some(true),
            None,
        )
        .await
        .map_err(openapi_err)?;
        Ok(ToolOutput::success(json!({"key": key, "ok": true}).to_string()))
    }
}

// ─── /jira_add_comment ────────────────────────────────────────────────────

const JIRA_ADD_COMMENT_BASE: &str = "Add a comment to a Jira issue. `body` accepts \
    markdown (headings, lists, **bold**, *italic*, `code`, links) — \
    converted to Atlassian Document Format automatically.";
const JIRA_ADD_COMMENT_SCOPES: &[&str] = &["write:jira-work"];

pub struct JiraAddCommentTool {
    integration: Arc<AtlassianIntegration>,
    description: String,
}

impl JiraAddCommentTool {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self {
            integration,
            description: format!(
                "{JIRA_ADD_COMMENT_BASE}\n\nRequires Atlassian scope(s): {}.",
                JIRA_ADD_COMMENT_SCOPES.join(", ")
            ),
        }
    }
}

#[async_trait]
impl Tool for JiraAddCommentTool {
    fn name(&self) -> &str {
        "jira_add_comment"
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
                "key": { "type": "string", "description": "Issue key" },
                "body": { "type": "string", "description": "Comment text" },
                "site": { "type": "string", "description": "Atlassian site (optional)" }
            },
            "required": ["key", "body"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        check_scopes(&self.integration, JIRA_ADD_COMMENT_SCOPES)?;
        let key = params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'key'".into()))?
            .to_string();
        let body_text = params
            .get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'body'".into()))?
            .to_string();
        let site = site_param(&params);

        // Comment.body is `Option<Value>`; we pass an ADF doc.
        let comment: Comment = serde_json::from_value(json!({
            "body": adf_from_markdown(&body_text),
        }))
        .map_err(|e| ToolError::ExecutionFailed(format!("build Comment: {e}")))?;

        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let cfg = client.jira_config(site).await.map_err(integ_err)?;
        let resp = issue_comments_api::add_comment(&cfg, &key, comment, None)
            .await
            .map_err(openapi_err)?;
        Ok(ToolOutput::success(serde_json::to_string(&resp).unwrap()))
    }
}

// ─── /jira_transition_issue ───────────────────────────────────────────────

const JIRA_TRANSITION_ISSUE_BASE: &str = "Move a Jira issue to a different status \
    (e.g. 'In Progress' → 'Done'). The transition_name should match one of \
    the names from jira_get_issue's available_transitions list — the tool \
    resolves it to the correct transition_id automatically.";
const JIRA_TRANSITION_ISSUE_SCOPES: &[&str] = &["write:jira-work"];

pub struct JiraTransitionIssueTool {
    integration: Arc<AtlassianIntegration>,
    description: String,
}

impl JiraTransitionIssueTool {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self {
            integration,
            description: format!(
                "{JIRA_TRANSITION_ISSUE_BASE}\n\nRequires Atlassian scope(s): {}.",
                JIRA_TRANSITION_ISSUE_SCOPES.join(", ")
            ),
        }
    }
}

#[async_trait]
impl Tool for JiraTransitionIssueTool {
    fn name(&self) -> &str {
        "jira_transition_issue"
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
                "key": { "type": "string", "description": "Issue key" },
                "transition_name": {
                    "type": "string",
                    "description": "Name of the transition (matches available_transitions[].name)"
                },
                "site": { "type": "string", "description": "Atlassian site (optional)" }
            },
            "required": ["key", "transition_name"]
        })
    }
    async fn execute(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError> {
        check_scopes(&self.integration, JIRA_TRANSITION_ISSUE_SCOPES)?;
        let key = params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'key'".into()))?
            .to_string();
        let transition_name = params
            .get("transition_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'transition_name'".into()))?
            .to_string();
        let site = site_param(&params);

        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let cfg = client.jira_config(site).await.map_err(integ_err)?;

        // Resolve transition_name → transition_id.
        let resp = issues_api::get_transitions(&cfg, &key, None, None, None, None, None)
            .await
            .map_err(openapi_err)?;
        let target = resp
            .transitions
            .unwrap_or_default()
            .into_iter()
            .find(|t| {
                t.name
                    .as_deref()
                    .map(|n| n.eq_ignore_ascii_case(&transition_name))
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                ToolError::ExecutionFailed(format!(
                    "no transition named '{transition_name}' available on {key}. \
                     Use jira_get_issue to see available_transitions."
                ))
            })?;

        let target_id = target.id.clone().unwrap_or_default();
        let target_name = target.name.clone().unwrap_or_default();

        // Build IssueUpdateDetails containing only the transition object.
        let details: IssueUpdateDetails = serde_json::from_value(json!({
            "transition": { "id": target_id },
        }))
        .map_err(|e| ToolError::ExecutionFailed(format!("build IssueUpdateDetails: {e}")))?;

        match issues_api::do_transition(&cfg, &key, details).await {
            Ok(_) => {}
            Err(e) => tolerate_empty_body(e)?,
        }
        Ok(ToolOutput::success(
            json!({"key": key, "transitioned_to": target_name, "transition_id": target_id})
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_issue_extracts_nested_fields() {
        let raw_json = json!({
            "summary": "Test bug",
            "status": { "name": "In Progress" },
            "issuetype": { "name": "Bug" },
            "priority": { "name": "High" },
            "assignee": { "displayName": "Alice" },
            "reporter": { "displayName": "Bob" },
            "updated": "2026-05-05T10:00:00.000+0000"
        });
        let fields: Map<String, Value> = raw_json.as_object().unwrap().clone();
        let s = summarize_issue("ENG-1", &fields);
        assert_eq!(s.key, "ENG-1");
        assert_eq!(s.summary.as_deref(), Some("Test bug"));
        assert_eq!(s.status.as_deref(), Some("In Progress"));
        assert_eq!(s.issue_type.as_deref(), Some("Bug"));
        assert_eq!(s.priority.as_deref(), Some("High"));
        assert_eq!(s.assignee.as_deref(), Some("Alice"));
        assert_eq!(s.reporter.as_deref(), Some("Bob"));
    }

    #[test]
    fn summarize_issue_handles_missing_fields() {
        let s = summarize_issue("ENG-2", &Map::new());
        assert_eq!(s.key, "ENG-2");
        assert!(s.summary.is_none());
        assert!(s.assignee.is_none());
    }
}
