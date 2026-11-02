//! Linear agent tools (COLLIERY-I-0532 T-C).
//!
//! Read: `linear_list_my_issues`, `linear_get_issue`. Write:
//! `linear_comment`, `linear_set_state`, `linear_set_assignee`,
//! `linear_set_priority`. Each tool takes an issue as its identifier
//! (`ENG-123`) or its id, and builds a [`super::LinearClient`] from the
//! stored key on every call. Each write gives back the new value and the
//! issue URL. Names given by the user (a state, a person) are resolved
//! against the issue's team; a miss lists the valid names.

use std::sync::Arc;

use arawn_tool::{PermissionCategory, Tool, ToolCategory, ToolContext, ToolError, ToolOutput};
use async_trait::async_trait;
use serde_json::{Value, json};

use super::client::LinearClient;
use super::integration::LinearIntegration;
use super::models::{Issue, Member};

fn integ_err(e: crate::IntegrationError) -> ToolError {
    ToolError::ExecutionFailed(e.user_message())
}

fn fail(msg: impl Into<String>) -> ToolError {
    ToolError::ExecutionFailed(msg.into())
}

fn str_param<'a>(params: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| fail(format!("missing '{key}'")))
}

fn client(integration: &LinearIntegration) -> Result<LinearClient, ToolError> {
    integration.client().map_err(integ_err)
}

const ISSUE_PARAM: &str = "Issue identifier (e.g. ENG-123) or id";

/// The compact view a list gives per issue.
fn summary(i: &Issue) -> Value {
    json!({
        "identifier": i.identifier,
        "title": i.title,
        "state": i.state.name,
        "priority": i.priority_label,
        "due_date": i.due_date,
        "team": i.team.key,
        "assignee": i.assignee.as_ref().map(|a| &a.name),
        "updated_at": i.updated_at,
        "url": i.url,
    })
}

/// Everything `linear_get_issue` gives.
fn detail(i: &Issue) -> Value {
    json!({
        "id": i.id,
        "identifier": i.identifier,
        "title": i.title,
        "description": i.description,
        "state": { "name": i.state.name, "type": i.state.kind },
        "priority": i.priority,
        "priority_label": i.priority_label,
        "due_date": i.due_date,
        "team": { "key": i.team.key, "name": i.team.name },
        "labels": i.labels.nodes.iter().map(|l| &l.name).collect::<Vec<_>>(),
        "assignee": i.assignee.as_ref().map(|a| json!({"name": a.name, "email": a.email})),
        "creator": i.creator.as_ref().map(|c| json!({"name": c.name, "email": c.email})),
        "created_at": i.created_at,
        "updated_at": i.updated_at,
        "url": i.url,
        "comments": i.comments.nodes.iter().map(|c| json!({
            "author": c.user.as_ref().map(|u| &u.name),
            "body": c.body,
            "created_at": c.created_at,
        })).collect::<Vec<_>>(),
    })
}

/// Run a tool body and wrap its JSON in a [`ToolOutput`].
fn output(v: Result<Value, ToolError>) -> Result<ToolOutput, ToolError> {
    v.map(|v| ToolOutput::success(v.to_string()))
}

// ─── linear_list_my_issues ───────────────────────────────────────────────

pub struct LinearListMyIssuesTool {
    integration: Arc<LinearIntegration>,
}

impl LinearListMyIssuesTool {
    pub fn new(integration: Arc<LinearIntegration>) -> Self {
        Self { integration }
    }

    pub(crate) async fn run(&self, params: &Value) -> Result<Value, ToolError> {
        let state = params
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("open");
        let limit = params
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(25)
            .clamp(1, 100) as usize;
        let mut filter = super::my_issues_filter(None);
        match state {
            "open" => filter["state"] = json!({ "type": { "nin": ["completed", "canceled"] } }),
            "closed" => filter["state"] = json!({ "type": { "in": ["completed", "canceled"] } }),
            "all" => {}
            other => {
                return Err(fail(format!(
                    "unknown state filter `{other}`. Use open, closed or all"
                )));
            }
        }
        let c = client(&self.integration)?;
        let mut out = Vec::new();
        let mut after: Option<String> = None;
        loop {
            let (_, page) = c
                .issues_page(filter.clone(), after.as_deref())
                .await
                .map_err(integ_err)?;
            out.extend(page.issues.iter().map(summary));
            match page.next {
                Some(next) if out.len() < limit => after = Some(next),
                _ => break,
            }
        }
        out.truncate(limit);
        Ok(json!({ "issues": out }))
    }
}

#[async_trait]
impl Tool for LinearListMyIssuesTool {
    fn name(&self) -> &str {
        "linear_list_my_issues"
    }
    fn description(&self) -> &str {
        "List the Linear issues assigned to me or created by me, most recently updated \
         first. `state`: open (default), closed or all. Gives identifier, title, state, \
         priority, due date, team, assignee and URL per issue."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Linear
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "state": { "type": "string", "enum": ["open", "closed", "all"],
                           "description": "Which issues (default open)" },
                "limit": { "type": "integer", "description": "Max issues (default 25, max 100)" }
            }
        })
    }
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        output(self.run(&params).await)
    }
}

// ─── linear_get_issue ────────────────────────────────────────────────────

pub struct LinearGetIssueTool {
    integration: Arc<LinearIntegration>,
}

impl LinearGetIssueTool {
    pub fn new(integration: Arc<LinearIntegration>) -> Self {
        Self { integration }
    }

    pub(crate) async fn run(&self, params: &Value) -> Result<Value, ToolError> {
        let id = str_param(params, "issue")?;
        let issue = client(&self.integration)?
            .issue(id)
            .await
            .map_err(integ_err)?;
        Ok(detail(&issue))
    }
}

#[async_trait]
impl Tool for LinearGetIssueTool {
    fn name(&self) -> &str {
        "linear_get_issue"
    }
    fn description(&self) -> &str {
        "Get one Linear issue with its description, state, priority, due date, labels, \
         assignee and the ten newest comments."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Linear
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::ReadOnly
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "issue": { "type": "string", "description": ISSUE_PARAM } },
            "required": ["issue"]
        })
    }
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        output(self.run(&params).await)
    }
}

// ─── linear_comment ──────────────────────────────────────────────────────

pub struct LinearCommentTool {
    integration: Arc<LinearIntegration>,
}

impl LinearCommentTool {
    pub fn new(integration: Arc<LinearIntegration>) -> Self {
        Self { integration }
    }

    pub(crate) async fn run(&self, params: &Value) -> Result<Value, ToolError> {
        let id = str_param(params, "issue")?;
        let body = str_param(params, "body")?;
        let c = client(&self.integration)?;
        // commentCreate takes the issue's id, not its identifier.
        let issue = c.issue_ref(id).await.map_err(integ_err)?;
        let comment = c.comment_create(&issue.id, body).await.map_err(integ_err)?;
        Ok(json!({
            "identifier": issue.identifier,
            "comment_url": comment.url,
            "url": issue.url,
        }))
    }
}

#[async_trait]
impl Tool for LinearCommentTool {
    fn name(&self) -> &str {
        "linear_comment"
    }
    fn description(&self) -> &str {
        "Add a comment to a Linear issue. `body` is Markdown."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Linear
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::Other
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "issue": { "type": "string", "description": ISSUE_PARAM },
                "body": { "type": "string", "description": "Comment text (Markdown)" }
            },
            "required": ["issue", "body"]
        })
    }
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        output(self.run(&params).await)
    }
}

// ─── linear_set_state ────────────────────────────────────────────────────

pub struct LinearSetStateTool {
    integration: Arc<LinearIntegration>,
}

impl LinearSetStateTool {
    pub fn new(integration: Arc<LinearIntegration>) -> Self {
        Self { integration }
    }

    pub(crate) async fn run(&self, params: &Value) -> Result<Value, ToolError> {
        let id = str_param(params, "issue")?;
        let wanted = str_param(params, "state")?;
        let c = client(&self.integration)?;
        let (issue, states) = c.issue_team_states(id).await.map_err(integ_err)?;
        let Some(state) = states.iter().find(|s| s.name.eq_ignore_ascii_case(wanted)) else {
            let names: Vec<&str> = states.iter().map(|s| s.name.as_str()).collect();
            return Err(fail(format!(
                "no state `{wanted}` in team {}. Valid states: {}",
                issue.team.key,
                names.join(", ")
            )));
        };
        let updated = c
            .issue_update(&issue.id, json!({ "stateId": state.id }))
            .await
            .map_err(integ_err)?;
        Ok(json!({
            "identifier": updated.identifier,
            "state": updated.state.name,
            "url": updated.url,
        }))
    }
}

#[async_trait]
impl Tool for LinearSetStateTool {
    fn name(&self) -> &str {
        "linear_set_state"
    }
    fn description(&self) -> &str {
        "Move a Linear issue to a workflow state by name (e.g. \"In Progress\", \"Done\"), \
         case-insensitive. The name must be a state of the issue's team; an unknown name \
         gives the list of valid ones."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Linear
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::Other
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "issue": { "type": "string", "description": ISSUE_PARAM },
                "state": { "type": "string", "description": "Workflow state name" }
            },
            "required": ["issue", "state"]
        })
    }
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        output(self.run(&params).await)
    }
}

// ─── linear_set_assignee ─────────────────────────────────────────────────

pub struct LinearSetAssigneeTool {
    integration: Arc<LinearIntegration>,
}

/// Find one member by email (exact, case-insensitive), else by display
/// name or full name. `Err` lists the candidates.
fn resolve_member<'a>(members: &'a [Member], wanted: &str) -> Result<&'a Member, String> {
    let by_email: Vec<&Member> = members
        .iter()
        .filter(|m| {
            m.email
                .as_deref()
                .is_some_and(|e| e.eq_ignore_ascii_case(wanted))
        })
        .collect();
    let hits = if by_email.is_empty() {
        members
            .iter()
            .filter(|m| {
                m.name.eq_ignore_ascii_case(wanted) || m.display_name.eq_ignore_ascii_case(wanted)
            })
            .collect()
    } else {
        by_email
    };
    let list = |ms: &mut dyn Iterator<Item = &Member>| -> String {
        ms.map(|m| match &m.email {
            Some(e) => format!("{} <{e}>", m.name),
            None => m.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
    };
    match hits.as_slice() {
        [one] => Ok(one),
        [] => Err(format!(
            "no member `{wanted}` in the issue's team. Members: {}",
            list(&mut members.iter())
        )),
        many => Err(format!(
            "`{wanted}` matches more than one member: {}. Give the email",
            list(&mut many.iter().copied())
        )),
    }
}

impl LinearSetAssigneeTool {
    pub fn new(integration: Arc<LinearIntegration>) -> Self {
        Self { integration }
    }

    pub(crate) async fn run(&self, params: &Value) -> Result<Value, ToolError> {
        let id = str_param(params, "issue")?;
        let wanted = str_param(params, "assignee")?;
        let c = client(&self.integration)?;
        let assignee_id: Value = match wanted.to_ascii_lowercase().as_str() {
            "none" => Value::Null,
            "me" => {
                let creds = self
                    .integration
                    .load_credentials()
                    .map_err(integ_err)?
                    .ok_or_else(|| fail("linear is not set up — run: arawn setup linear"))?;
                json!(creds.viewer_id)
            }
            _ => {
                let (issue, members) = c.issue_team_members(id).await.map_err(integ_err)?;
                let m = resolve_member(&members, wanted)
                    .map_err(|e| fail(format!("{e} (team {})", issue.team.key)))?;
                json!(m.id)
            }
        };
        let updated = c
            .issue_update(id, json!({ "assigneeId": assignee_id }))
            .await
            .map_err(integ_err)?;
        Ok(json!({
            "identifier": updated.identifier,
            "assignee": updated.assignee.as_ref().map(|a| &a.name),
            "url": updated.url,
        }))
    }
}

#[async_trait]
impl Tool for LinearSetAssigneeTool {
    fn name(&self) -> &str {
        "linear_set_assignee"
    }
    fn description(&self) -> &str {
        "Assign a Linear issue. `assignee`: `me`, a team member's email or display name, \
         or `none` to unassign. An unknown name gives the team's members."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Linear
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::Other
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "issue": { "type": "string", "description": ISSUE_PARAM },
                "assignee": { "type": "string",
                              "description": "me, an email, a display name, or none" }
            },
            "required": ["issue", "assignee"]
        })
    }
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        output(self.run(&params).await)
    }
}

// ─── linear_set_priority ─────────────────────────────────────────────────

pub struct LinearSetPriorityTool {
    integration: Arc<LinearIntegration>,
}

/// 0–4, as a number or a digit string, or `none|urgent|high|medium|low`.
fn parse_priority(v: &Value) -> Result<u8, String> {
    let n = match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "none" | "no priority" => Some(0),
            "urgent" => Some(1),
            "high" => Some(2),
            "medium" => Some(3),
            "low" => Some(4),
            other => other.parse().ok(),
        },
        _ => None,
    };
    n.filter(|n| *n <= 4)
        .map(|n| n as u8)
        .ok_or_else(|| format!("priority must be 0–4 or none, urgent, high, medium, low; not {v}"))
}

impl LinearSetPriorityTool {
    pub fn new(integration: Arc<LinearIntegration>) -> Self {
        Self { integration }
    }

    pub(crate) async fn run(&self, params: &Value) -> Result<Value, ToolError> {
        let id = str_param(params, "issue")?;
        let priority = params
            .get("priority")
            .ok_or_else(|| fail("missing 'priority'"))
            .and_then(|v| parse_priority(v).map_err(fail))?;
        let updated = client(&self.integration)?
            .issue_update(id, json!({ "priority": priority }))
            .await
            .map_err(integ_err)?;
        Ok(json!({
            "identifier": updated.identifier,
            "priority": updated.priority,
            "priority_label": updated.priority_label,
            "url": updated.url,
        }))
    }
}

#[async_trait]
impl Tool for LinearSetPriorityTool {
    fn name(&self) -> &str {
        "linear_set_priority"
    }
    fn description(&self) -> &str {
        "Set a Linear issue's priority: 0–4 or none, urgent, high, medium, low \
         (0 none, 1 urgent, 2 high, 3 medium, 4 low)."
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Linear
    }
    fn permission_category(&self) -> PermissionCategory {
        PermissionCategory::Other
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "issue": { "type": "string", "description": ISSUE_PARAM },
                "priority": { "description": "0–4, or none|urgent|high|medium|low" }
            },
            "required": ["issue", "priority"]
        })
    }
    async fn execute(
        &self,
        _ctx: &dyn ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        output(self.run(&params).await)
    }
}

/// Every Linear tool, for startup registration.
pub fn linear_tools(integration: Arc<LinearIntegration>) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(LinearListMyIssuesTool::new(Arc::clone(&integration))) as Box<dyn Tool>,
        Box::new(LinearGetIssueTool::new(Arc::clone(&integration))),
        Box::new(LinearCommentTool::new(Arc::clone(&integration))),
        Box::new(LinearSetStateTool::new(Arc::clone(&integration))),
        Box::new(LinearSetAssigneeTool::new(Arc::clone(&integration))),
        Box::new(LinearSetPriorityTool::new(integration)),
    ]
}

#[cfg(test)]
mod tests {
    use super::super::client::tests::{FakeTransport, issue_json};
    use super::super::integration::LinearCredentials;
    use super::*;
    use tempfile::TempDir;

    /// An integration with a saved key over a fake transport that gives
    /// these bodies in order.
    fn connected(bodies: &[Value]) -> (Arc<LinearIntegration>, Arc<FakeTransport>, TempDir) {
        let tmp = TempDir::new().unwrap();
        let t = FakeTransport::ok(bodies);
        let l = LinearIntegration::new(tmp.path().to_path_buf()).with_transport(t.clone());
        l.save_credentials(&LinearCredentials {
            api_key: "lin_api_k".into(),
            viewer_id: "u-me".into(),
            viewer_name: "Ada".into(),
            organization: "Acme".into(),
        })
        .unwrap();
        (Arc::new(l), t, tmp)
    }

    fn op(body: &Value) -> &str {
        body["query"]
            .as_str()
            .unwrap()
            .split(['(', ' '])
            .nth(1)
            .unwrap()
    }

    fn team_states() -> Value {
        json!({"data": {"issue": {
            "id": "uuid-7", "identifier": "ENG-7", "url": "https://linear.app/acme/issue/ENG-7",
            "team": {"id": "t1", "key": "ENG", "states": {"nodes": [
                {"id": "s-todo", "name": "Todo", "type": "unstarted"},
                {"id": "s-prog", "name": "In Progress", "type": "started"},
                {"id": "s-done", "name": "Done", "type": "completed"}
            ]}}
        }}})
    }

    fn team_members() -> Value {
        json!({"data": {"issue": {
            "id": "uuid-7", "identifier": "ENG-7", "url": "https://linear.app/acme/issue/ENG-7",
            "team": {"id": "t1", "key": "ENG", "members": {"nodes": [
                {"id": "u-me", "name": "Ada Lovelace", "displayName": "ada", "email": "ada@acme.io"},
                {"id": "u-bea", "name": "Bea Smith", "displayName": "bea", "email": "bea@acme.io"},
                {"id": "u-bo", "name": "Bo Smith", "displayName": "bo", "email": null}
            ]}}
        }}})
    }

    fn updated(state: &str, assignee: Option<&str>, priority: u8, label: &str) -> Value {
        json!({"data": {"issueUpdate": {"success": true, "issue": {
            "id": "uuid-7", "identifier": "ENG-7", "url": "https://linear.app/acme/issue/ENG-7",
            "priority": priority, "priorityLabel": label,
            "state": {"name": state},
            "assignee": assignee.map(|a| json!({"name": a}))
        }}}})
    }

    #[tokio::test]
    async fn comment_resolves_the_id_then_sends_comment_create() {
        let (l, t, _tmp) = connected(&[
            json!({"data": {"issue": {"id": "uuid-7", "identifier": "ENG-7",
                "url": "https://linear.app/acme/issue/ENG-7", "team": {"id": "t1", "key": "ENG"}}}}),
            json!({"data": {"commentCreate": {"success": true,
                "comment": {"id": "c9", "url": "https://linear.app/acme/issue/ENG-7#comment-c9"}}}}),
        ]);
        let out = LinearCommentTool::new(l)
            .run(&json!({"issue": "ENG-7", "body": "Fixed in **#42**"}))
            .await
            .unwrap();
        let sent = t.sent();
        assert_eq!(op(&sent[0].1), "IssueRef");
        assert_eq!(sent[0].1["variables"]["id"], "ENG-7");
        assert_eq!(op(&sent[1].1), "CommentCreate");
        assert_eq!(
            sent[1].1["variables"],
            json!({"issueId": "uuid-7", "body": "Fixed in **#42**"})
        );
        assert_eq!(sent[1].0, "lin_api_k");
        assert_eq!(out["url"], "https://linear.app/acme/issue/ENG-7");
        assert!(out["comment_url"].as_str().unwrap().contains("comment-c9"));
    }

    #[tokio::test]
    async fn set_state_resolves_the_name_case_insensitively() {
        let (l, t, _tmp) = connected(&[team_states(), updated("In Progress", None, 2, "High")]);
        let out = LinearSetStateTool::new(l)
            .run(&json!({"issue": "ENG-7", "state": "in progress"}))
            .await
            .unwrap();
        let sent = t.sent();
        assert_eq!(op(&sent[0].1), "IssueTeamStates");
        assert_eq!(op(&sent[1].1), "IssueUpdate");
        assert_eq!(
            sent[1].1["variables"],
            json!({"id": "uuid-7", "input": {"stateId": "s-prog"}})
        );
        assert_eq!(out["state"], "In Progress");
        assert_eq!(out["url"], "https://linear.app/acme/issue/ENG-7");
    }

    #[tokio::test]
    async fn unknown_state_lists_the_valid_ones_and_sends_no_update() {
        let (l, t, _tmp) = connected(&[team_states()]);
        let err = LinearSetStateTool::new(l)
            .run(&json!({"issue": "ENG-7", "state": "Shipped"}))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("no state `Shipped` in team ENG"), "{err}");
        assert!(err.contains("Todo, In Progress, Done"), "{err}");
        assert_eq!(t.sent().len(), 1);
    }

    #[tokio::test]
    async fn set_assignee_by_email_name_me_and_none() {
        // By email.
        let (l, t, _tmp) = connected(&[
            team_members(),
            updated("Todo", Some("Bea Smith"), 2, "High"),
        ]);
        let out = LinearSetAssigneeTool::new(l)
            .run(&json!({"issue": "ENG-7", "assignee": "BEA@acme.io"}))
            .await
            .unwrap();
        assert_eq!(op(&t.sent()[0].1), "IssueTeamMembers");
        assert_eq!(
            t.sent()[1].1["variables"],
            json!({"id": "ENG-7", "input": {"assigneeId": "u-bea"}})
        );
        assert_eq!(out["assignee"], "Bea Smith");

        // By display name.
        let (l, t, _tmp) =
            connected(&[team_members(), updated("Todo", Some("Bo Smith"), 2, "High")]);
        LinearSetAssigneeTool::new(l)
            .run(&json!({"issue": "ENG-7", "assignee": "bo"}))
            .await
            .unwrap();
        assert_eq!(t.sent()[1].1["variables"]["input"]["assigneeId"], "u-bo");

        // `me` is the key's user, with no member lookup.
        let (l, t, _tmp) = connected(&[updated("Todo", Some("Ada Lovelace"), 2, "High")]);
        LinearSetAssigneeTool::new(l)
            .run(&json!({"issue": "ENG-7", "assignee": "me"}))
            .await
            .unwrap();
        assert_eq!(t.sent().len(), 1);
        assert_eq!(t.sent()[0].1["variables"]["input"]["assigneeId"], "u-me");

        // `none` unassigns.
        let (l, t, _tmp) = connected(&[updated("Todo", None, 2, "High")]);
        let out = LinearSetAssigneeTool::new(l)
            .run(&json!({"issue": "ENG-7", "assignee": "none"}))
            .await
            .unwrap();
        assert!(t.sent()[0].1["variables"]["input"]["assigneeId"].is_null());
        assert!(out["assignee"].is_null());
    }

    #[tokio::test]
    async fn unknown_or_ambiguous_assignee_lists_the_candidates() {
        let (l, t, _tmp) = connected(&[team_members()]);
        let err = LinearSetAssigneeTool::new(l)
            .run(&json!({"issue": "ENG-7", "assignee": "Carol"}))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("no member `Carol`"), "{err}");
        assert!(
            err.contains("Ada Lovelace <ada@acme.io>, Bea Smith <bea@acme.io>, Bo Smith"),
            "{err}"
        );
        assert_eq!(t.sent().len(), 1, "no update sent");

        let members: Vec<Member> = serde_json::from_value(
            team_members()["data"]["issue"]["team"]["members"]["nodes"].clone(),
        )
        .unwrap();
        let mut twins = members.clone();
        twins[2].display_name = "bea".into();
        let err = resolve_member(&twins, "bea").unwrap_err();
        assert!(err.contains("more than one member"), "{err}");
    }

    #[tokio::test]
    async fn set_priority_maps_words_and_numbers() {
        for (given, sent) in [
            (json!("urgent"), 1),
            (json!("High"), 2),
            (json!("medium"), 3),
            (json!("low"), 4),
            (json!("none"), 0),
            (json!(3), 3),
            (json!("2"), 2),
        ] {
            let (l, t, _tmp) = connected(&[updated("Todo", None, sent, "Some")]);
            let out = LinearSetPriorityTool::new(l)
                .run(&json!({"issue": "ENG-7", "priority": given}))
                .await
                .unwrap();
            assert_eq!(op(&t.sent()[0].1), "IssueUpdate");
            assert_eq!(
                t.sent()[0].1["variables"],
                json!({"id": "ENG-7", "input": {"priority": sent}}),
                "{given}"
            );
            assert_eq!(out["priority"], sent);
        }
        assert!(parse_priority(&json!(5)).is_err());
        assert!(parse_priority(&json!("soon")).is_err());
    }

    #[tokio::test]
    async fn list_asks_for_open_issues_and_follows_pages_up_to_the_limit() {
        let page = |ids: &[&str], next: Option<&str>| {
            json!({"data": {"viewer": {"id": "u-me"}, "issues": {
                "nodes": ids.iter().map(|i| issue_json(i, "2026-10-05T00:00:00.000Z")).collect::<Vec<_>>(),
                "pageInfo": {"hasNextPage": next.is_some(), "endCursor": next}
            }}})
        };
        let (l, t, _tmp) = connected(&[
            page(&["ENG-1", "ENG-2"], Some("c1")),
            page(&["ENG-3"], None),
        ]);
        let out = LinearListMyIssuesTool::new(l)
            .run(&json!({"limit": 2}))
            .await
            .unwrap();
        assert_eq!(out["issues"].as_array().unwrap().len(), 2);
        assert_eq!(out["issues"][0]["identifier"], "ENG-1");
        let sent = t.sent();
        assert_eq!(sent.len(), 1, "the limit was reached on page one");
        assert_eq!(
            sent[0].1["variables"]["filter"]["state"],
            json!({"type": {"nin": ["completed", "canceled"]}})
        );
    }

    #[tokio::test]
    async fn get_issue_gives_the_comments() {
        let (l, _t, _tmp) = connected(&[json!({"data": {"issue":
            issue_json("ENG-1", "2026-10-05T00:00:00.000Z")}})]);
        let out = LinearGetIssueTool::new(l)
            .run(&json!({"issue": "ENG-1"}))
            .await
            .unwrap();
        assert_eq!(out["comments"][0]["body"], "On it");
        assert_eq!(out["state"]["type"], "started");
    }

    #[test]
    fn read_tools_are_read_only_and_writes_ask() {
        let l = Arc::new(LinearIntegration::new(std::env::temp_dir()));
        for t in linear_tools(l) {
            assert_eq!(t.category(), ToolCategory::Linear);
            let read = matches!(t.name(), "linear_list_my_issues" | "linear_get_issue");
            assert_eq!(
                t.permission_category() == PermissionCategory::ReadOnly,
                read,
                "{}",
                t.name()
            );
        }
    }
}
