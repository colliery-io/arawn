//! `cadence_check` — "who am I overdue with?"
//!
//! ARAWN-I-0065 T-B. Joins `PersonProfile.relation_to_user` (the I-0064
//! org substrate) against the `calendar_events` projection to surface
//! direct reports / managers / peers the user hasn't met with in a while.
//! Sorted oldest-first; people with no recorded meeting bubble to the top.
//!
//! Attendee matching is best-effort substring against the person's Entity
//! title — "Sarah Lee" matches an attendee like `sarah.lee@company.com`
//! or any attendee string containing "sarah" or "lee". Calendar invites
//! that don't carry a recognisable attendee surface as "no recorded
//! meeting." Refining this is a known follow-up once we have a
//! `Person.email` field or a proper alias table.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde_json::{Value, json};

use arawn_memory::{Entity, EntityType, RelationToUser};
use arawn_projections::ProjectionStore;

use crate::lens_router::MemoryHandle;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

pub struct CadenceCheckTool {
    memory: MemoryHandle,
    projections: Arc<ProjectionStore>,
}

impl CadenceCheckTool {
    pub fn new(memory: impl Into<MemoryHandle>, projections: Arc<ProjectionStore>) -> Self {
        Self {
            memory: memory.into(),
            projections,
        }
    }
}

#[async_trait]
impl Tool for CadenceCheckTool {
    fn name(&self) -> &str {
        "cadence_check"
    }

    fn description(&self) -> &str {
        "Who am I overdue with? Pulls direct reports / managers / peers \
         from the org model (PersonProfile.relation_to_user) and joins \
         against recent calendar events to compute days-since-last-meeting. \
         Returns a sorted list, oldest first, of people past the overdue \
         threshold. Pass `role` (`directs` / `managers` / `peers` / `all`; \
         default `directs`) and `overdue_days` (default 14)."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Memory
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "role": {
                    "type": "string",
                    "enum": ["directs", "managers", "peers", "all"],
                    "description": "Which org-relation bucket to check (default: directs)"
                },
                "overdue_days": {
                    "type": "integer",
                    "description": "Threshold for overdue (default: 14)"
                }
            }
        })
    }

    async fn execute(
        &self,
        _ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let role = params
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("directs");
        let overdue_days = params
            .get("overdue_days")
            .and_then(|v| v.as_i64())
            .unwrap_or(14);

        let relations_to_check = match role {
            "directs" => vec![RelationToUser::Manages],
            "managers" => vec![RelationToUser::ReportsToUser],
            "peers" => vec![RelationToUser::PeerOfUser],
            "all" => vec![
                RelationToUser::Manages,
                RelationToUser::ReportsToUser,
                RelationToUser::PeerOfUser,
            ],
            other => {
                return Err(ToolError::ExecutionFailed(format!(
                    "unknown role `{other}` — must be one of: directs, managers, peers, all"
                )));
            }
        };

        let manager = self
            .memory
            .manager()
            .map_err(|e| ToolError::ExecutionFailed(format!("memory routing: {e}")))?;
        let store = &manager.global;

        // Gather every (profile, entity) pair we're going to score.
        let mut targets: Vec<(Entity, RelationToUser)> = Vec::new();
        for rel in &relations_to_check {
            let profiles = store
                .list_person_profiles_by_relation_to_user(*rel)
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("list profiles for {rel:?}: {e}"))
                })?;
            for p in profiles {
                if let Some(entity) = store
                    .get_entity(p.entity_id)
                    .map_err(|e| ToolError::ExecutionFailed(format!("read entity: {e}")))?
                {
                    targets.push((entity, *rel));
                }
            }
        }

        if targets.is_empty() {
            return Ok(ToolOutput::success(format!(
                "No {role} found in the org model. Capture some with \
                 `memory_store(entity_type=\"person\", title=\"…\", content=\"… is someone I manage\")`."
            )));
        }

        // Pull the last-90-day calendar events ONCE; index attendees in
        // memory and look up each person against that index.
        let events = recent_calendar_events(&self.projections, 90)?;
        let now = Utc::now();
        let mut latest_per_person: HashMap<uuid::Uuid, (DateTime<Utc>, String)> = HashMap::new();
        for ev in &events {
            for (entity, _) in &targets {
                if person_matches_any_attendee(&entity.title, &ev.attendees)
                    && latest_per_person
                        .get(&entity.id)
                        .map(|(ts, _)| ev.start > *ts)
                        .unwrap_or(true)
                {
                    latest_per_person.insert(entity.id, (ev.start, ev.title.clone()));
                }
            }
        }

        // Score each target.
        #[derive(Debug)]
        struct Row {
            name: String,
            rel: RelationToUser,
            days_since: Option<i64>,
            last_event_title: Option<String>,
        }
        let mut rows: Vec<Row> = targets
            .into_iter()
            .map(|(entity, rel)| {
                let latest = latest_per_person.get(&entity.id);
                let days_since = latest.map(|(ts, _)| (now - *ts).num_days());
                let last_event_title = latest.map(|(_, t)| t.clone());
                Row {
                    name: entity.title,
                    rel,
                    days_since,
                    last_event_title,
                }
            })
            .collect();

        // Filter to overdue (None = "never met" counts as overdue).
        rows.retain(|r| r.days_since.map(|d| d >= overdue_days).unwrap_or(true));

        // Sort: None (never met) first, then by days_since descending.
        rows.sort_by_key(|r| std::cmp::Reverse(r.days_since.unwrap_or(i64::MAX)));

        if rows.is_empty() {
            return Ok(ToolOutput::success(format!(
                "All {role} are within the {overdue_days}-day threshold. 🎉",
            )));
        }

        let mut out = format!(
            "# Cadence — {role}\n{} overdue (> {overdue_days} days)\n\n",
            rows.len()
        );
        for r in &rows {
            let rel_label = match r.rel {
                RelationToUser::Manages => "direct",
                RelationToUser::ReportsToUser => "manager",
                RelationToUser::PeerOfUser => "peer",
            };
            match (r.days_since, &r.last_event_title) {
                (Some(d), Some(t)) => {
                    out.push_str(&format!(
                        "- **{name}** ({rel_label}) — last met {d} days ago (\"{t}\")\n",
                        name = r.name,
                    ));
                }
                _ => {
                    out.push_str(&format!(
                        "- **{name}** ({rel_label}) — no recorded meeting in last 90 days\n",
                        name = r.name,
                    ));
                }
            }
        }
        Ok(ToolOutput::success(out))
    }
}

/// One calendar event (subset of `CalEvent` we need for cadence).
struct CalRow {
    title: String,
    start: DateTime<Utc>,
    attendees: Vec<String>,
}

/// Pull every calendar event in the last `days` days from the
/// projection. Mirrors the read pattern in `ProjectionsCalendarSource`.
fn recent_calendar_events(
    projections: &Arc<ProjectionStore>,
    days: i64,
) -> Result<Vec<CalRow>, ToolError> {
    projections
        .ensure_feed_type("calendar_events")
        .map_err(|e| ToolError::ExecutionFailed(format!("ensure calendar_events: {e}")))?;
    let conn = projections
        .conn()
        .lock()
        .map_err(|_| ToolError::ExecutionFailed("projections mutex poisoned".into()))?;
    let now = Utc::now();
    let cutoff = (now - chrono::Duration::days(days)).to_rfc3339();
    let now_str = now.to_rfc3339();
    let mut stmt = conn
        .prepare(
            "SELECT title, metadata, source_ts \
             FROM calendar_events \
             WHERE source_ts BETWEEN ?1 AND ?2 \
             ORDER BY source_ts DESC",
        )
        .map_err(|e| ToolError::ExecutionFailed(format!("prepare cal query: {e}")))?;
    let mut rows = stmt
        .query(params![cutoff, now_str])
        .map_err(|e| ToolError::ExecutionFailed(format!("query cal: {e}")))?;
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| ToolError::ExecutionFailed(format!("cal row: {e}")))?
    {
        let title: String = row
            .get(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("col title: {e}")))?;
        let metadata_str: String = row
            .get(1)
            .map_err(|e| ToolError::ExecutionFailed(format!("col metadata: {e}")))?;
        let source_ts_str: String = row
            .get(2)
            .map_err(|e| ToolError::ExecutionFailed(format!("col source_ts: {e}")))?;
        let start = DateTime::parse_from_rfc3339(&source_ts_str)
            .map_err(|e| ToolError::ExecutionFailed(format!("parse source_ts: {e}")))?
            .with_timezone(&Utc);
        let metadata: serde_json::Value = serde_json::from_str(&metadata_str)
            .map_err(|e| ToolError::ExecutionFailed(format!("metadata json: {e}")))?;
        let attendees: Vec<String> = metadata
            .get("attendees")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        out.push(CalRow {
            title,
            start,
            attendees,
        });
    }
    Ok(out)
}

/// Token-based fuzzy match: split the person's title into lowercase
/// alphanumeric tokens (≥2 chars), then return true if any attendee
/// string contains any token as a substring. Handles "Sarah Lee" matching
/// `sarah.lee@company.com`, `slee@example.com` (no — only `sarah.lee`
/// matches), or `Sarah` alone. Trade-off accepted: short common names
/// can false-positive across unrelated people. Refined alias matching is
/// a known follow-up.
fn person_matches_any_attendee(person_title: &str, attendees: &[String]) -> bool {
    let tokens = name_tokens(person_title);
    if tokens.is_empty() {
        return false;
    }
    for att in attendees {
        let att_lower = att.to_lowercase();
        if tokens.iter().any(|t| att_lower.contains(t.as_str())) {
            return true;
        }
    }
    false
}

/// Token-split the title: lowercase, split on whitespace, drop tokens
/// shorter than 2 chars. "Sarah Lee" → ["sarah", "lee"].
fn name_tokens(title: &str) -> Vec<String> {
    title
        .split_whitespace()
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_lowercase())
        .collect()
}

#[allow(dead_code)]
fn _unused_imports_silencer(_e: &EntityType) {}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_core::Lens;
    use arawn_memory::{ConfidenceSource, Entity, EntityType, MemoryManager, PersonProfile};
    use arawn_projections::ProjectionStore;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn setup() -> (
        TempDir,
        Arc<MemoryManager>,
        Arc<ProjectionStore>,
        crate::context::EngineToolContext,
    ) {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("lenses/test-ws")).unwrap();
        let mgr = Arc::new(MemoryManager::open(tmp.path(), "test-ws", None).unwrap());
        let projections =
            Arc::new(ProjectionStore::open(&tmp.path().join("projections.db")).unwrap());
        projections.ensure_feed_type("calendar_events").unwrap();
        let ws = Lens::scratch(tmp.path());
        let ctx = crate::context::EngineToolContext::new(&ws, Uuid::new_v4());
        (tmp, mgr, projections, ctx)
    }

    fn make_direct(mgr: &MemoryManager, name: &str) -> Uuid {
        let p = Entity::new(EntityType::Person, name).with_confidence(ConfidenceSource::Stated);
        let id = p.id;
        mgr.global.insert_entity(&p).unwrap();
        mgr.global
            .upsert_person_profile(
                &PersonProfile::new(id).with_relation_to_user(RelationToUser::Manages),
            )
            .unwrap();
        id
    }

    fn insert_event(
        projections: &Arc<ProjectionStore>,
        title: &str,
        days_ago: i64,
        attendees: &[&str],
    ) {
        let ts = (Utc::now() - chrono::Duration::days(days_ago)).to_rfc3339();
        let metadata = json!({
            "attendees": attendees,
            "end_ts": ts,
        })
        .to_string();
        let now = Utc::now().to_rfc3339();
        let conn = projections.conn().lock().unwrap();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO calendar_events \
                 (id, feed_id, source_id, source_ts, title, body_text, \
                  metadata, body_hash, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id, "test-feed", id, ts, title, "", metadata, "", now, now,
            ],
        )
        .unwrap();
    }

    #[tokio::test]
    async fn returns_helpful_message_when_no_directs_exist() {
        let (_tmp, mgr, projections, ctx) = setup();
        let tool = CadenceCheckTool::new(mgr, projections);
        let result = tool.execute(&ctx, json!({})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("No directs found"));
    }

    #[tokio::test]
    async fn flags_direct_with_no_recorded_meeting_as_overdue() {
        let (_tmp, mgr, projections, ctx) = setup();
        make_direct(&mgr, "Marcus");
        let tool = CadenceCheckTool::new(mgr, projections);
        let result = tool.execute(&ctx, json!({})).await.unwrap();
        assert!(result.content.contains("Marcus"));
        assert!(result.content.contains("no recorded meeting"));
    }

    #[tokio::test]
    async fn omits_recent_meeting_from_overdue_list() {
        let (_tmp, mgr, projections, ctx) = setup();
        make_direct(&mgr, "Sarah Lee");
        insert_event(
            &projections,
            "1:1 Sarah / Dylan",
            5,
            &["sarah.lee@company.com", "dylan@company.com"],
        );
        let tool = CadenceCheckTool::new(mgr, projections);
        let result = tool.execute(&ctx, json!({})).await.unwrap();
        // 5 days ago < 14-day threshold → should not appear.
        assert!(
            result.content.contains("within the 14-day threshold"),
            "got:\n{}",
            result.content
        );
    }

    #[tokio::test]
    async fn flags_meeting_older_than_threshold() {
        let (_tmp, mgr, projections, ctx) = setup();
        make_direct(&mgr, "Marcus");
        insert_event(&projections, "Marcus / Dylan 1:1", 30, &["Marcus"]);
        let tool = CadenceCheckTool::new(mgr, projections);
        let result = tool.execute(&ctx, json!({})).await.unwrap();
        assert!(result.content.contains("Marcus"));
        assert!(result.content.contains("30 days ago"));
        assert!(result.content.contains("Marcus / Dylan 1:1"));
    }

    #[tokio::test]
    async fn sorts_oldest_first_with_never_met_at_top() {
        let (_tmp, mgr, projections, ctx) = setup();
        make_direct(&mgr, "Marcus");
        make_direct(&mgr, "Anita");
        let pat_id = make_direct(&mgr, "Pat");
        // Marcus: 20 days ago
        insert_event(&projections, "Marcus 1:1", 20, &["Marcus"]);
        // Anita: 40 days ago
        insert_event(&projections, "Anita 1:1", 40, &["Anita"]);
        // Pat: no meeting

        let tool = CadenceCheckTool::new(mgr, projections);
        let result = tool.execute(&ctx, json!({})).await.unwrap();
        let pos_pat = result.content.find("**Pat**").unwrap_or(usize::MAX);
        let pos_anita = result.content.find("**Anita**").unwrap_or(usize::MAX);
        let pos_marcus = result.content.find("**Marcus**").unwrap_or(usize::MAX);
        assert!(
            pos_pat < pos_anita && pos_anita < pos_marcus,
            "expected Pat (never) before Anita (40d) before Marcus (20d), got:\n{}",
            result.content
        );
        let _ = pat_id;
    }

    #[tokio::test]
    async fn role_managers_only_returns_relation_to_user_reports_to_user() {
        let (_tmp, mgr, projections, ctx) = setup();
        make_direct(&mgr, "Marcus");
        // Add a manager (ReportsToUser).
        let david = Entity::new(EntityType::Person, "David").with_confidence(ConfidenceSource::Stated);
        let did = david.id;
        mgr.global.insert_entity(&david).unwrap();
        mgr.global
            .upsert_person_profile(
                &PersonProfile::new(did)
                    .with_relation_to_user(RelationToUser::ReportsToUser),
            )
            .unwrap();

        let tool = CadenceCheckTool::new(mgr, projections);
        let result = tool
            .execute(&ctx, json!({"role": "managers"}))
            .await
            .unwrap();
        assert!(result.content.contains("David"));
        assert!(!result.content.contains("Marcus"));
    }

    #[tokio::test]
    async fn unknown_role_errors() {
        let (_tmp, mgr, projections, ctx) = setup();
        let tool = CadenceCheckTool::new(mgr, projections);
        let err = tool
            .execute(&ctx, json!({"role": "bogus"}))
            .await
            .unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("unknown role"));
    }

    #[tokio::test]
    async fn overdue_days_threshold_is_respected() {
        let (_tmp, mgr, projections, ctx) = setup();
        make_direct(&mgr, "Marcus");
        insert_event(&projections, "Marcus 1:1", 10, &["Marcus"]);

        let tool = CadenceCheckTool::new(mgr, projections);
        // Default 14: 10 days ago → NOT overdue.
        let result = tool.execute(&ctx, json!({})).await.unwrap();
        assert!(result.content.contains("within"));

        // Lower threshold 7: 10 days ago → overdue.
        let result = tool
            .execute(&ctx, json!({"overdue_days": 7}))
            .await
            .unwrap();
        assert!(result.content.contains("Marcus"));
        assert!(result.content.contains("10 days ago"));
    }

    #[test]
    fn name_tokens_drops_short_tokens() {
        assert_eq!(name_tokens("Sarah Lee"), vec!["sarah", "lee"]);
        assert_eq!(name_tokens("X Y Z"), Vec::<String>::new());
        assert_eq!(name_tokens(""), Vec::<String>::new());
    }

    #[test]
    fn person_matches_email_address() {
        assert!(person_matches_any_attendee(
            "Sarah Lee",
            &["sarah.lee@company.com".into(), "x@y.com".into()],
        ));
    }

    #[test]
    fn person_does_not_match_unrelated_attendee() {
        assert!(!person_matches_any_attendee(
            "Marcus",
            &["sarah.lee@company.com".into(), "dylan@company.com".into()],
        ));
    }
}
