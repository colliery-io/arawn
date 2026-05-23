
/// T-0362: TUI `/usage` slash command — call the server-side
/// `usage.summary` RPC and pretty-print via the shared renderer
/// in `arawn_llm::usage::render_usage_human`. Server-side
/// parameter validation is authoritative — bad `period` values
/// surface as an `error.message` string here.
pub(super) async fn render_usage(client: &mut crate::ws_client::WsClient, period: &str) -> String {
    let resp = match client
        .request_response(
            "usage.summary",
            serde_json::json!({"period": period, "by_site": true}),
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return format!("/usage failed: {e}"),
    };
    let Some(result) = resp.get("result") else {
        let err = resp
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return format!("/usage failed: {err}");
    };
    let summary: arawn_llm::usage::UsageSummary = match serde_json::from_value(result.clone()) {
        Ok(s) => s,
        Err(e) => return format!("/usage: malformed response: {e}"),
    };
    let body = arawn_llm::usage::render_usage_human(&summary);
    format!("```\n{body}```")
}

