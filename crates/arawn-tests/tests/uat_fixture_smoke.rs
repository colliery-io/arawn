//! Smoke test: the real signal-extraction-e2e fixture parses and the
//! row counts match what the UAT scenario expects.

mod uat_fixture;

#[test]
fn signal_extraction_e2e_fixture_parses() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("uat")
        .join("signal-extraction-e2e.json");
    let fx = uat_fixture::load(&path).expect("fixture parses");
    assert_eq!(fx.workstreams.len(), 2);
    let work = fx
        .workstreams
        .iter()
        .find(|w| w.name == "work")
        .expect("work workstream");
    let dnd = fx
        .workstreams
        .iter()
        .find(|w| w.name == "dnd")
        .expect("dnd workstream");
    assert!(
        work.rows.len() >= 10,
        "work has {} rows (want >=10)",
        work.rows.len()
    );
    assert!(
        dnd.rows.len() >= 8,
        "dnd has {} rows (want >=8)",
        dnd.rows.len()
    );
}

/// T-0332: the synthetic life-assistant fixture must parse, have a
/// single `personal` workstream pinned to the assistant persona, and
/// carry rows across all four sources (gmail, slack, calendar, jira).
#[test]
fn personal_day_fixture_parses() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("uat")
        .join("personal-day.json");
    let fx = uat_fixture::load(&path).expect("personal-day fixture parses");
    assert_eq!(fx.workstreams.len(), 1);
    let ws = &fx.workstreams[0];
    assert_eq!(ws.name, "personal");
    assert_eq!(ws.identity_profile.as_deref(), Some("assistant"));

    let mut gmail = 0;
    let mut slack = 0;
    let mut calendar = 0;
    let mut jira_issues = 0;
    let mut jira_comments = 0;
    for row in &ws.rows {
        match row {
            uat_fixture::FixtureRow::GmailMessages(_) => gmail += 1,
            uat_fixture::FixtureRow::SlackMessages(_) => slack += 1,
            uat_fixture::FixtureRow::CalendarEvents(_) => calendar += 1,
            uat_fixture::FixtureRow::JiraIssues(_) => jira_issues += 1,
            uat_fixture::FixtureRow::JiraComments(_) => jira_comments += 1,
            uat_fixture::FixtureRow::FilesystemSignals(_) => {}
        }
    }
    assert!(gmail >= 8, "gmail rows: {gmail} (want >=8)");
    assert!(slack >= 6, "slack rows: {slack} (want >=6)");
    assert!(calendar >= 4, "calendar rows: {calendar} (want >=4)");
    assert!(jira_issues >= 3, "jira issues: {jira_issues} (want >=3)");
    assert!(
        jira_comments >= 1,
        "jira comments: {jira_comments} (want >=1)"
    );
}
