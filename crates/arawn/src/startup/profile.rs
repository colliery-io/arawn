//! Setup profiles (ARAWN-T-0507): a ready-made set of `[[lenses]]` and
//! `[[feeds]]` that `arawn init --profile <name>` writes into arawn.toml.
//!
//! A profile is only declarations. The reconciler (ARAWN-T-0506) applies
//! them when the server starts, and each feed waits until its service is
//! connected. Feed ids match the default feeds that `arawn connect`
//! creates (ARAWN-T-0510), so the two never make duplicate feeds.

use anyhow::{Result, bail};
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

use crate::config::{FeedDecl, LensDecl};

/// Profiles that `--profile` accepts.
pub const PROFILES: &[&str] = &["work"];

/// The declarations of a profile.
pub fn profile(name: &str) -> Result<(Vec<LensDecl>, Vec<FeedDecl>)> {
    match name {
        "work" => Ok(work()),
        other => bail!(
            "unknown profile `{other}`. Use one of: {}",
            PROFILES.join(", ")
        ),
    }
}

fn feed(id: &str, template: &str) -> FeedDecl {
    FeedDecl {
        id: id.into(),
        template: template.into(),
        ..Default::default()
    }
}

/// A `work` lens over the standard work sources: mail, calendar, chat,
/// tickets and code review.
fn work() -> (Vec<LensDecl>, Vec<FeedDecl>) {
    let feeds = vec![
        feed("gmail-inbox", "gmail/inbox-archive"),
        feed("calendar-upcoming", "calendar/upcoming-archive"),
        feed("slack-mentions", "slack/my-mentions"),
        feed("jira-assigned", "jira/assignee-tracker"),
        feed("github-review-queue", "github/review-queue"),
        feed("github-notifications", "github/notifications"),
    ];
    let lens = LensDecl {
        name: "work".into(),
        display_name: Some("Work".into()),
        description: "My job: the decisions, commitments, people and projects that come \
                      through work mail, meetings, chat, tickets and code review. Edit this \
                      to describe your role; the extractor reads it."
            .into(),
        tags: [
            "decisions",
            "action-items",
            "commitments",
            "people",
            "projects",
            "deadlines",
            "incidents",
            "reviews",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        feeds: feeds.iter().map(|f| f.id.clone()).collect(),
    };
    (vec![lens], feeds)
}

fn array_of_tables<'a>(doc: &'a mut DocumentMut, key: &str) -> Result<&'a mut ArrayOfTables> {
    if !doc.contains_key(key) {
        doc.insert(key, Item::ArrayOfTables(ArrayOfTables::new()));
    }
    doc[key]
        .as_array_of_tables_mut()
        .ok_or_else(|| anyhow::anyhow!("`{key}` in arawn.toml is not a list of [[{key}]] tables"))
}

fn has_entry(aot: &ArrayOfTables, field: &str, wanted: &str) -> bool {
    aot.iter()
        .any(|t| t.get(field).and_then(|v| v.as_str()) == Some(wanted))
}

/// What [`apply`] added and what it left alone.
#[derive(Debug, Default, PartialEq)]
pub struct Applied {
    pub added_lenses: Vec<String>,
    pub added_feeds: Vec<String>,
    /// Profile feeds appended to the `feeds` list of a lens that was
    /// already declared: `(lens, feed ids)`.
    pub bound_to_kept: Vec<(String, Vec<String>)>,
    /// Names / ids already in arawn.toml; not changed.
    pub kept: Vec<String>,
}

impl Applied {
    /// True when the file was not changed.
    pub fn is_empty(&self) -> bool {
        self.added_lenses.is_empty() && self.added_feeds.is_empty() && self.bound_to_kept.is_empty()
    }
}

/// `params` as an inline TOML table for a `[[feeds]]` entry.
fn inline_params(params: &toml::Table) -> Result<toml_edit::InlineTable> {
    let mut wrapper = toml::Table::new();
    wrapper.insert("params".into(), toml::Value::Table(params.clone()));
    let doc: DocumentMut = toml::to_string(&wrapper)?.parse()?;
    Ok(doc["params"]
        .as_table()
        .cloned()
        .unwrap_or_default()
        .into_inline_table())
}

/// Add a profile's declarations to arawn.toml, in place. Additive: an
/// entry whose lens name or feed id is already declared is left as it is,
/// except that a kept lens gets the profile's missing feed ids appended
/// to its `feeds` list, so a new feed is never left bound to no lens.
pub fn apply(doc: &mut DocumentMut, lenses: &[LensDecl], feeds: &[FeedDecl]) -> Result<Applied> {
    let mut out = Applied::default();

    let aot = array_of_tables(doc, "lenses")?;
    for l in lenses {
        if let Some(t) = aot
            .iter_mut()
            .find(|t| t.get("name").and_then(|v| v.as_str()) == Some(l.name.as_str()))
        {
            out.kept.push(format!("lens {}", l.name));
            if !t.contains_key("feeds") {
                t["feeds"] = value(toml_edit::Array::new());
            }
            let Some(list) = t["feeds"].as_array_mut() else {
                bail!("`feeds` of lens `{}` in arawn.toml is not a list", l.name);
            };
            let mut added = Vec::new();
            for id in &l.feeds {
                if !list.iter().any(|v| v.as_str() == Some(id.as_str())) {
                    list.push(id.as_str());
                    added.push(id.clone());
                }
            }
            if !added.is_empty() {
                out.bound_to_kept.push((l.name.clone(), added));
            }
            continue;
        }
        let mut t = Table::new();
        t["name"] = value(&l.name);
        if let Some(d) = &l.display_name {
            t["display_name"] = value(d);
        }
        t["description"] = value(&l.description);
        t["tags"] = value(l.tags.iter().cloned().collect::<toml_edit::Array>());
        t["feeds"] = value(l.feeds.iter().cloned().collect::<toml_edit::Array>());
        aot.push(t);
        out.added_lenses.push(l.name.clone());
    }

    let aot = array_of_tables(doc, "feeds")?;
    for f in feeds {
        if has_entry(aot, "id", &f.id) {
            out.kept.push(format!("feed {}", f.id));
            continue;
        }
        let mut t = Table::new();
        t["id"] = value(&f.id);
        t["template"] = value(&f.template);
        if !f.params.is_empty() {
            t["params"] = value(inline_params(&f.params)?);
        }
        if let Some(c) = &f.cadence {
            t["cadence"] = value(c);
        }
        aot.push(t);
        out.added_feeds.push(f.id.clone());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_profile_templates_exist_and_accept_their_params() {
        let reg = arawn_feeds::templates::default_registry();
        let (_, feeds) = profile("work").unwrap();
        for f in &feeds {
            let tmpl = reg
                .get(&f.template)
                .unwrap_or_else(|| panic!("template {} is not registered", f.template));
            let params = arawn_feeds::TemplateParams::new(serde_json::to_value(&f.params).unwrap());
            tmpl.validate(&params)
                .unwrap_or_else(|e| panic!("{}: {e}", f.template));
        }
    }

    #[test]
    fn work_profile_binds_every_feed_and_matches_default_feed_ids() {
        let (lenses, feeds) = profile("work").unwrap();
        let ids: Vec<_> = feeds.iter().map(|f| f.id.clone()).collect();
        assert_eq!(lenses[0].feeds, ids);
        assert!(!lenses[0].tags.is_empty());
        // Same ids as the default feeds that `arawn connect` creates, so
        // the declared feed and the default feed are one feed.
        for (id, template) in [
            ("gmail-inbox", "gmail/inbox-archive"),
            ("calendar-upcoming", "calendar/upcoming-archive"),
            ("slack-mentions", "slack/my-mentions"),
            ("jira-assigned", "jira/assignee-tracker"),
        ] {
            assert!(
                feeds.iter().any(|f| f.id == id && f.template == template),
                "{id}"
            );
        }
    }

    #[test]
    fn apply_writes_a_config_that_loads_and_plans_cleanly() {
        let mut doc: DocumentMut = "# mine\n[engine]\nllm = \"default\"\n".parse().unwrap();
        let (l, f) = profile("work").unwrap();
        let applied = apply(&mut doc, &l, &f).unwrap();
        assert_eq!(applied.added_lenses, ["work"]);
        assert_eq!(applied.added_feeds.len(), 6);
        let text = doc.to_string();
        assert!(text.starts_with("# mine\n"));
        let cfg: crate::ArawnConfig = toml::from_str(&text).unwrap();
        assert_eq!(cfg.lenses, l);
        assert_eq!(cfg.feeds, f);
        // The reconciler accepts it: no problems on a fresh store.
        let declared = crate::local_service::declared::Declared {
            lenses: cfg.lenses,
            feeds: cfg.feeds,
        };
        let existing = crate::local_service::declared::Existing {
            lenses: Default::default(),
            feeds: Some(Default::default()),
        };
        let steps = crate::local_service::declared::plan(&declared, &existing, &Default::default());
        assert!(
            !steps
                .iter()
                .any(|s| matches!(s, crate::local_service::declared::Step::Problem(_))),
            "{steps:?}"
        );
    }

    #[test]
    fn apply_is_additive_and_keeps_existing_entries() {
        let mut doc: DocumentMut = r#"
[[lenses]]
name = "work"
description = "mine, edited"
tags = ["custom"]

[[feeds]]
id = "gmail-inbox"
template = "gmail/label-archive"
params = { label = "Work" }
"#
        .parse()
        .unwrap();
        let (l, f) = profile("work").unwrap();
        let applied = apply(&mut doc, &l, &f).unwrap();
        assert!(applied.added_lenses.is_empty());
        assert!(applied.kept.contains(&"lens work".to_string()));
        assert!(applied.kept.contains(&"feed gmail-inbox".to_string()));
        assert_eq!(applied.added_feeds.len(), 5);
        let cfg: crate::ArawnConfig = toml::from_str(&doc.to_string()).unwrap();
        assert_eq!(cfg.lenses[0].description, "mine, edited");
        assert_eq!(cfg.lenses[0].tags, ["custom"]);
        assert_eq!(cfg.feeds[0].template, "gmail/label-archive");
        // Review regression: the kept lens gets the profile's feeds, so no
        // new feed is left bound to no lens.
        assert_eq!(cfg.lenses[0].feeds, l[0].feeds);
        assert_eq!(applied.bound_to_kept[0].0, "work");

        // A second apply changes nothing.
        let again = apply(&mut doc, &l, &f).unwrap();
        assert!(again.is_empty(), "{again:?}");
    }

    #[test]
    fn apply_keeps_feed_params() {
        let mut f = feed("notes", "filesystem/folder");
        f.params
            .insert("root".into(), toml::Value::String("/tmp/notes".into()));
        f.params
            .insert("recursive".into(), toml::Value::Boolean(false));
        let mut doc = DocumentMut::new();
        apply(&mut doc, &[], std::slice::from_ref(&f)).unwrap();
        let cfg: crate::ArawnConfig = toml::from_str(&doc.to_string()).unwrap();
        assert_eq!(cfg.feeds, [f]);
        assert!(doc.to_string().contains("params = {"), "{doc}");
    }

    #[test]
    fn unknown_profile_names_the_choices() {
        let err = profile("home").unwrap_err();
        assert!(format!("{err}").contains("Use one of: work"));
    }
}
