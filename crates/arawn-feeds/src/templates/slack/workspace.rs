//! The `workspace` param every Slack template takes (COLLIERY-I-0520): the
//! alias of the Slack workspace the feed reads, `acme` for `slack@acme`.

use std::sync::Arc;

use serde_json::json;

use crate::clients::SlackFeedClient;
use crate::error::FeedError;
use crate::param_schema::{ParamKind, ParamSpec};
use crate::template::{DiscoveryRow, TemplateCtx};
use crate::types::TemplateParams;

/// The `workspace` param, checked for shape only (no client).
pub fn workspace_param<'a>(
    template: &str,
    params: &'a TemplateParams,
) -> Result<&'a str, FeedError> {
    let ws = params.get_str("workspace").ok_or_else(|| {
        FeedError::InvalidParams(format!(
            "{template} requires `workspace` (the alias of a connected Slack workspace, \
             like `acme` for slack@acme)"
        ))
    })?;
    if !arawn_integrations::slack::is_valid_alias(ws) {
        return Err(FeedError::InvalidParams(format!(
            "{template} `workspace` `{ws}` is not a Slack workspace alias"
        )));
    }
    Ok(ws)
}

/// The `workspace` param, which must name a connected workspace. The
/// error lists the connected ones.
pub fn check_workspace(
    template: &str,
    ctx: &TemplateCtx,
    params: &TemplateParams,
) -> Result<(), FeedError> {
    let connected = ctx.clients().slack_workspaces();
    let listed = if connected.is_empty() {
        "no Slack workspace is connected (run: arawn connect slack)".to_string()
    } else {
        format!("connected: {}", connected.join(", "))
    };
    let ws = workspace_param(template, params)
        .map_err(|e| FeedError::InvalidParams(format!("{}; {listed}", strip(e))))?;
    if !connected.iter().any(|c| c == ws) {
        return Err(FeedError::InvalidParams(format!(
            "{template}: Slack workspace `{ws}` is not connected; {listed}"
        )));
    }
    Ok(())
}

fn strip(e: FeedError) -> String {
    match e {
        FeedError::InvalidParams(m) => m,
        other => other.to_string(),
    }
}

/// The client of the feed's workspace.
pub fn client(
    template: &str,
    ctx: &TemplateCtx,
    params: &TemplateParams,
) -> Result<Arc<dyn SlackFeedClient>, FeedError> {
    let ws = workspace_param(template, params)?;
    ctx.clients()
        .slack(ws)
        .ok_or_else(|| FeedError::Auth(format!("slack@{ws} is not connected")))
}

/// The `workspace` field of the `/watch` form: picked from the
/// connected workspaces.
pub fn workspace_spec() -> ParamSpec {
    ParamSpec::required(
        "workspace",
        "Workspace",
        ParamKind::Text,
        "Alias of a connected Slack workspace (acme for slack@acme).",
    )
    .discoverable()
}

/// One pick-list row per connected workspace.
pub fn workspace_rows(ctx: &TemplateCtx) -> Vec<DiscoveryRow> {
    ctx.clients()
        .slack_workspaces()
        .into_iter()
        .map(|ws| DiscoveryRow {
            label: format!("slack@{ws}"),
            hint: None,
            params: json!({ "workspace": ws }),
        })
        .collect()
}
