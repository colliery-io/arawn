//! Workflow scaffold generation — produces a compilable Cargo project
//! using cloacina-workflow macros that can be packaged as a `.cloacina` archive.

use std::path::Path;

/// Definition of a single task within a workflow.
pub struct TaskDef {
    /// Unique task ID within the workflow.
    pub id: String,
    /// IDs of tasks this depends on (must complete first).
    pub dependencies: Vec<String>,
    /// Rust async function body (inserted into the task fn).
    pub body: String,
    /// Max retry attempts (default: 3).
    pub retry_attempts: Option<i32>,
}

/// Definition of a workflow to scaffold.
pub struct WorkflowDef {
    /// Workflow name (used as crate name and workflow identifier).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Tasks in the workflow DAG.
    pub tasks: Vec<TaskDef>,
    /// Optional cron expression (e.g., "0 8 * * 1-5").
    pub cron: Option<String>,
    /// Cron timezone (default: "UTC").
    pub cron_timezone: Option<String>,
}

/// Generate a complete workflow Cargo project in the given directory.
///
/// Creates: Cargo.toml, build.rs, package.toml, src/lib.rs
pub fn generate(dir: &Path, def: &WorkflowDef) -> Result<(), ScaffoldError> {
    let src_dir = dir.join("src");
    std::fs::create_dir_all(&src_dir)
        .map_err(|e| ScaffoldError(format!("create src dir: {e}")))?;

    let crate_name = def.name.replace('-', "_");

    std::fs::write(dir.join("Cargo.toml"), cargo_toml(&def.name))
        .map_err(|e| ScaffoldError(format!("write Cargo.toml: {e}")))?;

    std::fs::write(dir.join("build.rs"), BUILD_RS)
        .map_err(|e| ScaffoldError(format!("write build.rs: {e}")))?;

    std::fs::write(dir.join("package.toml"), package_toml(&def.name, &crate_name, &def.description))
        .map_err(|e| ScaffoldError(format!("write package.toml: {e}")))?;

    std::fs::write(src_dir.join("lib.rs"), lib_rs(def, &crate_name))
        .map_err(|e| ScaffoldError(format!("write src/lib.rs: {e}")))?;

    Ok(())
}

fn cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
cloacina-workflow = {{ version = "0.4", features = ["packaged"] }}
serde_json = "1"
tokio = {{ version = "1", features = ["full"] }}

[build-dependencies]
cloacina-build = "0.4"
"#
    )
}

const BUILD_RS: &str = r#"fn main() {
    cloacina_build::configure();
}
"#;

fn package_toml(name: &str, workflow_name: &str, description: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"

[metadata]
workflow_name = "{workflow_name}"
language = "rust"
description = "{description}"
"#
    )
}

fn lib_rs(def: &WorkflowDef, crate_name: &str) -> String {
    let mut out = String::new();
    out.push_str("use cloacina_workflow::{workflow, task, Context, TaskError};\n");
    out.push_str("use serde_json::Value;\n\n");

    // Trigger import if cron is set
    if def.cron.is_some() {
        out.push_str("use cloacina_workflow::trigger;\n\n");
    }

    // Workflow module
    out.push_str(&format!(
        "#[workflow(name = \"{crate_name}\", description = \"{}\")]\n",
        def.description.replace('"', "\\\"")
    ));
    out.push_str(&format!("pub mod {crate_name} {{\n"));
    out.push_str("    use super::*;\n\n");

    for task in &def.tasks {
        let deps = task
            .dependencies
            .iter()
            .map(|d| format!("\"{d}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let mut attrs = format!("id = \"{}\", dependencies = [{deps}]", task.id);
        if let Some(retries) = task.retry_attempts {
            attrs.push_str(&format!(", retry_attempts = {retries}"));
        }

        out.push_str(&format!("    #[task({attrs})]\n"));
        out.push_str(&format!(
            "    pub async fn {}(context: &mut Context<Value>) -> Result<(), TaskError> {{\n",
            task.id
        ));

        // Indent body lines
        for line in task.body.lines() {
            out.push_str(&format!("        {line}\n"));
        }

        out.push_str("    }\n\n");
    }

    out.push_str("}\n");

    // Trigger function if cron is set
    if let Some(ref cron) = def.cron {
        let tz = def.cron_timezone.as_deref().unwrap_or("UTC");
        out.push_str(&format!(
            "\n#[trigger(on = \"{crate_name}\", cron = \"{cron}\", timezone = \"{tz}\")]\n"
        ));
        out.push_str(&format!("pub async fn scheduled() {{}}\n"));
    }

    out
}

#[derive(Debug, thiserror::Error)]
#[error("scaffold error: {0}")]
pub struct ScaffoldError(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_project_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let def = WorkflowDef {
            name: "test-workflow".into(),
            description: "A test workflow".into(),
            tasks: vec![
                TaskDef {
                    id: "fetch".into(),
                    dependencies: vec![],
                    body: "context.insert(\"data\", serde_json::json!({\"ok\": true}))?;\nOk(())".into(),
                    retry_attempts: Some(3),
                },
                TaskDef {
                    id: "process".into(),
                    dependencies: vec!["fetch".into()],
                    body: "let _data = context.get(\"data\");\nOk(())".into(),
                    retry_attempts: None,
                },
            ],
            cron: Some("0 8 * * 1-5".into()),
            cron_timezone: None,
        };

        generate(tmp.path(), &def).unwrap();

        assert!(tmp.path().join("Cargo.toml").exists());
        assert!(tmp.path().join("build.rs").exists());
        assert!(tmp.path().join("package.toml").exists());
        assert!(tmp.path().join("src/lib.rs").exists());

        let cargo = std::fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
        assert!(cargo.contains("test-workflow"));
        assert!(cargo.contains("cdylib"));
        assert!(cargo.contains("packaged"));

        let lib = std::fs::read_to_string(tmp.path().join("src/lib.rs")).unwrap();
        assert!(lib.contains("#[workflow(name = \"test_workflow\""));
        assert!(lib.contains("#[task(id = \"fetch\""));
        assert!(lib.contains("dependencies = [\"fetch\"]"));
        assert!(lib.contains("#[trigger(on = \"test_workflow\""));
        assert!(lib.contains("cron = \"0 8 * * 1-5\""));
    }

    #[test]
    fn no_trigger_when_no_cron() {
        let tmp = tempfile::tempdir().unwrap();
        let def = WorkflowDef {
            name: "simple".into(),
            description: "No cron".into(),
            tasks: vec![TaskDef {
                id: "run".into(),
                dependencies: vec![],
                body: "Ok(())".into(),
                retry_attempts: None,
            }],
            cron: None,
            cron_timezone: None,
        };

        generate(tmp.path(), &def).unwrap();

        let lib = std::fs::read_to_string(tmp.path().join("src/lib.rs")).unwrap();
        assert!(!lib.contains("trigger"));
    }
}
