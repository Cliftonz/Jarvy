//! Preview changes before running setup (dry-run)
//!
//! Shows what tools would be installed, updated, or are already satisfied.

use crate::config::Config;
use crate::output::{ExitCode, Format, Outputable, colors, header, icons, subheader};
use crate::tools::common::{cmd_satisfies, has};
use crate::tools::spec::{get_tool_default_hook, get_tool_install_info, get_tool_spec};
use serde::Serialize;

/// A change that would be made during setup
#[derive(Debug, Clone, Serialize)]
pub struct ToolChange {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_method: Option<String>,
}

/// A hook that would run during setup
#[derive(Debug, Clone, Serialize)]
pub struct HookInfo {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_preview: Option<String>,
}

/// A service that would start/stop
#[derive(Debug, Clone, Serialize)]
pub struct ServiceChange {
    pub name: String,
    pub action: String,
    pub backend: String,
}

/// Complete diff result
#[derive(Debug, Clone, Serialize)]
pub struct DiffResult {
    pub to_install: Vec<ToolChange>,
    pub to_update: Vec<ToolChange>,
    pub satisfied: Vec<ToolChange>,
    pub unknown: Vec<ToolChange>,
    pub hooks_to_run: Vec<HookInfo>,
    pub services: Vec<ServiceChange>,
}

impl DiffResult {
    /// Check if there are any changes to make
    pub fn has_changes(&self) -> bool {
        !self.to_install.is_empty()
            || !self.to_update.is_empty()
            || !self.hooks_to_run.is_empty()
            || !self.services.is_empty()
    }

    /// Total number of tools that need action
    pub fn action_count(&self) -> usize {
        self.to_install.len() + self.to_update.len()
    }
}

impl Outputable for DiffResult {
    fn to_human(&self) -> String {
        let mut output = String::new();

        output.push_str(&header("Jarvy Diff - Preview of Changes"));
        output.push('\n');

        if !self.has_changes() && self.unknown.is_empty() {
            output.push_str(&format!(
                "\n{}All {} tools are satisfied. Nothing to do.{}\n",
                colors::GREEN,
                self.satisfied.len(),
                colors::RESET
            ));
            return output;
        }

        // Tools to Install
        if !self.to_install.is_empty() {
            output.push_str(&subheader("Tools to Install"));
            for tool in &self.to_install {
                let method = tool
                    .install_method
                    .as_ref()
                    .map(|m| format!(" via {}", m))
                    .unwrap_or_default();
                output.push_str(&format!(
                    "  {}{}{} {} ({}){}\n",
                    colors::GREEN,
                    icons::INSTALL,
                    colors::RESET,
                    tool.name,
                    tool.version,
                    method
                ));
            }
        }

        // Tools to Update
        if !self.to_update.is_empty() {
            output.push_str(&subheader("Tools to Update"));
            for tool in &self.to_update {
                let current = tool
                    .current_version
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or("unknown");
                output.push_str(&format!(
                    "  {}{}{} {} {} -> {} (requires: {})\n",
                    colors::YELLOW,
                    icons::UPDATE,
                    colors::RESET,
                    tool.name,
                    current,
                    tool.version,
                    tool.version
                ));
            }
        }

        // Tools Already Satisfied
        if !self.satisfied.is_empty() {
            output.push_str(&subheader("Tools Already Satisfied"));
            for tool in &self.satisfied {
                let current = tool
                    .current_version
                    .as_ref()
                    .map(|v| format!(" ({})", v))
                    .unwrap_or_default();
                output.push_str(&format!(
                    "  {}{}{} {}{} (requires: {})\n",
                    colors::DIM,
                    icons::SATISFIED,
                    colors::RESET,
                    tool.name,
                    current,
                    tool.version
                ));
            }
        }

        // Unknown Tools
        if !self.unknown.is_empty() {
            output.push_str(&subheader("Unknown Tools"));
            for tool in &self.unknown {
                output.push_str(&format!(
                    "  {}?{} {} ({}) - not in registry\n",
                    colors::CYAN,
                    colors::RESET,
                    tool.name,
                    tool.version
                ));
            }
        }

        // Hooks to Run
        if !self.hooks_to_run.is_empty() {
            output.push_str(&subheader("Hooks to Run"));
            for hook in &self.hooks_to_run {
                output.push_str(&format!(
                    "  {}{}{} {}: {}\n",
                    colors::BLUE,
                    icons::HOOK,
                    colors::RESET,
                    hook.name,
                    hook.description
                ));
            }
        }

        // Services
        if !self.services.is_empty() {
            output.push_str(&subheader("Services"));
            for service in &self.services {
                output.push_str(&format!(
                    "  {}{}{} {} ({} via {})\n",
                    colors::BLUE,
                    icons::HOOK,
                    colors::RESET,
                    service.name,
                    service.action,
                    service.backend
                ));
            }
        }

        output.push_str(&format!(
            "\n{}No changes will be made.{} Run 'jarvy setup' to apply.\n",
            colors::DIM,
            colors::RESET
        ));

        output
    }

    fn exit_code(&self) -> ExitCode {
        if !self.unknown.is_empty() {
            ExitCode::Warning
        } else {
            ExitCode::Ok
        }
    }
}

/// Run the diff command
pub fn run_diff(config: &Config, changes_only: bool) -> DiffResult {
    let mut to_install = Vec::new();
    let mut to_update = Vec::new();
    let mut satisfied = Vec::new();
    let mut unknown = Vec::new();
    let mut hooks_to_run = Vec::new();
    let mut services = Vec::new();

    let tools = config.get_tool_configs();

    for (_, tool) in &tools {
        let spec = get_tool_spec(&tool.name);
        let is_known = spec.is_some() || crate::tools::get_tool(&tool.name).is_some();

        if !is_known {
            unknown.push(ToolChange {
                name: tool.name.clone(),
                version: tool.version.clone(),
                current_version: None,
                install_method: None,
            });
            continue;
        }

        let command = spec.map(|s| s.command).unwrap_or(tool.name.as_str());
        let is_installed = has(command);
        let is_satisfied = cmd_satisfies(command, &tool.version);
        let current_version = if is_installed {
            get_installed_version(command)
        } else {
            None
        };

        // Get install method
        let install_method = get_tool_install_info(&tool.name, &tool.version)
            .map(|info| format!("{:?}", info.package_manager).to_lowercase());

        if !is_installed {
            to_install.push(ToolChange {
                name: tool.name.clone(),
                version: tool.version.clone(),
                current_version: None,
                install_method,
            });

            // Check for default hook
            if let Some(hook) = get_tool_default_hook(&tool.name) {
                hooks_to_run.push(HookInfo {
                    name: format!("{} (default)", tool.name),
                    description: hook.description.to_string(),
                    script_preview: Some(
                        hook.script.lines().take(2).collect::<Vec<_>>().join("\n"),
                    ),
                });
            }

            // Check for user hook
            if let Some(hook) = config.get_tool_hooks(&tool.name) {
                if hook.post_install.is_some() {
                    hooks_to_run.push(HookInfo {
                        name: format!("{} (user)", tool.name),
                        description: "Custom post-install hook".to_string(),
                        script_preview: None,
                    });
                }
            }
        } else if !is_satisfied {
            to_update.push(ToolChange {
                name: tool.name.clone(),
                version: tool.version.clone(),
                current_version,
                install_method,
            });
        } else if !changes_only {
            satisfied.push(ToolChange {
                name: tool.name.clone(),
                version: tool.version.clone(),
                current_version,
                install_method: None,
            });
        }
    }

    // Check for pre/post setup hooks
    let hooks_config = config.get_hooks();
    if hooks_config.pre_setup.is_some() {
        hooks_to_run.insert(
            0,
            HookInfo {
                name: "pre_setup".to_string(),
                description: "Pre-setup hook".to_string(),
                script_preview: None,
            },
        );
    }
    if hooks_config.post_setup.is_some() {
        hooks_to_run.push(HookInfo {
            name: "post_setup".to_string(),
            description: "Post-setup hook".to_string(),
            script_preview: None,
        });
    }

    // Check for services
    if config.services.enabled && config.services.auto_start {
        services.push(ServiceChange {
            name: "project services".to_string(),
            action: "start".to_string(),
            backend: if config.services.compose_file.is_some() {
                "docker-compose".to_string()
            } else if config.services.tilt_file.is_some() {
                "tilt".to_string()
            } else {
                "auto-detect".to_string()
            },
        });
    }

    DiffResult {
        to_install,
        to_update,
        satisfied,
        unknown,
        hooks_to_run,
        services,
    }
}

fn get_installed_version(command: &str) -> Option<String> {
    for flag in ["--version", "-v", "-V", "version"] {
        if let Ok(output) = std::process::Command::new(command).arg(flag).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}{}", stdout, stderr);

                if let Some(version) = extract_version(&combined) {
                    return Some(version);
                }
            }
        }
    }
    None
}

fn extract_version(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"v?(\d+\.\d+(?:\.\d+)?)").ok()?;
    re.captures(text).map(|c| c[1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_result_has_changes() {
        let empty_result = DiffResult {
            to_install: vec![],
            to_update: vec![],
            satisfied: vec![],
            unknown: vec![],
            hooks_to_run: vec![],
            services: vec![],
        };
        assert!(!empty_result.has_changes());

        let result_with_install = DiffResult {
            to_install: vec![ToolChange {
                name: "test".to_string(),
                version: "latest".to_string(),
                current_version: None,
                install_method: None,
            }],
            to_update: vec![],
            satisfied: vec![],
            unknown: vec![],
            hooks_to_run: vec![],
            services: vec![],
        };
        assert!(result_with_install.has_changes());
    }

    #[test]
    fn test_diff_result_action_count() {
        let result = DiffResult {
            to_install: vec![
                ToolChange {
                    name: "a".to_string(),
                    version: "1".to_string(),
                    current_version: None,
                    install_method: None,
                },
                ToolChange {
                    name: "b".to_string(),
                    version: "2".to_string(),
                    current_version: None,
                    install_method: None,
                },
            ],
            to_update: vec![ToolChange {
                name: "c".to_string(),
                version: "3".to_string(),
                current_version: Some("2".to_string()),
                install_method: None,
            }],
            satisfied: vec![],
            unknown: vec![],
            hooks_to_run: vec![],
            services: vec![],
        };
        assert_eq!(result.action_count(), 3);
    }

    #[test]
    fn test_extract_version() {
        assert_eq!(
            extract_version("git version 2.43.0"),
            Some("2.43.0".to_string())
        );
        assert_eq!(extract_version("v20.11.0"), Some("20.11.0".to_string()));
        assert_eq!(
            extract_version("Docker version 24.0.7"),
            Some("24.0.7".to_string())
        );
    }

    #[test]
    fn test_diff_result_to_human_empty() {
        let result = DiffResult {
            to_install: vec![],
            to_update: vec![],
            satisfied: vec![ToolChange {
                name: "git".to_string(),
                version: "latest".to_string(),
                current_version: Some("2.43.0".to_string()),
                install_method: None,
            }],
            unknown: vec![],
            hooks_to_run: vec![],
            services: vec![],
        };
        let output = result.to_human();
        assert!(output.contains("All") && output.contains("satisfied"));
    }
}
