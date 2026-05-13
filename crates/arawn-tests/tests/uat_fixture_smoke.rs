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
    assert!(work.rows.len() >= 10, "work has {} rows (want >=10)", work.rows.len());
    assert!(dnd.rows.len() >= 8, "dnd has {} rows (want >=8)", dnd.rows.len());
}
