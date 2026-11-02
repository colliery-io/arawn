//! Declarative lenses and feeds (ARAWN-T-0506): make the store match the
//! `[[lenses]]` and `[[feeds]]` tables of `arawn.toml`.
//!
//! Additive and idempotent. Missing lenses, tags, bindings and feeds are
//! created; nothing is ever deleted, and a running feed that differs from
//! its declaration is reported, not overwritten. A feed whose integration
//! is not connected waits; the reconcile runs again after each successful
//! connect, so it appears as soon as the service is connected.
//!
//! [`plan`] is pure (declared + existing + connected → steps) and carries
//! the rules; [`ReconcileCtx::reconcile`] reads the current state and
//! applies the steps.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use arawn_core::Lens;
use arawn_storage::Store;
use tracing::{info, warn};

use crate::config::{FeedDecl, LensDecl};
use crate::lock_ext::Recover;

/// The declared part of the config, shared with the connect flow.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Declared {
    pub lenses: Vec<LensDecl>,
    pub feeds: Vec<FeedDecl>,
}

impl Declared {
    pub fn is_empty(&self) -> bool {
        self.lenses.is_empty() && self.feeds.is_empty()
    }
}

/// A lens as it exists now.
#[derive(Debug, Clone, Default)]
pub struct ExistingLens {
    pub description: String,
    pub bindings: Vec<String>,
    /// Normalized tags in its ontology.
    pub tags: Vec<String>,
    /// Archived lenses are reported, never changed.
    pub archived: bool,
}

/// What exists now.
#[derive(Debug, Clone, Default)]
pub struct Existing {
    pub lenses: HashMap<String, ExistingLens>,
    /// `id → (template, params)`. `None` when the feed runtime is not
    /// available, so feeds cannot be checked or created.
    pub feeds: Option<HashMap<String, (String, serde_json::Value)>>,
}

/// One thing to do, or to report.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    CreateLens(LensDecl),
    UpdateDescription {
        lens: String,
        description: String,
    },
    AddTags {
        lens: String,
        tags: Vec<String>,
    },
    Bind {
        lens: String,
        feed: String,
    },
    CreateFeed(FeedDecl),
    /// Not done yet; `reason` says what it waits for.
    Waiting {
        feed: String,
        reason: String,
    },
    /// Cannot be done as declared; the message names the fix.
    Problem(String),
}

/// The integration a feed template needs, or `None` (e.g. filesystem).
pub fn template_service(template: &str) -> Option<&'static str> {
    match template.split('/').next().unwrap_or("") {
        "gmail" => Some(arawn_integrations::gmail::SERVICE_NAME),
        "calendar" => Some(arawn_integrations::calendar::SERVICE_NAME),
        "drive" => Some(arawn_integrations::drive::SERVICE_NAME),
        "slack" => Some(arawn_integrations::slack::SERVICE_NAME),
        "jira" | "confluence" => Some(arawn_integrations::atlassian::SERVICE_NAME),
        "github" => Some(arawn_integrations::github::SERVICE_NAME),
        _ => None,
    }
}

/// `null` and `{}` both mean "no params".
fn params_eq(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    let empty = |v: &serde_json::Value| v.is_null() || v.as_object().is_some_and(|o| o.is_empty());
    (empty(a) && empty(b)) || a == b
}

fn params_json(decl: &FeedDecl) -> serde_json::Value {
    serde_json::to_value(&decl.params).unwrap_or(serde_json::Value::Null)
}

/// Decide what to do. Pure: no I/O.
pub fn plan(declared: &Declared, existing: &Existing, connected: &HashSet<String>) -> Vec<Step> {
    let mut steps = Vec::new();

    let mut seen = HashSet::new();
    for l in &declared.lenses {
        if !seen.insert(l.name.as_str()) {
            steps.push(Step::Problem(format!(
                "lens `{}` is declared more than once in arawn.toml; only the first is used",
                l.name
            )));
        }
    }
    let mut seen_feeds = HashSet::new();
    for f in &declared.feeds {
        if !seen_feeds.insert(f.id.as_str()) {
            steps.push(Step::Problem(format!(
                "feed `{}` is declared more than once in arawn.toml; only the first is used",
                f.id
            )));
        }
    }

    let mut done = HashSet::new();
    for l in &declared.lenses {
        if !done.insert(l.name.as_str()) {
            continue;
        }
        // The same rule as lens_new and the store (arawn_core).
        if l.name == arawn_core::SCRATCH_NAME {
            steps.push(Step::Problem(
                "lens name `scratch` is reserved; use a different name".into(),
            ));
            continue;
        }
        if let Err(e) = arawn_core::validate_name(&l.name) {
            steps.push(Step::Problem(format!(
                "lens name `{}` is not allowed: {e}",
                l.name
            )));
            continue;
        }
        let tags: Vec<String> = {
            let mut s = HashSet::new();
            l.tags
                .iter()
                .map(|t| arawn_memory::normalize_tag(t))
                .filter(|t| !t.is_empty() && s.insert(t.clone()))
                .collect()
        };
        match existing.lenses.get(&l.name) {
            None => {
                if tags.is_empty() {
                    steps.push(Step::Problem(format!(
                        "lens `{}` has no tags: give `tags = [...]` in its [[lenses]] entry",
                        l.name
                    )));
                    continue;
                }
                steps.push(Step::CreateLens(LensDecl {
                    tags: tags.clone(),
                    ..l.clone()
                }));
                for feed in &l.feeds {
                    steps.push(Step::Bind {
                        lens: l.name.clone(),
                        feed: feed.clone(),
                    });
                }
            }
            Some(ex) if ex.archived => {
                // Feed routing ignores archived lenses: binding to one
                // would silently send data nowhere.
                steps.push(Step::Problem(format!(
                    "lens `{}` is archived, so arawn does not change it. Remove it from \
                     arawn.toml, or unarchive it",
                    l.name
                )));
            }
            Some(ex) => {
                if !l.description.trim().is_empty() && l.description != ex.description {
                    steps.push(Step::UpdateDescription {
                        lens: l.name.clone(),
                        description: l.description.clone(),
                    });
                }
                let missing: Vec<String> =
                    tags.into_iter().filter(|t| !ex.tags.contains(t)).collect();
                if !missing.is_empty() {
                    steps.push(Step::AddTags {
                        lens: l.name.clone(),
                        tags: missing,
                    });
                }
                for feed in &l.feeds {
                    if !ex.bindings.contains(feed) {
                        steps.push(Step::Bind {
                            lens: l.name.clone(),
                            feed: feed.clone(),
                        });
                    }
                }
            }
        }
    }

    let mut done = HashSet::new();
    for f in &declared.feeds {
        if !done.insert(f.id.as_str()) {
            continue;
        }
        let Some(feeds) = &existing.feeds else {
            steps.push(Step::Waiting {
                feed: f.id.clone(),
                reason: "the feed runtime is not available".into(),
            });
            continue;
        };
        match feeds.get(&f.id) {
            Some((template, _)) if template != &f.template => {
                steps.push(Step::Problem(format!(
                    "feed `{}` exists with template `{template}`, but arawn.toml declares \
                     `{}`. arawn keeps the existing feed. To use the declared one: \
                     /feeds rm {}, then restart arawn serve",
                    f.id, f.template, f.id
                )));
            }
            Some((_, params)) if !params_eq(params, &params_json(f)) => {
                steps.push(Step::Problem(format!(
                    "feed `{}` has params that differ from arawn.toml. arawn keeps the running \
                     feed. To apply the declared params: /feeds rm {}, then restart arawn serve",
                    f.id, f.id
                )));
            }
            Some(_) => {}
            None => match template_service(&f.template) {
                Some(svc) if !connected.contains(svc) => steps.push(Step::Waiting {
                    feed: f.id.clone(),
                    reason: format!("{svc} is not connected (run: arawn connect {svc})"),
                }),
                _ => steps.push(Step::CreateFeed(f.clone())),
            },
        }
    }
    steps
}

/// The declared lenses that exist in `store`, with their bindings and tags.
pub fn existing_lenses(
    store: &Store,
    data_dir: &std::path::Path,
    declared: &Declared,
) -> HashMap<String, ExistingLens> {
    let mut lenses = HashMap::new();
    for l in &declared.lenses {
        if let Ok(Some(lens)) = store.find_lens_by_name(&l.name) {
            let tags = arawn_memory::TagOntologyStore::open(data_dir, &l.name)
                .and_then(|o| o.tags())
                .unwrap_or_default();
            lenses.insert(
                l.name.clone(),
                ExistingLens {
                    description: lens.description,
                    bindings: lens.bindings,
                    tags,
                    archived: lens.archived,
                },
            );
        }
    }
    lenses
}

/// Feed records as `id → (template, params)`.
pub fn feed_map(
    records: Vec<arawn_feeds::FeedRecord>,
) -> HashMap<String, (String, serde_json::Value)> {
    records
        .into_iter()
        .map(|r| (r.id, (r.template, r.params.as_value().clone())))
        .collect()
}

/// Everything the reconcile needs, as clones that can move into a task.
#[derive(Clone)]
pub struct ReconcileCtx {
    pub store: Arc<Mutex<Store>>,
    pub data_dir: PathBuf,
    pub feed_runtime: Arc<std::sync::RwLock<Option<Arc<arawn_feeds::FeedRuntime>>>>,
    pub registry: Arc<std::sync::RwLock<HashMap<String, Arc<dyn arawn_integrations::Integration>>>>,
    pub declared: Arc<std::sync::RwLock<Declared>>,
    /// One reconcile at a time: two connects that finish together must
    /// not both try to create the same feed.
    pub lock: Arc<tokio::sync::Mutex<()>>,
}

/// What a reconcile did, one human-readable line per item.
#[derive(Debug, Clone, Default)]
pub struct ReconcileReport {
    pub done: Vec<String>,
    pub waiting: Vec<String>,
    pub problems: Vec<String>,
}

impl ReconcileCtx {
    async fn connected(&self) -> HashSet<String> {
        let registry: Vec<(String, Arc<dyn arawn_integrations::Integration>)> = self
            .registry
            .read()
            .recover()
            .iter()
            .map(|(k, v)| (k.clone(), Arc::clone(v)))
            .collect();
        let mut out = HashSet::new();
        for (name, i) in registry {
            if i.is_connected().await {
                out.insert(name);
            }
        }
        out
    }

    async fn existing(&self, declared: &Declared) -> Existing {
        let lenses = {
            let store = self.store.lock().recover();
            existing_lenses(&store, &self.data_dir, declared)
        };
        let runtime = self.feed_runtime.read().recover().clone();
        let feeds = match runtime {
            Some(rt) => {
                let conn = rt.runtime_ctx().conn.lock().await;
                arawn_feeds::FeedStore::new(&conn)
                    .list_all()
                    .ok()
                    .map(feed_map)
            }
            None => None,
        };
        Existing { lenses, feeds }
    }

    /// Make the store match the declaration. Never fails as a whole:
    /// each step that fails is reported and the rest continue.
    pub async fn reconcile(&self) -> ReconcileReport {
        let declared = self.declared.read().recover().clone();
        let mut report = ReconcileReport::default();
        if declared.is_empty() {
            return report;
        }
        let _one_at_a_time = self.lock.lock().await;
        let existing = self.existing(&declared).await;
        let connected = self.connected().await;
        for step in plan(&declared, &existing, &connected) {
            match self.apply(step).await {
                Outcome::Done(s) => {
                    info!("arawn.toml: {s}");
                    report.done.push(s);
                }
                Outcome::Waiting(s) => {
                    info!("arawn.toml: {s}");
                    report.waiting.push(s);
                }
                Outcome::Problem(s) => {
                    warn!("arawn.toml: {s}");
                    report.problems.push(s);
                }
            }
        }
        report
    }

    async fn apply(&self, step: Step) -> Outcome {
        match step {
            Step::CreateLens(l) => {
                let root = self.data_dir.join("lenses").join(&l.name);
                let mut lens = Lens::new(&l.name, &root);
                lens.display_name = l.display_name.clone().unwrap_or_else(|| l.name.clone());
                lens.description = l.description.clone();
                if let Err(e) = self.store.lock().recover().create_lens(&lens) {
                    return Outcome::Problem(format!("cannot create lens `{}`: {e}", l.name));
                }
                match self.add_tags(&l.name, &l.tags) {
                    Ok(()) => Outcome::Done(format!(
                        "created lens `{}` with {} tag(s)",
                        l.name,
                        l.tags.len()
                    )),
                    Err(e) => Outcome::Problem(format!(
                        "created lens `{}`, but its tags were not added: {e}",
                        l.name
                    )),
                }
            }
            Step::UpdateDescription { lens, description } => {
                match self
                    .store
                    .lock()
                    .recover()
                    .update_lens_description(&lens, &description)
                {
                    Ok(()) => Outcome::Done(format!("updated the description of lens `{lens}`")),
                    Err(e) => Outcome::Problem(format!(
                        "cannot update the description of lens `{lens}`: {e}"
                    )),
                }
            }
            Step::AddTags { lens, tags } => match self.add_tags(&lens, &tags) {
                Ok(()) => {
                    Outcome::Done(format!("added tag(s) {} to lens `{lens}`", tags.join(", ")))
                }
                Err(e) => Outcome::Problem(format!("cannot add tags to lens `{lens}`: {e}")),
            },
            Step::Bind { lens, feed } => {
                match self.store.lock().recover().add_lens_binding(&lens, &feed) {
                    Ok(()) => Outcome::Done(format!("bound feed `{feed}` to lens `{lens}`")),
                    Err(e) => {
                        Outcome::Problem(format!("cannot bind feed `{feed}` to lens `{lens}`: {e}"))
                    }
                }
            }
            Step::CreateFeed(f) => {
                let runtime = self.feed_runtime.read().recover().clone();
                let Some(rt) = runtime else {
                    return Outcome::Waiting(format!(
                        "feed `{}` waits: the feed runtime is not available",
                        f.id
                    ));
                };
                let params = arawn_feeds::TemplateParams::new(params_json(&f));
                match rt
                    .register_feed_dynamic(&f.template, &f.id, params, f.cadence.clone())
                    .await
                {
                    Ok(_) => Outcome::Done(format!("created feed `{}` ({})", f.id, f.template)),
                    Err(e) => Outcome::Problem(format!(
                        "cannot create feed `{}` ({}): {e}",
                        f.id, f.template
                    )),
                }
            }
            Step::Waiting { feed, reason } => {
                Outcome::Waiting(format!("feed `{feed}` waits: {reason}"))
            }
            Step::Problem(p) => Outcome::Problem(p),
        }
    }

    fn add_tags(&self, lens: &str, tags: &[String]) -> Result<(), String> {
        let o = arawn_memory::TagOntologyStore::open(&self.data_dir, lens)
            .map_err(|e| e.to_string())?;
        o.add_many(tags.iter().cloned(), arawn_memory::AddedVia::Manual)
            .map_err(|e| e.to_string())
    }
}

enum Outcome {
    Done(String),
    Waiting(String),
    Problem(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lens(name: &str, tags: &[&str], feeds: &[&str]) -> LensDecl {
        LensDecl {
            name: name.into(),
            display_name: None,
            description: format!("{name} lens"),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            feeds: feeds.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn feed(id: &str, template: &str) -> FeedDecl {
        FeedDecl {
            id: id.into(),
            template: template.into(),
            ..Default::default()
        }
    }

    fn connected(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn no_feeds() -> Existing {
        Existing {
            lenses: HashMap::new(),
            feeds: Some(HashMap::new()),
        }
    }

    #[test]
    fn fresh_install_creates_lens_binds_and_creates_connected_feeds_only() {
        let d = Declared {
            lenses: vec![lens(
                "work",
                &["Hiring", "architecture"],
                &["gmail-inbox", "slack-mentions"],
            )],
            feeds: vec![
                feed("gmail-inbox", "gmail/inbox-archive"),
                feed("slack-mentions", "slack/my-mentions"),
            ],
        };
        let steps = plan(&d, &no_feeds(), &connected(&["gmail"]));
        assert!(matches!(&steps[0], Step::CreateLens(l) if l.tags == ["hiring", "architecture"]));
        assert!(steps.contains(&Step::Bind {
            lens: "work".into(),
            feed: "gmail-inbox".into()
        }));
        assert!(steps.contains(&Step::Bind {
            lens: "work".into(),
            feed: "slack-mentions".into()
        }));
        assert!(steps.contains(&Step::CreateFeed(feed(
            "gmail-inbox",
            "gmail/inbox-archive"
        ))));
        assert!(steps.iter().any(|s| matches!(s,
            Step::Waiting { feed, reason } if feed == "slack-mentions" && reason.contains("arawn connect slack"))));
    }

    #[test]
    fn second_run_over_the_result_is_a_no_op() {
        let d = Declared {
            lenses: vec![lens("work", &["hiring"], &["gmail-inbox"])],
            feeds: vec![feed("gmail-inbox", "gmail/inbox-archive")],
        };
        let existing = Existing {
            lenses: HashMap::from([(
                "work".into(),
                ExistingLens {
                    description: "work lens".into(),
                    bindings: vec!["gmail-inbox".into()],
                    // Tags added later at runtime are kept, not removed.
                    tags: vec!["hiring".into(), "added-later".into()],
                    archived: false,
                },
            )]),
            feeds: Some(HashMap::from([(
                "gmail-inbox".into(),
                ("gmail/inbox-archive".into(), serde_json::Value::Null),
            )])),
        };
        assert!(plan(&d, &existing, &connected(&["gmail"])).is_empty());
    }

    #[test]
    fn existing_lens_gets_missing_tags_bindings_and_description_only() {
        let mut l = lens("work", &["hiring", "vendors"], &["a", "b"]);
        l.description = "new description".into();
        let d = Declared {
            lenses: vec![l],
            feeds: vec![],
        };
        let existing = Existing {
            lenses: HashMap::from([(
                "work".into(),
                ExistingLens {
                    description: "old".into(),
                    bindings: vec!["a".into()],
                    tags: vec!["hiring".into()],
                    archived: false,
                },
            )]),
            feeds: Some(HashMap::new()),
        };
        let steps = plan(&d, &existing, &connected(&[]));
        assert_eq!(
            steps,
            [
                Step::UpdateDescription {
                    lens: "work".into(),
                    description: "new description".into()
                },
                Step::AddTags {
                    lens: "work".into(),
                    tags: vec!["vendors".into()]
                },
                Step::Bind {
                    lens: "work".into(),
                    feed: "b".into()
                },
            ]
        );
    }

    #[test]
    fn drift_is_reported_never_overwritten() {
        let mut f = feed("gmail-inbox", "gmail/inbox-archive");
        f.params
            .insert("label".into(), toml::Value::String("INBOX".into()));
        let d = Declared {
            lenses: vec![],
            feeds: vec![f, feed("drive", "drive/recent")],
        };
        let existing = Existing {
            lenses: HashMap::new(),
            feeds: Some(HashMap::from([
                (
                    "gmail-inbox".into(),
                    ("gmail/inbox-archive".into(), serde_json::json!({})),
                ),
                (
                    "drive".into(),
                    ("slack/my-mentions".into(), serde_json::Value::Null),
                ),
            ])),
        };
        let steps = plan(&d, &existing, &connected(&["gmail", "google_drive"]));
        assert_eq!(steps.len(), 2, "{steps:?}");
        assert!(steps.iter().all(|s| matches!(s, Step::Problem(_))));
        assert!(format!("{steps:?}").contains("params that differ"));
        assert!(format!("{steps:?}").contains("exists with template `slack/my-mentions`"));
    }

    #[test]
    fn bad_declarations_are_problems_not_actions() {
        let d = Declared {
            lenses: vec![
                lens("Work Stuff", &["a"], &[]),
                lens("scratch", &["a"], &[]),
                lens("empty", &[], &[]),
                lens("dup", &["a"], &[]),
                lens("dup", &["b"], &[]),
            ],
            feeds: vec![
                feed("x", "filesystem/folder"),
                feed("x", "filesystem/folder"),
            ],
        };
        let steps = plan(&d, &no_feeds(), &connected(&[]));
        let problems: Vec<_> = steps
            .iter()
            .filter_map(|s| match s {
                Step::Problem(p) => Some(p.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(problems.len(), 5, "{problems:?}");
        // The first `dup` is still created; filesystem needs no connect.
        assert!(
            steps
                .iter()
                .any(|s| matches!(s, Step::CreateLens(l) if l.name == "dup" && l.tags == ["a"]))
        );
        assert_eq!(
            steps
                .iter()
                .filter(|s| matches!(s, Step::CreateFeed(_)))
                .count(),
            1
        );
    }

    #[test]
    fn no_feed_runtime_means_every_feed_waits() {
        let d = Declared {
            lenses: vec![],
            feeds: vec![feed("f", "filesystem/folder")],
        };
        let existing = Existing::default();
        assert!(matches!(
            &plan(&d, &existing, &connected(&[]))[0],
            Step::Waiting { reason, .. } if reason.contains("feed runtime")
        ));
    }

    #[test]
    fn archived_lens_is_a_problem_and_is_not_bound() {
        // Review regression: routing ignores archived lenses, so binding a
        // new feed to one would send its data nowhere.
        let d = Declared {
            lenses: vec![lens("work", &["hiring"], &["new-feed"])],
            feeds: vec![],
        };
        let existing = Existing {
            lenses: HashMap::from([(
                "work".into(),
                ExistingLens {
                    archived: true,
                    ..Default::default()
                },
            )]),
            feeds: Some(HashMap::new()),
        };
        let steps = plan(&d, &existing, &connected(&[]));
        assert_eq!(steps.len(), 1, "{steps:?}");
        assert!(matches!(&steps[0], Step::Problem(p) if p.contains("archived")));
    }

    #[test]
    fn lens_names_follow_the_core_rule() {
        // Review regression: a leading '-' or '_' passed the old local
        // check, then failed in the store on every startup.
        for bad in ["-work", "_work", &"x".repeat(65)] {
            let d = Declared {
                lenses: vec![lens(bad, &["a"], &[])],
                feeds: vec![],
            };
            let steps = plan(&d, &no_feeds(), &connected(&[]));
            assert!(
                matches!(&steps[..], [Step::Problem(p)] if p.contains("not allowed")),
                "{bad}: {steps:?}"
            );
        }
    }

    #[test]
    fn template_services_match_integration_names() {
        assert_eq!(template_service("gmail/inbox-archive"), Some("gmail"));
        assert_eq!(
            template_service("calendar/upcoming-archive"),
            Some("google_calendar")
        );
        assert_eq!(
            template_service("confluence/space-archive"),
            Some("atlassian")
        );
        assert_eq!(template_service("github/review-queue"), Some("github"));
        assert_eq!(template_service("filesystem/folder"), None);
    }
}
