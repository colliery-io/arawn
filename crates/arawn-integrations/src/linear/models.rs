//! Typed views of the Linear objects arawn reads. Field names follow
//! the GraphQL schema (camelCase on the wire); only what
//! [`super::queries::ISSUE_FIELDS`] asks for.

use serde::{Deserialize, Deserializer, Serialize};

/// `{ nodes: [...] }` connection wrapper.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Nodes<T> {
    #[serde(default = "Vec::new")]
    pub nodes: Vec<T>,
}

impl<T> Default for Nodes<T> {
    fn default() -> Self {
        Self { nodes: Vec::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowState {
    pub id: String,
    pub name: String,
    /// `triage | backlog | unstarted | started | completed | canceled`.
    #[serde(rename = "type")]
    pub kind: String,
}

impl WorkflowState {
    /// Done or canceled: no longer on anyone's plate.
    pub fn is_closed(&self) -> bool {
        matches!(self.kind.as_str(), "completed" | "canceled")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Team {
    pub id: String,
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Label {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: String,
    pub body: String,
    pub created_at: String,
    /// `None` for comments by integrations.
    #[serde(default)]
    pub user: Option<CommentUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommentUser {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub id: String,
    /// Team key + number, e.g. `ENG-123`.
    pub identifier: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub url: String,
    /// 0 none, 1 urgent, 2 high, 3 medium, 4 low. A `Float` in the
    /// schema; always a whole number.
    #[serde(deserialize_with = "whole_number")]
    pub priority: u8,
    pub priority_label: String,
    /// `YYYY-MM-DD`.
    #[serde(default)]
    pub due_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub state: WorkflowState,
    pub team: Team,
    #[serde(default)]
    pub assignee: Option<User>,
    #[serde(default)]
    pub creator: Option<User>,
    #[serde(default)]
    pub labels: Nodes<Label>,
    #[serde(default)]
    pub comments: Nodes<Comment>,
}

fn whole_number<'de, D: Deserializer<'de>>(d: D) -> Result<u8, D::Error> {
    let f = f64::deserialize(d)?;
    Ok(f.clamp(0.0, 4.0) as u8)
}

/// One page of issues.
#[derive(Debug, Clone, Default)]
pub struct IssuePage {
    pub issues: Vec<Issue>,
    /// The cursor of the next page, when there is one.
    pub next: Option<String>,
}
