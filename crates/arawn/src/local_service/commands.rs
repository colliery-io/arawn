//! `LocalService` inherent methods backing the `commands.*` portion of
//! `ArawnService`. The trait shell in `super::mod` delegates to these.

use arawn_service::{CommandInfo, InventoryItem, ServiceError, WorkflowInfo};

use super::{LocalService, first_sentence};

impl LocalService {
    pub(super) async fn query_inventory_inner(
        &self,
        kind: &str,
    ) -> Result<Vec<InventoryItem>, ServiceError> {
        let items = match kind {
            "tools" => self
                .registry
                .tool_definitions()
                .iter()
                .map(|t| InventoryItem {
                    name: t.name.clone(),
                    description: first_sentence(&t.description),
                    kind: None,
                    enabled: None,
                    user_invocable: None,
                })
                .collect(),
            "skills" => {
                if let Some(ref reg) = self.skill_registry {
                    reg.all()
                        .iter()
                        .map(|s| InventoryItem {
                            name: s.name.clone(),
                            description: first_sentence(&s.description),
                            kind: None,
                            enabled: None,
                            user_invocable: Some(s.user_invocable),
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            "plugins" => {
                if let Some(ref reg) = self.plugin_registry {
                    reg.all()
                        .iter()
                        .map(|p| InventoryItem {
                            name: p.name().to_string(),
                            description: p
                                .manifest
                                .description
                                .as_deref()
                                .unwrap_or("")
                                .to_string(),
                            kind: None,
                            enabled: Some(p.enabled),
                            user_invocable: None,
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            "agents" => arawn_engine::agent_defs::built_in_agents()
                .iter()
                .map(|a| InventoryItem {
                    name: a.name.clone(),
                    description: first_sentence(&a.when_to_use),
                    kind: None,
                    enabled: None,
                    user_invocable: None,
                })
                .collect(),
            "mcp" => Vec::new(),
            _ => Vec::new(),
        };
        Ok(items)
    }

    pub(super) async fn list_available_commands_inner(
        &self,
    ) -> Result<Vec<CommandInfo>, ServiceError> {
        let mut commands = Vec::new();
        if let Some(ref reg) = self.skill_registry {
            for skill in reg.user_invocable() {
                commands.push(CommandInfo {
                    name: skill.name.clone(),
                    description: first_sentence(&skill.description),
                    kind: "skill".to_string(),
                });
            }
        }
        Ok(commands)
    }

    pub(super) async fn list_workflows_inner(&self) -> Result<Vec<WorkflowInfo>, ServiceError> {
        let workflows_dir = self.data_dir.join("workflows");
        let mut workflows = Vec::new();
        if workflows_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&workflows_dir)
        {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let pkg_toml = entry.path().join("package.toml");
                    let cron = pkg_toml
                        .exists()
                        .then(|| {
                            std::fs::read_to_string(&pkg_toml).ok().and_then(|s| {
                                s.lines().find(|l| l.contains("cron")).map(|l| {
                                    l.split('=')
                                        .nth(1)
                                        .unwrap_or("")
                                        .trim()
                                        .trim_matches('"')
                                        .to_string()
                                })
                            })
                        })
                        .flatten();
                    workflows.push(WorkflowInfo { name, cron });
                }
            }
        }
        Ok(workflows)
    }
}
