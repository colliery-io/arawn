//! The GraphQL documents arawn sends to Linear. One named operation per
//! const, so a change in Linear's schema shows up as a diff here
//! (NFR-002) and in the request bodies the tests check.

/// The key's user and workspace. `arawn setup linear` validates a key
/// with it before saving.
pub const VIEWER: &str = "query Viewer { viewer { id name email organization { name } } }";

/// The issue fields every issue read asks for. `comments(last: 10)` is
/// the ten newest comments.
pub const ISSUE_FIELDS: &str = "fragment IssueFields on Issue { \
    id identifier title description url priority priorityLabel dueDate createdAt updatedAt \
    state { id name type } \
    team { id key name } \
    assignee { id name email } \
    creator { id name email } \
    labels { nodes { name } } \
    comments(last: 10) { nodes { id body createdAt user { id name } } } \
}";

/// One page of the issues assigned to or created by the key's user.
/// `$filter` is built by [`super::my_issues_filter`] (the `updatedAt`
/// bound is left out on a first run).
pub const MY_ISSUES: &str = "query MyIssues($filter: IssueFilter, $after: String) { \
    viewer { id } \
    issues(filter: $filter, first: 50, after: $after, orderBy: updatedAt) { \
        nodes { ...IssueFields } \
        pageInfo { hasNextPage endCursor } \
    } \
}";

/// One issue with its fields. `$id` is the identifier (`ENG-123`) or id.
pub const ISSUE: &str = "query Issue($id: String!) { issue(id: $id) { ...IssueFields } }";

/// The issue's id, identifier, URL and team: what a write needs first.
pub const ISSUE_REF: &str =
    "query IssueRef($id: String!) { issue(id: $id) { id identifier url team { id key } } }";

/// The workflow states of the issue's team, to resolve a state name.
pub const ISSUE_TEAM_STATES: &str = "query IssueTeamStates($id: String!) { issue(id: $id) { \
    id identifier url team { id key states(first: 100) { nodes { id name type } } } } }";

/// The members of the issue's team, to resolve an assignee.
pub const ISSUE_TEAM_MEMBERS: &str = "query IssueTeamMembers($id: String!) { issue(id: $id) { \
    id identifier url team { id key members(first: 250) { nodes { id name displayName email } } } } }";

/// Add a comment. `$issueId` is the issue's id.
pub const COMMENT_CREATE: &str = "mutation CommentCreate($issueId: String!, $body: String!) { \
    commentCreate(input: { issueId: $issueId, body: $body }) { success comment { id url } } }";

/// Change an issue. `$input` carries one of `stateId`, `assigneeId`,
/// `priority`.
pub const ISSUE_UPDATE: &str = "mutation IssueUpdate($id: String!, $input: IssueUpdateInput!) { \
    issueUpdate(id: $id, input: $input) { success issue { \
        id identifier url priority priorityLabel state { name } assignee { name } } } }";
