//! Doc comments (COLLIERY-I-0533): the comment model arawn keeps, the
//! Drive calls behind it, and the "for me" filter.
//!
//! Comments live on Drive, not on the Docs API:
//! `GET /drive/v3/files/{fileId}/comments` (`fields` required, 100 per
//! page, `startModifiedTime`, `includeDeleted`) and
//! `POST /files/{fileId}/comments/{commentId}/replies`. Both work with
//! the `drive` scope arawn already asks for. The comment `author` has no
//! email; `author.me` says whether it is the user.
//!
//! The calls sit behind [`CommentsApi`] so the feed, the tools and their
//! tests share one seam: [`HubCommentsApi`] in production, a fake in tests.

use std::sync::Arc;

use async_trait::async_trait;
use google_drive3::api::{Comment, CommentList, Reply};
use serde::{Deserialize, Serialize};

use crate::error::IntegrationError;

use super::integration::GoogleDriveIntegration;

/// The comment fields arawn reads. `fields` is required by the API.
pub const COMMENT_LIST_FIELDS: &str = "nextPageToken,comments(id,content,createdTime,\
     modifiedTime,resolved,deleted,mentionedEmailAddresses,assigneeEmailAddress,\
     author(displayName,me),quotedFileContent(value),replies(id,content,createdTime,\
     deleted,action,mentionedEmailAddresses,author(displayName,me)))";

/// The reply fields `drive_reply_to_comment` gives back.
pub const REPLY_FIELDS: &str = "id,content,createdTime";

/// The file fields a comment snapshot needs.
pub const DOC_FIELDS: &str = "id,name,mimeType,webViewLink,ownedByMe";

/// Comments per page; the API maximum.
pub const COMMENT_PAGE_SIZE: i32 = 100;

/// A Doc, Sheet or Slides file as the comment feed sees it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocMeta {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub web_view_link: Option<String>,
    pub owned_by_me: bool,
}

/// One reply in a comment thread.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocReply {
    pub id: String,
    pub content: String,
    pub author: Option<String>,
    pub author_is_me: bool,
    pub created_time: Option<String>,
    pub deleted: bool,
    pub mentioned_emails: Vec<String>,
}

/// One comment thread: the comment and its replies, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocComment {
    pub id: String,
    pub content: String,
    pub author: Option<String>,
    pub author_is_me: bool,
    pub created_time: Option<String>,
    /// The comment or any of its replies; RFC3339.
    pub modified_time: Option<String>,
    pub resolved: bool,
    pub deleted: bool,
    pub mentioned_emails: Vec<String>,
    pub assignee_email: Option<String>,
    /// The document text the comment is anchored to.
    pub quoted: Option<String>,
    pub replies: Vec<DocReply>,
}

impl DocComment {
    /// The replies that are not deleted.
    pub fn live_replies(&self) -> impl Iterator<Item = &DocReply> {
        self.replies.iter().filter(|r| !r.deleted)
    }

    /// The last reply that is not deleted.
    pub fn last_reply(&self) -> Option<&DocReply> {
        self.live_replies().last()
    }
}

/// Why a comment thread is for the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForMe {
    /// Assigned to the user (an action item).
    Assigned,
    /// The user's email is mentioned in the comment or a reply.
    Mentioned,
    /// On a doc the user owns, and the last message is not the user's.
    OwnerUnanswered,
}

impl ForMe {
    pub fn as_str(self) -> &'static str {
        match self {
            ForMe::Assigned => "assigned",
            ForMe::Mentioned => "mentioned",
            ForMe::OwnerUnanswered => "owner_unanswered",
        }
    }
}

/// The for-me filter. `None` when the thread is resolved or deleted, or
/// when none of the reasons holds. `me` is the user's email.
pub fn for_me(c: &DocComment, me: &str, owned_by_me: bool) -> Option<ForMe> {
    if c.resolved || c.deleted {
        return None;
    }
    let is_me = |e: &String| e.eq_ignore_ascii_case(me);
    if c.assignee_email.as_ref().is_some_and(is_me) {
        return Some(ForMe::Assigned);
    }
    if c.mentioned_emails.iter().any(is_me)
        || c.live_replies()
            .any(|r| r.mentioned_emails.iter().any(is_me))
    {
        return Some(ForMe::Mentioned);
    }
    let last_is_mine = c
        .last_reply()
        .map(|r| r.author_is_me)
        .unwrap_or(c.author_is_me);
    if owned_by_me && !last_is_mine {
        return Some(ForMe::OwnerUnanswered);
    }
    None
}

/// The link that opens the doc on the comment: `webViewLink` plus
/// `disco=<commentId>`.
pub fn comment_link(web_view_link: &str, comment_id: &str) -> String {
    let sep = if web_view_link.contains('?') {
        '&'
    } else {
        '?'
    };
    format!("{web_view_link}{sep}disco={comment_id}")
}

fn rfc3339(t: Option<chrono::DateTime<chrono::Utc>>) -> Option<String> {
    t.map(|t| t.to_rfc3339())
}

fn from_reply(r: Reply) -> DocReply {
    let author = r.author.unwrap_or_default();
    DocReply {
        id: r.id.unwrap_or_default(),
        content: r.content.unwrap_or_default(),
        author: author.display_name,
        author_is_me: author.me.unwrap_or(false),
        created_time: rfc3339(r.created_time),
        deleted: r.deleted.unwrap_or(false),
        mentioned_emails: r.mentioned_email_addresses.unwrap_or_default(),
    }
}

/// Google's `Comment` to the model arawn keeps.
pub fn from_api(c: Comment) -> DocComment {
    let author = c.author.unwrap_or_default();
    DocComment {
        id: c.id.unwrap_or_default(),
        content: c.content.unwrap_or_default(),
        author: author.display_name,
        author_is_me: author.me.unwrap_or(false),
        created_time: rfc3339(c.created_time),
        modified_time: rfc3339(c.modified_time),
        resolved: c.resolved.unwrap_or(false),
        deleted: c.deleted.unwrap_or(false),
        mentioned_emails: c.mentioned_email_addresses.unwrap_or_default(),
        assignee_email: c.assignee_email_address,
        quoted: c.quoted_file_content.and_then(|q| q.value),
        replies: c
            .replies
            .unwrap_or_default()
            .into_iter()
            .map(from_reply)
            .collect(),
    }
}

/// The Drive calls behind doc comments. [`HubCommentsApi`] in
/// production; tests fake it and inspect what was sent.
#[async_trait]
pub trait CommentsApi: Send + Sync {
    /// The user's email (`about.get`, `fields=user(emailAddress)`).
    async fn about_me(&self) -> Result<String, IntegrationError>;

    /// `files.get` with [`DOC_FIELDS`].
    async fn doc_meta(&self, file_id: &str) -> Result<DocMeta, IntegrationError>;

    /// One page of `comments.list` with [`COMMENT_LIST_FIELDS`],
    /// [`COMMENT_PAGE_SIZE`] and `includeDeleted=true`.
    async fn list_comments_page(
        &self,
        file_id: &str,
        start_modified: Option<&str>,
        page_token: Option<&str>,
    ) -> Result<CommentList, IntegrationError>;

    /// `replies.create` with `request` as the body and `fields`.
    async fn create_reply(
        &self,
        file_id: &str,
        comment_id: &str,
        request: Reply,
        fields: &str,
    ) -> Result<Reply, IntegrationError>;
}

/// Every comment on `file_id` modified at or after `start_modified`
/// (all of them when `None`), deleted ones included, following every page.
pub async fn list_comments(
    api: &dyn CommentsApi,
    file_id: &str,
    start_modified: Option<&str>,
) -> Result<Vec<DocComment>, IntegrationError> {
    let mut out = Vec::new();
    let mut token: Option<String> = None;
    loop {
        let page = api
            .list_comments_page(file_id, start_modified, token.as_deref())
            .await?;
        out.extend(page.comments.unwrap_or_default().into_iter().map(from_api));
        match page.next_page_token {
            Some(t) if !t.is_empty() => token = Some(t),
            _ => break,
        }
    }
    Ok(out)
}

/// [`CommentsApi`] over the `google_drive3` hub of the Drive integration.
pub struct HubCommentsApi {
    integration: Arc<GoogleDriveIntegration>,
}

impl HubCommentsApi {
    pub fn new(integration: Arc<GoogleDriveIntegration>) -> Self {
        Self { integration }
    }
}

fn google_err(op: &str, e: google_drive3::Error) -> IntegrationError {
    IntegrationError::Provider(format!("{op}: {e}"))
}

#[async_trait]
impl CommentsApi for HubCommentsApi {
    async fn about_me(&self) -> Result<String, IntegrationError> {
        let hub = self.integration.hub()?;
        let (_, about) = hub
            .about()
            .get()
            .param("fields", "user(emailAddress)")
            .doit()
            .await
            .map_err(|e| google_err("about.get", e))?;
        about
            .user
            .and_then(|u| u.email_address)
            .ok_or_else(|| IntegrationError::Provider("about.get: no user email".into()))
    }

    async fn doc_meta(&self, file_id: &str) -> Result<DocMeta, IntegrationError> {
        let hub = self.integration.hub()?;
        let (_, f) = hub
            .files()
            .get(file_id)
            .param("fields", DOC_FIELDS)
            .doit()
            .await
            .map_err(|e| google_err("files.get", e))?;
        Ok(DocMeta {
            id: f.id.unwrap_or_default(),
            name: f.name.unwrap_or_default(),
            mime_type: f.mime_type.unwrap_or_default(),
            web_view_link: f.web_view_link,
            owned_by_me: f.owned_by_me.unwrap_or(false),
        })
    }

    async fn list_comments_page(
        &self,
        file_id: &str,
        start_modified: Option<&str>,
        page_token: Option<&str>,
    ) -> Result<CommentList, IntegrationError> {
        let hub = self.integration.hub()?;
        let mut req = hub
            .comments()
            .list(file_id)
            .page_size(COMMENT_PAGE_SIZE)
            .include_deleted(true)
            .param("fields", COMMENT_LIST_FIELDS);
        if let Some(s) = start_modified {
            req = req.start_modified_time(s);
        }
        if let Some(t) = page_token {
            req = req.page_token(t);
        }
        let (_, list) = req
            .doit()
            .await
            .map_err(|e| google_err("comments.list", e))?;
        Ok(list)
    }

    async fn create_reply(
        &self,
        file_id: &str,
        comment_id: &str,
        request: Reply,
        fields: &str,
    ) -> Result<Reply, IntegrationError> {
        let hub = self.integration.hub()?;
        let (_, reply) = hub
            .replies()
            .create(request, file_id, comment_id)
            .param("fields", fields)
            .doit()
            .await
            .map_err(|e| google_err("replies.create", e))?;
        Ok(reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(author_is_me: bool) -> DocReply {
        DocReply {
            id: "r".into(),
            content: "ok".into(),
            author_is_me,
            ..Default::default()
        }
    }

    fn comment() -> DocComment {
        DocComment {
            id: "c1".into(),
            content: "Can you check the numbers?".into(),
            author: Some("Robin".into()),
            ..Default::default()
        }
    }

    const ME: &str = "me@example.com";

    #[test]
    fn mentioned_assigned_and_owner_unanswered_are_for_me() {
        let mut c = comment();
        c.mentioned_emails = vec!["ME@example.com".into()];
        assert_eq!(for_me(&c, ME, false), Some(ForMe::Mentioned));

        let mut c = comment();
        c.assignee_email = Some(ME.into());
        assert_eq!(for_me(&c, ME, false), Some(ForMe::Assigned));

        let mut c = comment();
        c.replies = vec![DocReply {
            mentioned_emails: vec![ME.into()],
            ..reply(false)
        }];
        assert_eq!(for_me(&c, ME, false), Some(ForMe::Mentioned));

        assert_eq!(for_me(&comment(), ME, true), Some(ForMe::OwnerUnanswered));
        assert_eq!(for_me(&comment(), ME, false), None);
    }

    #[test]
    fn answered_resolved_and_deleted_are_not_for_me() {
        let mut c = comment();
        c.replies = vec![reply(true)];
        assert_eq!(for_me(&c, ME, true), None, "my reply is the last");

        // A deleted reply does not count as the last message.
        c.replies.push(DocReply {
            deleted: true,
            ..reply(false)
        });
        assert_eq!(for_me(&c, ME, true), None);

        let mut c = comment();
        c.author_is_me = true;
        assert_eq!(for_me(&c, ME, true), None, "my own comment, no reply");

        let mut c = comment();
        c.assignee_email = Some(ME.into());
        c.resolved = true;
        assert_eq!(for_me(&c, ME, true), None);
        c.resolved = false;
        c.deleted = true;
        assert_eq!(for_me(&c, ME, true), None);
    }

    #[test]
    fn link_adds_disco() {
        assert_eq!(
            comment_link("https://docs.google.com/document/d/F/edit", "AAA"),
            "https://docs.google.com/document/d/F/edit?disco=AAA"
        );
        assert_eq!(
            comment_link(
                "https://docs.google.com/document/d/F/edit?usp=drivesdk",
                "AAA"
            ),
            "https://docs.google.com/document/d/F/edit?usp=drivesdk&disco=AAA"
        );
    }

    #[test]
    fn api_comment_maps_to_the_model() {
        let c: Comment = serde_json::from_value(serde_json::json!({
            "id": "c1", "content": "hi", "resolved": false,
            "modifiedTime": "2026-10-05T10:00:00Z",
            "mentionedEmailAddresses": ["me@example.com"],
            "assigneeEmailAddress": "me@example.com",
            "author": {"displayName": "Robin", "me": false},
            "quotedFileContent": {"value": "Q3 total"},
            "replies": [{"id": "r1", "content": "done", "deleted": false,
                         "author": {"displayName": "Me", "me": true}}]
        }))
        .unwrap();
        let m = from_api(c);
        assert_eq!(m.author.as_deref(), Some("Robin"));
        assert!(!m.author_is_me);
        assert_eq!(m.quoted.as_deref(), Some("Q3 total"));
        assert_eq!(
            m.modified_time.as_deref(),
            Some("2026-10-05T10:00:00+00:00")
        );
        assert_eq!(m.replies.len(), 1);
        assert!(m.replies[0].author_is_me);
    }
}
