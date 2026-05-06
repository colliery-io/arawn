//! Jira tools — search, get, create, update, comment, transition.

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

// ─── Response shapes ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SearchResults {
    #[serde(default)]
    issues: Vec<RawIssue>,
    #[serde(default)]
    total: u64,
}

#[derive(Debug, Deserialize)]
struct RawIssue {
    key: String,
    #[serde(default)]
    fields: serde_json::Map<String, Value>,
}

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

fn summarize_issue(raw: &RawIssue) -> IssueSummary {
    let f = &raw.fields;
    IssueSummary {
        key: raw.key.clone(),
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
        updated: f.get("updated").and_then(|v| v.as_str()).map(String::from),
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

// ─── /jira_search ─────────────────────────────────────────────────────────

const JIRA_SEARCH_BASE: &str = "Search Jira issues using JQL (Jira Query Language). \
    Examples: `project = ENG AND status = 'In Progress'`, \
    `assignee = currentUser() AND updated >= -7d`, \
    `text ~ 'login bug'`. Returns issues with key, summary, status, \
    type, priority, assignee, updated. The `site` arg picks among connected \
    Atlassian sites (e.g. 'acme.atlassian.net'); omit for default site.";
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
            .and_then(|v| v.as_u64())
            .unwrap_or(25)
            .min(100);
        let site = site_param(&params);

        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let results: SearchResults = client
            .jira_get(
                "/search",
                site,
                &[
                    ("jql", jql),
                    ("maxResults", max.to_string()),
                    (
                        "fields",
                        "summary,status,issuetype,priority,assignee,reporter,updated".into(),
                    ),
                ],
            )
            .await
            .map_err(integ_err)?;
        let summaries: Vec<IssueSummary> = results.issues.iter().map(summarize_issue).collect();
        Ok(ToolOutput::success(
            json!({"issues": summaries, "total": results.total}).to_string(),
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

#[derive(Debug, Deserialize)]
struct RawIssueDetail {
    key: String,
    #[serde(default)]
    fields: serde_json::Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct TransitionsResp {
    #[serde(default)]
    transitions: Vec<RawTransition>,
}

#[derive(Debug, Deserialize)]
struct RawTransition {
    id: String,
    name: String,
    #[serde(default)]
    to: Option<RawTransitionTo>,
}

#[derive(Debug, Deserialize)]
struct RawTransitionTo {
    name: Option<String>,
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

        let issue: RawIssueDetail = client
            .jira_get(
                &format!("/issue/{key}"),
                site,
                &[(
                    "fields",
                    "summary,status,issuetype,priority,assignee,reporter,description,created,updated,comment".into(),
                )],
            )
            .await
            .map_err(integ_err)?;

        let transitions_resp: TransitionsResp = client
            .jira_get(&format!("/issue/{key}/transitions"), site, &[])
            .await
            .map_err(integ_err)?;

        let f = &issue.fields;
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
                        body: c
                            .get("body")
                            .map(|v| {
                                if let Some(s) = v.as_str() {
                                    s.to_string()
                                } else {
                                    serde_json::to_string(v).unwrap_or_default()
                                }
                            }),
                        created: c
                            .get("created")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let detail = IssueDetail {
            key: issue.key.clone(),
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
            description: f
                .get("description")
                .map(|v| {
                    if let Some(s) = v.as_str() {
                        s.to_string()
                    } else {
                        serde_json::to_string(v).unwrap_or_default()
                    }
                }),
            created: f.get("created").and_then(|v| v.as_str()).map(String::from),
            updated: f.get("updated").and_then(|v| v.as_str()).map(String::from),
            comments,
            available_transitions: transitions_resp
                .transitions
                .into_iter()
                .map(|t| TransitionSummary {
                    id: t.id,
                    name: t.name,
                    to: t.to.and_then(|to| to.name),
                })
                .collect(),
        };

        Ok(ToolOutput::success(serde_json::to_string(&detail).unwrap()))
    }
}

// ─── /jira_create_issue ───────────────────────────────────────────────────

const JIRA_CREATE_ISSUE_BASE: &str = "Create a new Jira issue in the given project. \
    Returns the new issue's key. `description` accepts plain text — Jira's \
    Atlassian Document Format is auto-wrapped. `issue_type` is the type name \
    (e.g. 'Task', 'Bug', 'Story').";
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

        let mut fields = json!({
            "project": { "key": project_key },
            "summary": summary,
            "issuetype": { "name": issue_type },
        });
        if let Some(desc) = description {
            // Jira API v3 wants ADF (Atlassian Document Format). Build a
            // minimal ADF doc with a single paragraph; markdown-ish input
            // becomes plain text — good enough for v1.
            fields["description"] = json!({
                "type": "doc",
                "version": 1,
                "content": [{
                    "type": "paragraph",
                    "content": [{ "type": "text", "text": desc }],
                }],
            });
        }
        let body = json!({ "fields": fields });

        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let resp: Value = client
            .jira_post("/issue", site, &body)
            .await
            .map_err(integ_err)?;
        Ok(ToolOutput::success(resp.to_string()))
    }
}

// ─── /jira_update_issue ───────────────────────────────────────────────────

const JIRA_UPDATE_ISSUE_BASE: &str = "Update fields on an existing Jira issue. Pass \
    `fields` as a JSON object using Jira field names (e.g. \
    `{\"summary\": \"new title\", \"priority\": {\"name\": \"High\"}}`). For \
    workflow status changes, use jira_transition_issue instead.";
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
        let fields = params
            .get("fields")
            .ok_or_else(|| ToolError::ExecutionFailed("missing 'fields'".into()))?
            .clone();
        let site = site_param(&params);

        let body = json!({ "fields": fields });
        let client = AtlassianClient::new(Arc::clone(&self.integration));
        client
            .jira_put(&format!("/issue/{key}"), site, &body)
            .await
            .map_err(integ_err)?;
        Ok(ToolOutput::success(json!({"key": key, "ok": true}).to_string()))
    }
}

// ─── /jira_add_comment ────────────────────────────────────────────────────

const JIRA_ADD_COMMENT_BASE: &str = "Add a comment to a Jira issue. `body` is plain \
    text — wrapped in Atlassian Document Format automatically.";
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

        let body = json!({
            "body": {
                "type": "doc",
                "version": 1,
                "content": [{
                    "type": "paragraph",
                    "content": [{ "type": "text", "text": body_text }],
                }],
            }
        });

        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let resp: Value = client
            .jira_post(&format!("/issue/{key}/comment"), site, &body)
            .await
            .map_err(integ_err)?;
        Ok(ToolOutput::success(resp.to_string()))
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

        // Resolve transition_name → transition_id.
        let resp: TransitionsResp = client
            .jira_get(&format!("/issue/{key}/transitions"), site, &[])
            .await
            .map_err(integ_err)?;
        let target = resp
            .transitions
            .into_iter()
            .find(|t| t.name.eq_ignore_ascii_case(&transition_name))
            .ok_or_else(|| {
                ToolError::ExecutionFailed(format!(
                    "no transition named '{transition_name}' available on {key}. \
                     Use jira_get_issue to see available_transitions."
                ))
            })?;

        let body = json!({ "transition": { "id": target.id.clone() } });
        let client2 = AtlassianClient::new(Arc::clone(&self.integration));
        // POST returns 204 No Content; piggyback send_no_body via jira_put? No,
        // POST is the right method here. Use raw post and ignore body.
        let _: Value = client2
            .jira_post(&format!("/issue/{key}/transitions"), site, &body)
            .await
            // Some endpoints return 204 with empty body; treat decode-of-empty
            // as success.
            .or_else(|e| match e {
                crate::IntegrationError::Provider(msg) if msg.contains("decode body") => {
                    Ok(Value::Null)
                }
                e => Err(e),
            })
            .map_err(integ_err)?;
        Ok(ToolOutput::success(
            json!({"key": key, "transitioned_to": target.name, "transition_id": target.id})
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
            "key": "ENG-1",
            "fields": {
                "summary": "Test bug",
                "status": { "name": "In Progress" },
                "issuetype": { "name": "Bug" },
                "priority": { "name": "High" },
                "assignee": { "displayName": "Alice" },
                "reporter": { "displayName": "Bob" },
                "updated": "2026-05-05T10:00:00.000+0000"
            }
        });
        let raw: RawIssue = serde_json::from_value(raw_json).unwrap();
        let s = summarize_issue(&raw);
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
        let raw_json = json!({ "key": "ENG-2", "fields": {} });
        let raw: RawIssue = serde_json::from_value(raw_json).unwrap();
        let s = summarize_issue(&raw);
        assert_eq!(s.key, "ENG-2");
        assert!(s.summary.is_none());
        assert!(s.assignee.is_none());
    }
}
