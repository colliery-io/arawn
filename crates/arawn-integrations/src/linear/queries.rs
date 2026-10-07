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
