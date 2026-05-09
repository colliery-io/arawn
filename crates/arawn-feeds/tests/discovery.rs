//! Discovery picker tests: each pickable template returns
//! `Some(rows)` with provider-shaped values; non-pickable templates
//! return `None`.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use arawn_feeds::{
    AtlassianFeedClient, CalendarFeedClient, ConfluencePageBody, ConfluencePageMeta,
    ConfluenceSpaceMeta, DriveFeedClient, FeedClients, FeedError, FeedTemplate, GmailFeedClient,
    JiraIssueDetail, JiraIssueMeta, JiraProjectMeta, SlackAuthInfo, SlackChannel, SlackFeedClient,
    SlackHistoryPage, TemplateCtx,
};
use arawn_feeds::templates::confluence::SpaceArchiveTemplate;
use arawn_feeds::templates::jira::ProjectTrackerTemplate;
use arawn_feeds::templates::slack::ChannelArchiveTemplate;

#[derive(Default)]
struct StubClients {
    slack_channels: Vec<SlackChannel>,
    jira_projects: Vec<JiraProjectMeta>,
    confluence_spaces: Vec<ConfluenceSpaceMeta>,
}

struct StubSlack(Vec<SlackChannel>);

#[async_trait]
impl SlackFeedClient for StubSlack {
    async fn resolve_channel(&self, _: &str) -> Result<String, FeedError> {
        unreachable!()
    }
    async fn channel_history(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        unreachable!()
    }
    async fn thread_replies(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        unreachable!()
    }
    async fn open_dm(&self, _: &str) -> Result<String, FeedError> {
        unreachable!()
    }
    async fn auth_test(&self) -> Result<SlackAuthInfo, FeedError> {
        unreachable!()
    }
    async fn search_messages(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> Result<SlackHistoryPage, FeedError> {
        unreachable!()
    }
    async fn list_channels(&self) -> Result<Vec<SlackChannel>, FeedError> {
        Ok(self.0.clone())
    }
}

struct StubAtlassian {
    projects: Vec<JiraProjectMeta>,
    spaces: Vec<ConfluenceSpaceMeta>,
}

#[async_trait]
impl AtlassianFeedClient for StubAtlassian {
    async fn space_pages_modified_since(
        &self,
        _: &str,
        _: Option<DateTime<Utc>>,
    ) -> Result<Vec<ConfluencePageMeta>, FeedError> {
        unreachable!()
    }
    async fn page_body_storage(&self, _: &str) -> Result<ConfluencePageBody, FeedError> {
        unreachable!()
    }
    async fn jql_search(&self, _: &str, _: u32) -> Result<Vec<JiraIssueMeta>, FeedError> {
        unreachable!()
    }
    async fn issue_full(
        &self,
        _: &str,
        _: bool,
        _: bool,
    ) -> Result<JiraIssueDetail, FeedError> {
        unreachable!()
    }
    async fn resolve_project(&self, _: &str) -> Result<String, FeedError> {
        unreachable!()
    }
    async fn list_jira_projects(&self) -> Result<Vec<JiraProjectMeta>, FeedError> {
        Ok(self.projects.clone())
    }
    async fn list_confluence_spaces(
        &self,
    ) -> Result<Vec<ConfluenceSpaceMeta>, FeedError> {
        Ok(self.spaces.clone())
    }
}

impl FeedClients for StubClients {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
        if self.slack_channels.is_empty() {
            None
        } else {
            Some(Arc::new(StubSlack(self.slack_channels.clone())))
        }
    }
    fn calendar(&self) -> Option<Arc<dyn CalendarFeedClient>> {
        None
    }
    fn gmail(&self) -> Option<Arc<dyn GmailFeedClient>> {
        None
    }
    fn drive(&self) -> Option<Arc<dyn DriveFeedClient>> {
        None
    }
    fn atlassian(&self) -> Option<Arc<dyn AtlassianFeedClient>> {
        if self.jira_projects.is_empty() && self.confluence_spaces.is_empty() {
            None
        } else {
            Some(Arc::new(StubAtlassian {
                projects: self.jira_projects.clone(),
                spaces: self.confluence_spaces.clone(),
            }))
        }
    }
}

#[tokio::test]
async fn slack_channel_archive_discovers_channels() {
    let clients = Arc::new(StubClients {
        slack_channels: vec![
            SlackChannel {
                id: "C0DESIGN".into(),
                name: "design".into(),
                is_private: false,
                is_dm: false,
            },
            SlackChannel {
                id: "C1ENG".into(),
                name: "engineering".into(),
                is_private: false,
                is_dm: false,
            },
            SlackChannel {
                id: "G1PRIV".into(),
                name: "leads-only".into(),
                is_private: true,
                is_dm: false,
            },
        ],
        ..Default::default()
    });
    let ctx = TemplateCtx::new(clients);
    let rows = ChannelArchiveTemplate
        .discover(&ctx)
        .await
        .unwrap()
        .expect("picker_supported");
    assert_eq!(rows.len(), 3);
    // Sorted by name alphabetically.
    assert_eq!(rows[0].label, "#design");
    assert_eq!(rows[0].params["channel"], "C0DESIGN");
    assert_eq!(rows[1].label, "#engineering");
    assert_eq!(rows[2].label, "#leads-only");
    assert!(rows[2].hint.as_deref().unwrap().contains("private"));
}

#[tokio::test]
async fn jira_project_tracker_discovers_projects() {
    let clients = Arc::new(StubClients {
        jira_projects: vec![
            JiraProjectMeta {
                id: "10001".into(),
                key: "ENG".into(),
                name: "Engineering".into(),
            },
            JiraProjectMeta {
                id: "10002".into(),
                key: "DESIGN".into(),
                name: "Design".into(),
            },
        ],
        ..Default::default()
    });
    let ctx = TemplateCtx::new(clients);
    let rows = ProjectTrackerTemplate
        .discover(&ctx)
        .await
        .unwrap()
        .expect("picker_supported");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].label, "DESIGN  —  Design");
    assert_eq!(rows[0].params["project"], "DESIGN");
    assert_eq!(rows[1].params["project"], "ENG");
}

#[tokio::test]
async fn confluence_space_archive_discovers_spaces() {
    let clients = Arc::new(StubClients {
        confluence_spaces: vec![
            ConfluenceSpaceMeta {
                key: "ENG".into(),
                name: "Engineering".into(),
            },
            ConfluenceSpaceMeta {
                key: "DOCS".into(),
                name: "".into(),
            },
        ],
        ..Default::default()
    });
    let ctx = TemplateCtx::new(clients);
    let rows = SpaceArchiveTemplate
        .discover(&ctx)
        .await
        .unwrap()
        .expect("picker_supported");
    assert_eq!(rows.len(), 2);
    // Empty name → label falls back to bare key.
    assert_eq!(rows[0].label, "DOCS");
    assert_eq!(rows[0].params["space_key"], "DOCS");
    assert_eq!(rows[1].label, "ENG  —  Engineering");
}

#[tokio::test]
async fn discover_returns_none_when_provider_missing() {
    // Stub bundle has nothing — slack().is_none(), atlassian().is_none().
    // Templates short-circuit to Ok(None) rather than returning an
    // empty list, so the picker can render a "not connected" message
    // instead of an empty modal.
    let clients = Arc::new(StubClients::default());
    let ctx = TemplateCtx::new(clients);
    assert!(ChannelArchiveTemplate.discover(&ctx).await.unwrap().is_none());
    assert!(ProjectTrackerTemplate.discover(&ctx).await.unwrap().is_none());
    assert!(SpaceArchiveTemplate.discover(&ctx).await.unwrap().is_none());
}

#[tokio::test]
async fn non_pickable_template_returns_none() {
    use arawn_feeds::templates::gmail::SenderFilterTemplate;
    let clients = Arc::new(StubClients::default());
    let ctx = TemplateCtx::new(clients);
    let res = SenderFilterTemplate.discover(&ctx).await.unwrap();
    assert!(
        res.is_none(),
        "free-form param templates default to None — TUI shows usage"
    );
    // Suppress unused warning on Value (used elsewhere in the file).
    let _: Value = serde_json::Value::Null;
}
