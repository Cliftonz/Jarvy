//! CLI Commands for Role Management
//!
//! Implements:
//! - `jarvy roles list` - List all available roles
//! - `jarvy roles show <name>` - Show role details
//! - `jarvy roles diff <a> <b>` - Compare two roles

use super::definition::RolesConfig;
use super::resolver::{RoleDiff, RoleResolver, diff_roles};
use clap::Subcommand;
use std::collections::HashMap;

/// Subcommands for `jarvy roles`
#[derive(Clone, Subcommand)]
pub enum RolesAction {
    /// List all available roles
    List {
        /// Show detailed output including tool counts
        #[clap(short, long)]
        verbose: bool,
        /// Output format: json, pretty
        #[clap(short = 'F', long = "format", default_value = "pretty")]
        output_format: String,
    },
    /// Show details for a specific role
    Show {
        /// Role name to show
        name: String,
        /// Show inheritance chain
        #[clap(long)]
        inheritance: bool,
        /// Show resolved tools (including inherited)
        #[clap(long)]
        resolved: bool,
        /// Output format: json, pretty
        #[clap(short = 'F', long = "format", default_value = "pretty")]
        output_format: String,
    },
    /// Compare two roles
    Diff {
        /// First role name
        role_a: String,
        /// Second role name (or --current to compare with assigned role)
        role_b: Option<String>,
        /// Compare with currently assigned role
        #[clap(long)]
        current: bool,
        /// Output format: json, pretty
        #[clap(short = 'F', long = "format", default_value = "pretty")]
        output_format: String,
    },
}

/// Handle the roles subcommand
pub fn handle_roles_command(
    action: RolesAction,
    roles_config: Option<&RolesConfig>,
    current_role: Option<&str>,
) -> Result<(), String> {
    let config = roles_config.ok_or("No roles defined in configuration")?;

    if config.roles.is_empty() {
        println!("No roles defined in configuration.");
        println!("\nTo define roles, add [roles.name] sections to your jarvy.toml:");
        println!();
        println!("  [roles.frontend]");
        println!("  description = \"Frontend development stack\"");
        println!("  tools = [\"node\", \"bun\", \"pnpm\"]");
        println!();
        println!("  [roles.frontend.tools]");
        println!("  node = \"20\"");
        return Ok(());
    }

    match action {
        RolesAction::List {
            verbose,
            output_format,
        } => handle_list(config, verbose, &output_format),
        RolesAction::Show {
            name,
            inheritance,
            resolved,
            output_format,
        } => handle_show(config, &name, inheritance, resolved, &output_format),
        RolesAction::Diff {
            role_a,
            role_b,
            current,
            output_format,
        } => handle_diff(
            config,
            &role_a,
            role_b.as_deref(),
            current,
            current_role,
            &output_format,
        ),
    }
}

fn handle_list(config: &RolesConfig, verbose: bool, output_format: &str) -> Result<(), String> {
    let resolver = RoleResolver::new(config);
    let mut roles: Vec<_> = resolver.list_roles().into_iter().collect();
    roles.sort();

    if output_format == "json" {
        let json_output: Vec<_> = roles
            .iter()
            .map(|name| {
                let role = resolver.get_role(name).unwrap();
                let mut obj = serde_json::json!({
                    "name": name,
                    "description": role.description(),
                    "extends": if role.has_extends() {
                        Some(role.get_extends())
                    } else {
                        None
                    },
                });
                if verbose {
                    let def = role.clone().into_definition();
                    obj["tool_count"] = serde_json::json!(def.tool_count());
                }
                obj
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
    } else {
        println!("Available roles:\n");
        for name in &roles {
            let role = resolver.get_role(name).unwrap();
            let desc = role
                .description()
                .map(|d| format!(" - {}", d))
                .unwrap_or_default();

            let extends_info = if role.has_extends() {
                format!(" (extends: {})", role.get_extends().join(", "))
            } else {
                String::new()
            };

            if verbose {
                let def = role.clone().into_definition();
                println!(
                    "  {} ({} tools){}\n    {}",
                    name,
                    def.tool_count(),
                    extends_info,
                    if desc.is_empty() {
                        "(no description)".to_string()
                    } else {
                        desc.trim_start_matches(" - ").to_string()
                    }
                );
            } else {
                println!("  {}{}{}", name, extends_info, desc);
            }
        }
        println!();
        println!("Use 'jarvy roles show <name>' for details on a specific role.");
    }

    Ok(())
}

fn handle_show(
    config: &RolesConfig,
    name: &str,
    show_inheritance: bool,
    show_resolved: bool,
    output_format: &str,
) -> Result<(), String> {
    let mut resolver = RoleResolver::new(config);

    // Get unresolved definition first and clone to avoid borrow conflict
    let role_def = resolver
        .get_role(name)
        .ok_or_else(|| format!("Role '{}' not found", name))?
        .clone();

    // Get resolved version if needed
    let resolved = resolver.resolve(name).map_err(|e| e.to_string())?;

    if output_format == "json" {
        let def = role_def.clone().into_definition();
        let mut json_output = serde_json::json!({
            "name": name,
            "description": resolved.description,
            "extends": if role_def.has_extends() {
                Some(role_def.get_extends())
            } else {
                None
            },
            "direct_tools": def.get_tools(),
        });

        if show_inheritance || show_resolved {
            json_output["inheritance_chain"] = serde_json::json!(resolved.inheritance_chain);
        }

        if show_resolved {
            let resolved_tools: HashMap<_, _> = resolved
                .tools
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        serde_json::json!({
                            "version": v.version,
                            "source": v.source_role,
                        }),
                    )
                })
                .collect();
            json_output["resolved_tools"] = serde_json::json!(resolved_tools);
        }

        println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
    } else {
        println!("Role: {}\n", name);

        if let Some(desc) = &resolved.description {
            println!("  Description: {}\n", desc);
        }

        if role_def.has_extends() {
            println!("  Extends: {}\n", role_def.get_extends().join(", "));
        }

        if show_inheritance {
            println!(
                "  Inheritance chain: {}\n",
                resolved.inheritance_chain.join(" -> ")
            );
        }

        let def = role_def.clone().into_definition();
        let direct_tools = def.get_tools();

        if !direct_tools.is_empty() {
            println!("  Direct tools ({}):", direct_tools.len());
            let mut tools: Vec<_> = direct_tools.iter().collect();
            tools.sort_by_key(|(name, _)| *name);
            for (tool_name, version) in tools {
                println!("    {} = {}", tool_name, version);
            }
            println!();
        }

        if show_resolved {
            println!("  Resolved tools ({} total):", resolved.tools.len());
            let mut tools: Vec<_> = resolved.tools.iter().collect();
            tools.sort_by_key(|(name, _)| *name);
            for (tool_name, tool) in tools {
                let source = if tool.source_role != name {
                    format!(" (from {})", tool.source_role)
                } else {
                    String::new()
                };
                println!("    {} = {}{}", tool_name, tool.version, source);
            }
        }
    }

    Ok(())
}

fn handle_diff(
    config: &RolesConfig,
    role_a: &str,
    role_b: Option<&str>,
    use_current: bool,
    current_role: Option<&str>,
    output_format: &str,
) -> Result<(), String> {
    let mut resolver = RoleResolver::new(config);

    // Determine second role
    let role_b_name = if use_current {
        current_role.ok_or("No role currently assigned. Use 'role = \"name\"' in jarvy.toml")?
    } else {
        role_b.ok_or("Please specify a second role or use --current")?
    };

    // Resolve both roles
    let resolved_a = resolver.resolve(role_a).map_err(|e| e.to_string())?;
    let resolved_b = resolver.resolve(role_b_name).map_err(|e| e.to_string())?;

    let diff = diff_roles(&resolved_a, &resolved_b);

    if output_format == "json" {
        let json_output = serde_json::json!({
            "role_a": diff.role_a,
            "role_b": diff.role_b,
            "is_identical": diff.is_identical(),
            "only_in_a": diff.only_in_a,
            "only_in_b": diff.only_in_b,
            "different_versions": diff.different_versions,
            "same": diff.same,
        });
        println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
    } else {
        print_diff_pretty(&diff);
    }

    Ok(())
}

fn print_diff_pretty(diff: &RoleDiff) {
    println!("Comparing roles: {} vs {}\n", diff.role_a, diff.role_b);

    if diff.is_identical() {
        println!("  Roles are identical.\n");
        return;
    }

    if !diff.only_in_a.is_empty() {
        println!("  Only in {}:", diff.role_a);
        let mut tools: Vec<_> = diff.only_in_a.iter().collect();
        tools.sort_by_key(|(name, _)| *name);
        for (name, version) in tools {
            println!("    - {} = {}", name, version);
        }
        println!();
    }

    if !diff.only_in_b.is_empty() {
        println!("  Only in {}:", diff.role_b);
        let mut tools: Vec<_> = diff.only_in_b.iter().collect();
        tools.sort_by_key(|(name, _)| *name);
        for (name, version) in tools {
            println!("    + {} = {}", name, version);
        }
        println!();
    }

    if !diff.different_versions.is_empty() {
        println!("  Different versions:");
        let mut tools: Vec<_> = diff.different_versions.iter().collect();
        tools.sort_by_key(|(name, _)| *name);
        for (name, (ver_a, ver_b)) in tools {
            println!("    {} : {} -> {}", name, ver_a, ver_b);
        }
        println!();
    }

    if !diff.same.is_empty() {
        println!("  Same in both ({} tools):", diff.same.len());
        let mut tools: Vec<_> = diff.same.keys().collect();
        tools.sort();
        let display: Vec<_> = tools.iter().take(5).map(|s| s.as_str()).collect();
        let remaining = tools.len().saturating_sub(5);
        print!("    {}", display.join(", "));
        if remaining > 0 {
            print!(" (+{} more)", remaining);
        }
        println!("\n");
    }

    println!(
        "Summary: {} differences ({} only in {}, {} only in {}, {} version differences)",
        diff.difference_count(),
        diff.only_in_a.len(),
        diff.role_a,
        diff.only_in_b.len(),
        diff.role_b,
        diff.different_versions.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roles::definition::{RoleDefinition, RoleDefinitionWrapper};

    fn create_test_config() -> RolesConfig {
        let mut roles = HashMap::new();

        let mut frontend = RoleDefinition::default();
        frontend.description = Some("Frontend tools".to_string());
        frontend.tools = vec!["node".to_string(), "bun".to_string()];
        roles.insert(
            "frontend".to_string(),
            RoleDefinitionWrapper::Simple(frontend),
        );

        let mut backend = RoleDefinition::default();
        backend.description = Some("Backend tools".to_string());
        backend.tools = vec!["rust".to_string(), "go".to_string()];
        roles.insert(
            "backend".to_string(),
            RoleDefinitionWrapper::Simple(backend),
        );

        RolesConfig { roles }
    }

    #[test]
    fn test_handle_list() {
        let config = create_test_config();
        let result = handle_list(&config, false, "pretty");
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_show() {
        let config = create_test_config();
        let result = handle_show(&config, "frontend", false, false, "pretty");
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_show_nonexistent() {
        let config = create_test_config();
        let result = handle_show(&config, "nonexistent", false, false, "pretty");
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_diff() {
        let config = create_test_config();
        let result = handle_diff(&config, "frontend", Some("backend"), false, None, "pretty");
        assert!(result.is_ok());
    }
}
