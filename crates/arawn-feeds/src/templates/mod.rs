//! Concrete `FeedTemplate` impls organized per provider.

pub mod calendar;
pub mod confluence;
pub mod drive;
pub mod github;
pub mod gmail;
pub mod jira;
pub mod slack;
pub mod stub;

use std::sync::Arc;

use crate::registry::FeedTemplateRegistry;

/// Build the registry of every template the binary supports. Wire all
/// new templates here. Order doesn't matter — registry is keyed by
/// template name.
pub fn default_registry() -> FeedTemplateRegistry {
    let mut r = FeedTemplateRegistry::new();
    r.register(Arc::new(stub::EchoTemplate));
    r.register(Arc::new(slack::ChannelArchiveTemplate));
    r.register(Arc::new(slack::DmArchiveTemplate));
    r.register(Arc::new(slack::MyMentionsTemplate));
    r.register(Arc::new(calendar::UpcomingArchiveTemplate));
    r.register(Arc::new(gmail::InboxArchiveTemplate));
    r.register(Arc::new(gmail::SenderFilterTemplate));
    r.register(Arc::new(gmail::LabelArchiveTemplate));
    r.register(Arc::new(drive::FolderSyncTemplate));
    r.register(Arc::new(drive::RecentTemplate));
    r.register(Arc::new(confluence::SpaceArchiveTemplate));
    r.register(Arc::new(jira::ProjectTrackerTemplate));
    r.register(Arc::new(jira::AssigneeTrackerTemplate));
    r.register(Arc::new(github::NotificationsTemplate));
    r.register(Arc::new(github::IssuesAndPrsTemplate));
    r.register(Arc::new(github::ReviewQueueTemplate));
    r.register(Arc::new(github::RepoMirrorTemplate));
    r.register(Arc::new(crate::clients::filesystem::FilesystemFeedTemplate));
    r
}

/// One-line, human-readable blurb for a template name — shown in the
/// `/watch` modal's stage-1 picker. Centralized here (rather than a trait
/// method) so the descriptions live in one place; `""` for an unknown name.
pub fn template_blurb(name: &str) -> &'static str {
    match name {
        "stub/echo" => "Test feed — echoes a message to the run log",
        "slack/channel-archive" => "Archive a Slack channel's messages",
        "slack/dm-archive" => "Archive a Slack DM thread",
        "slack/my-mentions" => "Slack messages that mention you",
        "calendar/upcoming-archive" => "Upcoming Google Calendar events",
        "gmail/inbox-archive" => "Recent Gmail inbox messages",
        "gmail/sender-filter" => "Gmail messages from a sender pattern",
        "gmail/label-archive" => "Gmail messages under a label",
        "drive/folder-sync" => "Mirror a Google Drive folder",
        "drive/recent" => "Recently changed Google Drive files",
        "confluence/space-archive" => "Pages in a Confluence space",
        "jira/project-tracker" => "Issues in a Jira project",
        "jira/assignee-tracker" => "Jira issues assigned to you",
        "github/notifications" => "Your GitHub notifications",
        "github/issues-and-prs" => "GitHub issues and PRs you're involved in",
        "github/review-queue" => "GitHub PRs awaiting your review",
        "github/repo-mirror" => "Mirror a GitHub repo's activity",
        "filesystem/folder" => "Watch a local folder for text-file changes",
        _ => "",
    }
}

/// The full picker catalog: `(name, blurb)` for every registered template,
/// sorted by name. Backs the `feed_templates` RPC.
pub fn template_catalog() -> Vec<(&'static str, &'static str)> {
    let reg = default_registry();
    let mut names: Vec<&'static str> = reg.names().collect();
    names.sort_unstable();
    names.into_iter().map(|n| (n, template_blurb(n))).collect()
}

#[cfg(test)]
mod param_schema_tests {
    use super::*;
    use crate::param_schema::ParamKind;
    use std::collections::HashSet;

    /// Every registered template's `param_schema()` must be self-consistent:
    /// unique keys, each `default` type-matches its `kind`, and required params
    /// carry no default. Empty schemas are valid (param-less feeds).
    #[test]
    fn every_template_schema_is_self_consistent() {
        let reg = default_registry();
        let names: Vec<&'static str> = reg.names().collect();
        assert!(!names.is_empty(), "registry should not be empty");

        for name in names {
            let tpl = reg.get(name).expect("registered template resolves");
            let schema = tpl.param_schema();

            let mut seen = HashSet::new();
            for spec in &schema {
                assert!(
                    seen.insert(spec.key.clone()),
                    "{name}: duplicate param key '{}'",
                    spec.key
                );
                assert!(
                    spec.default_matches_kind(),
                    "{name}: param '{}' default {:?} doesn't match kind {:?}",
                    spec.key,
                    spec.default,
                    spec.kind
                );
                if spec.required {
                    assert!(
                        spec.default.is_none(),
                        "{name}: required param '{}' must not carry a default",
                        spec.key
                    );
                }
                assert!(
                    !spec.label.trim().is_empty(),
                    "{name}: param '{}' has an empty label",
                    spec.key
                );
            }
        }
    }

    /// The filesystem template's schema must describe exactly the params its
    /// `FilesystemFeedParams` struct accepts — guards against schema↔validate
    /// drift for the multi-param template that motivated this work.
    #[test]
    fn filesystem_schema_matches_its_params() {
        let reg = default_registry();
        let tpl = reg.get("filesystem/folder").expect("filesystem registered");
        let schema = tpl.param_schema();
        let keys: HashSet<&str> = schema.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(
            keys,
            HashSet::from(["root", "recursive", "include", "exclude", "copy_files"]),
            "filesystem schema keys drifted from FilesystemFeedParams"
        );
        let root = schema.iter().find(|s| s.key == "root").unwrap();
        assert!(root.required, "root must be required");
        assert!(matches!(root.kind, ParamKind::Path));
        let recursive = schema.iter().find(|s| s.key == "recursive").unwrap();
        assert_eq!(recursive.default, Some(serde_json::json!(true)));
    }

    /// Every registered template must have a non-empty picker blurb, and the
    /// catalog must cover exactly the registry.
    #[test]
    fn template_catalog_covers_registry_with_blurbs() {
        let reg = default_registry();
        let reg_names: std::collections::HashSet<&str> = reg.names().collect();
        let catalog = super::template_catalog();
        let cat_names: std::collections::HashSet<&str> = catalog.iter().map(|(n, _)| *n).collect();
        assert_eq!(cat_names, reg_names, "catalog must match the registry");
        for (name, blurb) in &catalog {
            assert!(!blurb.is_empty(), "{name} has no picker blurb");
        }
    }

    /// Spot-check that genuinely param-less feeds return empty schemas (so the
    /// "empty is valid" contract is actually exercised).
    #[test]
    fn paramless_feeds_have_empty_schema() {
        let reg = default_registry();
        for name in [
            "slack/my-mentions",
            "github/issues-and-prs",
            "github/review-queue",
        ] {
            let tpl = reg.get(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(
                tpl.param_schema().is_empty(),
                "{name} should have an empty schema"
            );
        }
    }
}
