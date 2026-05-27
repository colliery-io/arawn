use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use arawn_core::{Lens, SCRATCH_NAME};
use arawn_storage::Store;
use arawn_tool::{Tool, ToolCategory, ToolError, ToolOutput};

// ============================================================================
// lens_new
// ============================================================================

pub struct LensCreateTool {
    store: Arc<Mutex<Store>>,
}

impl LensCreateTool {
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for LensCreateTool {
    fn name(&self) -> &str {
        "lens_new"
    }

    fn description(&self) -> &str {
        "Create a new lens with a declared tag ontology. Per ADR-0004 \
         the ontology is required at creation — the agent should propose it \
         via `lens_propose_ontology(description)`, confirm with the \
         user, then call this tool with the agreed `tags_ontology`. Name \
         must be a slug (lowercase, digits, '-' and '_' only). Does not \
         switch into the new lens."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Lens
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Slug name (e.g. 'pat', 'auth-migration')"},
                "display_name": {"type": "string", "description": "Optional human label (defaults to name)"},
                "description": {"type": "string", "description": "Required free-text description — what this lens tracks"},
                "tags_ontology": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Required non-empty list of initial ontology tags. These form the closed list of tags the extractor may attach to entities. Add more later via `lens_apply` of `tag-promoter` proposals or directly with `lens_tag add`."
                }
            },
            "required": ["name", "description", "tags_ontology"]
        })
    }

    async fn execute(
        &self,
        ctx: &dyn arawn_tool::ToolContext,
        params: Value,
    ) -> Result<ToolOutput, ToolError> {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return Ok(ToolOutput::error("name is required".to_string())),
        };
        if name == SCRATCH_NAME {
            return Ok(ToolOutput::error(
                "the name 'scratch' is reserved — it always exists".to_string(),
            ));
        }
        let display_name = params
            .get("display_name")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| name.clone());
        let description = match params.get("description").and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => s.to_string(),
            _ => {
                return Ok(ToolOutput::error(
                    "description is required — it shapes ontology proposals and seeds the extractor".to_string(),
                ));
            }
        };

        // Ontology is required and non-empty. Normalize + dedupe so a
        // caller that sends `["Falcon", "falcon ", "FALCON"]` lands a
        // single `falcon` tag.
        let tags_ontology: Vec<String> = match params
            .get("tags_ontology")
            .and_then(|v| v.as_array())
        {
            Some(arr) if !arr.is_empty() => {
                let mut seen = std::collections::HashSet::new();
                let mut out = Vec::new();
                for v in arr {
                    if let Some(s) = v.as_str() {
                        let canonical = arawn_memory::normalize_tag(s);
                        if !canonical.is_empty() && seen.insert(canonical.clone()) {
                            out.push(canonical);
                        }
                    }
                }
                if out.is_empty() {
                    return Ok(ToolOutput::error(
                        "tags_ontology contained no valid tags after normalization".to_string(),
                    ));
                }
                out
            }
            _ => {
                return Ok(ToolOutput::error(
                    "tags_ontology is required — pass a non-empty array of initial ontology tags (use `lens_propose_ontology` to suggest)".to_string(),
                ));
            }
        };

        let data_dir = match ctx.data_dir() {
            Some(d) => d.to_path_buf(),
            None => {
                return Ok(ToolOutput::error(
                    "no data_dir available — required to materialize the lens's KB + ontology"
                        .to_string(),
                ));
            }
        };
        let root_dir = data_dir.join("lenses").join(&name);

        let mut ws = Lens::new(&name, &root_dir);
        ws.display_name = display_name;
        ws.description = description;

        // Insert the lens record first; if ontology seeding fails
        // afterwards the record stays — user can re-run `lens_tag
        // add` to recover. The alternative (rollback the lens)
        // doubles the failure modes for negligible benefit.
        {
            let store = self.store.lock().unwrap();
            if let Err(e) = store.create_lens(&ws) {
                return Ok(ToolOutput::error(format!("failed to create lens: {e}")));
            }
        }

        // Seed the ontology in the lens's colocated table.
        let ontology = match arawn_memory::TagOntologyStore::open(&data_dir, &name) {
            Ok(o) => o,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "lens record created but ontology open failed: {e}"
                )));
            }
        };
        if let Err(e) = ontology.add_many(
            tags_ontology.iter().cloned(),
            arawn_memory::AddedVia::Manual,
        ) {
            return Ok(ToolOutput::error(format!(
                "lens record created but ontology seed failed: {e}"
            )));
        }

        Ok(ToolOutput::success(
            json!({
                "name": ws.name,
                "display_name": ws.display_name,
                "root_dir": ws.root_dir.display().to_string(),
                "tags_ontology": tags_ontology,
            })
            .to_string(),
        ))
    }
}
