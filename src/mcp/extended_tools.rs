//! Extended MCP tools exposing Jarvy's broader feature surface.
//!
//! Phase 2 of the Jarvy MCP integration: beyond the tool-installer
//! family (list_tools, install_tool, ...), this module wraps the
//! subsystems an AI agent benefits from being able to introspect and
//! drive directly — AI hooks, MCP registration, drift detection, role
//! definitions, services, templates, config validation.
//!
//! Naming convention: every tool here is prefixed `jarvy_` so a
//! cross-server `tools/list` from the agent's perspective shows them
//! grouped with the existing surface.
//!
//! Safety model:
//! - Read-only tools (`*_list`, `*_check`, `*_status`, `*_show`,
//!   `validate_config`) have no rate limiting and run unconditionally.
//! - Mutating tools (`*_apply`, `services_start`, `templates_use`)
//!   default to `dry_run = true` and require confirmation when
//!   `dry_run = false`, mirroring the `install_tool` flow.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::mcp::error::{McpError, McpResult};
use crate::mcp::tools::McpToolDefinition;

/// Tool definitions appended to the main `list_tools()` registration.
pub fn extended_definitions() -> Vec<McpToolDefinition> {
    vec![
        // ---- AI hooks --------------------------------------------------
        def(
            "jarvy_ai_hooks_list",
            "List configured AI hooks in jarvy.toml and the curated built-in library. Use this to understand what guardrails Jarvy can ship to AI coding agents.",
            json!({
                "type": "object",
                "properties": {
                    "config_path": { "type": "string", "description": "Path to jarvy.toml (default: ./jarvy.toml)" },
                    "library": { "type": "boolean", "description": "Show built-in library instead of project config" }
                }
            }),
        ),
        def(
            "jarvy_ai_hooks_check",
            "Detect drift between configured AI hooks and what is currently provisioned in each agent's settings file. Returns per-agent missing + extra-jarvy lists.",
            json!({
                "type": "object",
                "properties": {
                    "config_path": { "type": "string" }
                }
            }),
        ),
        def(
            "jarvy_ai_hooks_apply",
            "Apply the AI hooks configuration. Defaults to dry_run = true so the agent can preview what would change. Set dry_run = false to actually write the agent settings files.",
            json!({
                "type": "object",
                "properties": {
                    "config_path": { "type": "string" },
                    "dry_run": { "type": "boolean", "description": "Preview only (default true)" }
                }
            }),
        ),
        // ---- MCP server registration ---------------------------------
        def(
            "jarvy_mcp_register_list",
            "List MCP servers Jarvy is configured to register with AI agents. Includes the always-on jarvy entry plus any allow-listed custom servers.",
            json!({
                "type": "object",
                "properties": {
                    "config_path": { "type": "string" }
                }
            }),
        ),
        def(
            "jarvy_mcp_register_check",
            "Detect drift between configured MCP server registrations and each agent's on-disk config file.",
            json!({
                "type": "object",
                "properties": {
                    "config_path": { "type": "string" }
                }
            }),
        ),
        def(
            "jarvy_mcp_register_apply",
            "Apply MCP server registrations to every configured agent. Defaults to dry_run = true.",
            json!({
                "type": "object",
                "properties": {
                    "config_path": { "type": "string" },
                    "dry_run": { "type": "boolean" }
                }
            }),
        ),
        // ---- Drift -----------------------------------------------------
        def(
            "jarvy_drift_check",
            "Detect configuration drift in the current project — installed tool versions vs the jarvy.toml baseline state.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": { "type": "string", "description": "Path to the project root (default: cwd)" }
                }
            }),
        ),
        def(
            "jarvy_drift_status",
            "Show the current drift baseline state file (tools tracked, file hashes, last update).",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": { "type": "string" }
                }
            }),
        ),
        // ---- Roles -----------------------------------------------------
        def(
            "jarvy_roles_list",
            "List roles defined in jarvy.toml. Each role bundles a set of tools so heterogeneous teams (frontend, devops, data) can share one config.",
            json!({
                "type": "object",
                "properties": {
                    "config_path": { "type": "string" }
                }
            }),
        ),
        def(
            "jarvy_roles_show",
            "Show full details for a specific role, including tools, inherited parents, and resolved tool list.",
            json!({
                "type": "object",
                "properties": {
                    "config_path": { "type": "string" },
                    "name": { "type": "string", "description": "Role name (e.g. 'frontend')" }
                },
                "required": ["name"]
            }),
        ),
        // ---- Services -------------------------------------------------
        def(
            "jarvy_services_status",
            "Check whether project services (docker-compose, Tilt) are running and which backend is active.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": { "type": "string" }
                }
            }),
        ),
        def(
            "jarvy_services_start",
            "Start project services (docker-compose up / tilt up). Defaults to dry_run = true; preview prints what would run. Pass detach = false to run in the foreground (attached).",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": { "type": "string" },
                    "dry_run": { "type": "boolean", "description": "Preview only (default true)" },
                    "detach": { "type": "boolean", "description": "Run detached / in background (default true)" }
                }
            }),
        ),
        // ---- Templates ------------------------------------------------
        def(
            "jarvy_templates_list",
            "List built-in jarvy.toml templates (node-bun, python-uv, k8s-platform, etc.) — useful for scaffolding new projects.",
            json!({
                "type": "object",
                "properties": {
                    "category": { "type": "string", "description": "Optional category filter" }
                }
            }),
        ),
        def(
            "jarvy_templates_show",
            "Show full details for a specific built-in template — tools, hooks, env vars, description.",
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            }),
        ),
        def(
            "jarvy_templates_use",
            "Scaffold a jarvy.toml from a built-in template. Defaults to dry_run = true; preview returns the would-be content. Set dry_run = false to write to disk (refuses to overwrite an existing file unless force = true).",
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Template name (run jarvy_templates_list to discover)" },
                    "output_path": { "type": "string", "description": "Where to write (default ./jarvy.toml)" },
                    "dry_run": { "type": "boolean" },
                    "force": { "type": "boolean", "description": "Overwrite an existing file (default false)" }
                },
                "required": ["name"]
            }),
        ),
        // ---- Config validation ----------------------------------------
        def(
            "jarvy_validate_config",
            "Parse and validate jarvy.toml. Returns the structured error list when the file is malformed or refers to unknown tools.",
            json!({
                "type": "object",
                "properties": {
                    "config_path": { "type": "string" }
                }
            }),
        ),
    ]
}

fn def(name: &str, description: &str, schema: Value) -> McpToolDefinition {
    McpToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        input_schema: schema,
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Wrap a JSON value into the MCP tool-call response envelope.
fn envelope(value: Value) -> McpResult<Value> {
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&value)?
        }]
    }))
}

#[derive(Deserialize, Default)]
struct PathArgs {
    #[serde(default)]
    config_path: Option<String>,
    #[serde(default)]
    project_dir: Option<String>,
}

fn config_path(args: &PathArgs) -> String {
    args.config_path
        .clone()
        .unwrap_or_else(|| "./jarvy.toml".to_string())
}

fn project_dir(args: &PathArgs) -> PathBuf {
    args.project_dir
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn parse<P: Default + serde::de::DeserializeOwned>(arguments: Option<Value>) -> McpResult<P> {
    Ok(arguments
        .map(serde_json::from_value)
        .transpose()?
        .unwrap_or_default())
}

// ---- AI hooks --------------------------------------------------------------

#[derive(Deserialize, Default)]
struct AiHooksListArgs {
    #[serde(default)]
    config_path: Option<String>,
    #[serde(default)]
    library: bool,
}

pub fn handle_ai_hooks_list(arguments: Option<Value>) -> McpResult<Value> {
    let args: AiHooksListArgs = parse(arguments)?;
    if args.library {
        let entries: Vec<Value> = crate::ai_hooks::library::LIBRARY
            .iter()
            .map(|h| {
                json!({
                    "name": h.name,
                    "description": h.description,
                    "event": h.event.to_string(),
                    "matcher": h.matcher,
                    "timeout_ms": h.timeout_ms,
                })
            })
            .collect();
        return envelope(json!({ "library": entries, "count": entries.len() }));
    }
    let file = args
        .config_path
        .unwrap_or_else(|| "./jarvy.toml".to_string());
    let Some(cfg) = load_ai_hooks(&file) else {
        return envelope(json!({
            "configured": false,
            "config_path": file,
            "message": "No [ai_hooks] section in config"
        }));
    };
    let refused = crate::ai_hooks::runner::audit_custom_commands(&cfg);
    let hooks: Vec<Value> = cfg
        .hooks
        .iter()
        .map(|h| {
            json!({
                "identifier": h.identifier(),
                "kind": if h.is_library() { "library" } else if h.is_custom_command() { "custom" } else { "invalid" },
            })
        })
        .collect();
    envelope(json!({
        "configured": true,
        "config_path": file,
        "agents": cfg.unique_agents().iter().map(|a| a.slug()).collect::<Vec<_>>(),
        "scope": format!("{:?}", cfg.scope),
        "allow_custom_commands": cfg.allow_custom_commands,
        "origin": format!("{:?}", cfg.origin),
        "hooks": hooks,
        "refused_custom": refused,
    }))
}

pub fn handle_ai_hooks_check(arguments: Option<Value>) -> McpResult<Value> {
    let args: PathArgs = parse(arguments)?;
    let file = config_path(&args);
    let Some(cfg) = load_ai_hooks(&file) else {
        return envelope(json!({ "configured": false, "config_path": file }));
    };
    let outcomes = crate::ai_hooks::check(&cfg);
    let mut report = Vec::with_capacity(outcomes.len());
    let mut drifted = 0usize;
    let mut errored = 0usize;
    for r in outcomes {
        match r {
            Ok(o) => {
                if !o.is_clean() {
                    drifted += 1;
                }
                report.push(json!({
                    "agent": o.agent,
                    "path": o.path.display().to_string(),
                    "clean": o.is_clean(),
                    "missing": o.missing,
                    "extra_jarvy": o.extra_jarvy,
                }));
            }
            Err((agent, e)) => {
                errored += 1;
                report.push(json!({
                    "agent": agent.slug(),
                    "error_type": e.kind(),
                }));
            }
        }
    }
    envelope(json!({
        "configured": true,
        "config_path": file,
        "agents_checked": report.len(),
        "drifted": drifted,
        "errored": errored,
        "report": report,
    }))
}

#[derive(Deserialize, Default)]
struct ApplyArgs {
    #[serde(default)]
    config_path: Option<String>,
    #[serde(default)]
    dry_run: Option<bool>,
}

pub fn handle_ai_hooks_apply(arguments: Option<Value>) -> McpResult<Value> {
    let args: ApplyArgs = parse(arguments)?;
    let file = args
        .config_path
        .unwrap_or_else(|| "./jarvy.toml".to_string());
    let Some(cfg) = load_ai_hooks(&file) else {
        return envelope(json!({ "configured": false, "config_path": file }));
    };
    let dry_run = args.dry_run.unwrap_or(true);
    if dry_run {
        let refused = crate::ai_hooks::runner::audit_custom_commands(&cfg);
        return envelope(json!({
            "dry_run": true,
            "would_apply_hooks": cfg.hooks.len(),
            "would_target_agents": cfg.unique_agents().iter().map(|a| a.slug()).collect::<Vec<_>>(),
            "would_refuse_custom": refused,
            "notes": "Set dry_run to false to actually write agent settings files. Mutating changes go through the host's stderr confirmation flow.",
        }));
    }
    match crate::ai_hooks::apply(&cfg) {
        Ok(report) => envelope(json!({
            "dry_run": false,
            "applied": report.total_applied(),
            "agents_touched": report.agents_touched(),
            "successes": report.successes.iter().map(|o| json!({
                "agent": o.agent,
                "path": o.path.display().to_string(),
                "applied": o.applied,
            })).collect::<Vec<_>>(),
            "failures": report.failures.iter().map(|(t, e)| json!({
                "agent": t.slug(),
                "error_type": e.kind(),
            })).collect::<Vec<_>>(),
            "refused_custom": report.refused_custom,
            "remote_refused": report.remote_refused_custom,
        })),
        Err(e) => Err(McpError::internal_error(format!(
            "ai_hooks::apply failed ({}): {e}",
            e.kind()
        ))),
    }
}

fn load_ai_hooks(file: &str) -> Option<crate::ai_hooks::AiHooksConfig> {
    let body = std::fs::read_to_string(file).ok()?;
    let cfg: crate::config::Config = toml::from_str(&body).ok()?;
    let mut ai = cfg.ai_hooks?;
    ai.origin = crate::ai_hooks::ConfigOrigin::Local;
    Some(ai)
}

// ---- MCP register ----------------------------------------------------------

pub fn handle_mcp_register_list(arguments: Option<Value>) -> McpResult<Value> {
    let args: PathArgs = parse(arguments)?;
    let file = config_path(&args);
    let Some(cfg) = load_mcp_register(&file) else {
        return envelope(json!({ "configured": false, "config_path": file }));
    };
    let refused = crate::mcp_register::runner::audit_custom_servers(&cfg);
    envelope(json!({
        "configured": true,
        "config_path": file,
        "agents": cfg.unique_agents().iter().map(|a| a.slug()).collect::<Vec<_>>(),
        "scope": format!("{:?}", cfg.scope),
        "allow_custom_servers": cfg.allow_custom_servers,
        "origin": format!("{:?}", cfg.origin),
        "jarvy_server": "built-in (always registered)",
        "custom_servers": cfg.servers.iter().map(|s| json!({
            "name": s.name,
            "transport": format!("{:?}", s.transport),
        })).collect::<Vec<_>>(),
        "refused_custom": refused,
    }))
}

pub fn handle_mcp_register_check(arguments: Option<Value>) -> McpResult<Value> {
    let args: PathArgs = parse(arguments)?;
    let file = config_path(&args);
    let Some(cfg) = load_mcp_register(&file) else {
        return envelope(json!({ "configured": false, "config_path": file }));
    };
    let outcomes = crate::mcp_register::check(&cfg);
    let mut report = Vec::with_capacity(outcomes.len());
    let mut drifted = 0usize;
    let mut errored = 0usize;
    for r in outcomes {
        match r {
            Ok(o) => {
                if !o.is_clean() {
                    drifted += 1;
                }
                report.push(json!({
                    "agent": o.agent,
                    "path": o.path.display().to_string(),
                    "clean": o.is_clean(),
                    "missing": o.missing,
                    "extra_jarvy": o.extra_jarvy,
                }));
            }
            Err((agent, e)) => {
                errored += 1;
                report.push(json!({ "agent": agent.slug(), "error_type": e.kind() }));
            }
        }
    }
    envelope(json!({
        "configured": true,
        "config_path": file,
        "agents_checked": report.len(),
        "drifted": drifted,
        "errored": errored,
        "report": report,
    }))
}

pub fn handle_mcp_register_apply(arguments: Option<Value>) -> McpResult<Value> {
    let args: ApplyArgs = parse(arguments)?;
    let file = args
        .config_path
        .unwrap_or_else(|| "./jarvy.toml".to_string());
    let Some(cfg) = load_mcp_register(&file) else {
        return envelope(json!({ "configured": false, "config_path": file }));
    };
    let dry_run = args.dry_run.unwrap_or(true);
    if dry_run {
        return envelope(json!({
            "dry_run": true,
            "would_register_servers": cfg.servers.len() + 1,
            "would_target_agents": cfg.unique_agents().iter().map(|a| a.slug()).collect::<Vec<_>>(),
            "notes": "Set dry_run to false to actually write agent MCP config files.",
        }));
    }
    match crate::mcp_register::apply(&cfg) {
        Ok(report) => envelope(json!({
            "dry_run": false,
            "applied": report.total_applied(),
            "agents_touched": report.agents_touched(),
            "successes": report.successes.iter().map(|o| json!({
                "agent": o.agent,
                "path": o.path.display().to_string(),
                "applied": o.applied,
            })).collect::<Vec<_>>(),
            "failures": report.failures.iter().map(|(t, e)| json!({
                "agent": t.slug(),
                "error_type": e.kind(),
            })).collect::<Vec<_>>(),
            "refused_custom": report.refused_custom,
            "remote_refused": report.remote_refused,
        })),
        Err(e) => Err(McpError::internal_error(format!(
            "mcp_register::apply failed ({}): {e}",
            e.kind()
        ))),
    }
}

fn load_mcp_register(file: &str) -> Option<crate::mcp_register::McpRegisterConfig> {
    let body = std::fs::read_to_string(file).ok()?;
    let cfg: crate::config::Config = toml::from_str(&body).ok()?;
    let mut mcp = cfg.mcp_register?;
    mcp.origin = crate::ai_hooks::ConfigOrigin::Local;
    Some(mcp)
}

// ---- Drift -----------------------------------------------------------------

pub fn handle_drift_check(arguments: Option<Value>) -> McpResult<Value> {
    let args: PathArgs = parse(arguments)?;
    let dir = project_dir(&args);
    let state_path = crate::paths::state_json(&dir);
    if !state_path.exists() {
        return envelope(json!({
            "baseline_exists": false,
            "project_dir": dir.display().to_string(),
            "message": "No drift baseline at .jarvy/state.json. Run `jarvy setup` first to capture one.",
        }));
    }
    // Read state, compare to current tool inventory. We surface the raw
    // baseline tool count + a sample so the agent can decide whether to
    // shell out to `jarvy drift check` for a full report.
    match crate::drift::state::EnvironmentState::load(&dir) {
        Ok(Some(state)) => envelope(json!({
            "baseline_exists": true,
            "project_dir": dir.display().to_string(),
            "tool_count": state.tool_count(),
            "files_tracked": state.file_count(),
            "notes": "Run `jarvy drift check` for the full per-tool comparison.",
        })),
        Ok(None) => envelope(json!({ "baseline_exists": false })),
        Err(e) => Err(McpError::internal_error(format!(
            "drift state load failed: {e}"
        ))),
    }
}

pub fn handle_drift_status(arguments: Option<Value>) -> McpResult<Value> {
    let args: PathArgs = parse(arguments)?;
    let dir = project_dir(&args);
    match crate::drift::state::EnvironmentState::load(&dir) {
        Ok(Some(state)) => envelope(json!({
            "baseline_exists": true,
            "project_dir": dir.display().to_string(),
            "tool_count": state.tool_count(),
            "files_tracked": state.file_count(),
        })),
        Ok(None) => envelope(json!({
            "baseline_exists": false,
            "project_dir": dir.display().to_string(),
        })),
        Err(e) => Err(McpError::internal_error(format!(
            "drift status load failed: {e}"
        ))),
    }
}

// ---- Roles -----------------------------------------------------------------

pub fn handle_roles_list(arguments: Option<Value>) -> McpResult<Value> {
    let args: PathArgs = parse(arguments)?;
    let file = config_path(&args);
    let Some(roles) = load_roles(&file) else {
        return envelope(json!({ "configured": false, "config_path": file }));
    };
    let entries: Vec<Value> = roles
        .iter()
        .map(|(name, def)| {
            json!({
                "name": name,
                "description": def.description,
                "extends": def.get_extends(),
                "tool_count": def.tool_count(),
            })
        })
        .collect();
    envelope(json!({
        "configured": true,
        "config_path": file,
        "count": entries.len(),
        "roles": entries,
    }))
}

#[derive(Deserialize)]
struct RolesShowArgs {
    name: String,
    #[serde(default)]
    config_path: Option<String>,
}

pub fn handle_roles_show(arguments: Option<Value>) -> McpResult<Value> {
    let args: RolesShowArgs = arguments
        .ok_or_else(|| McpError::invalid_params("Missing role name"))
        .and_then(|v| serde_json::from_value(v).map_err(McpError::from))?;
    let file = args
        .config_path
        .unwrap_or_else(|| "./jarvy.toml".to_string());
    let Some(roles) = load_roles(&file) else {
        return envelope(json!({ "configured": false, "config_path": file }));
    };
    let Some(def) = roles.get(&args.name) else {
        return Err(McpError::invalid_params(format!(
            "Unknown role: {}",
            args.name
        )));
    };
    envelope(json!({
        "name": args.name,
        "description": def.description,
        "extends": def.get_extends(),
        "tools": def.get_tools(),
        "tool_count": def.tool_count(),
    }))
}

fn load_roles(
    file: &str,
) -> Option<std::collections::HashMap<String, crate::roles::definition::RoleDefinition>> {
    let body = std::fs::read_to_string(file).ok()?;
    let cfg: crate::config::Config = toml::from_str(&body).ok()?;
    let mut out = std::collections::HashMap::new();
    for (name, raw) in cfg.roles_config.roles.into_iter() {
        out.insert(name, raw.into_definition());
    }
    if out.is_empty() { None } else { Some(out) }
}

// ---- Services --------------------------------------------------------------

pub fn handle_services_status(arguments: Option<Value>) -> McpResult<Value> {
    let args: PathArgs = parse(arguments)?;
    let dir = project_dir(&args);
    let Some((backend, config_path)) = crate::services::detect_backend(&dir) else {
        return envelope(json!({
            "backend": null,
            "project_dir": dir.display().to_string(),
            "message": "No service backend detected (no docker-compose.yml / Tiltfile in project).",
        }));
    };
    let backend_impl = crate::services::get_backend(backend);
    envelope(json!({
        "backend": format!("{:?}", backend),
        "config_path": config_path.display().to_string(),
        "installed": backend_impl.is_installed(),
        "project_dir": dir.display().to_string(),
    }))
}

#[derive(Deserialize, Default)]
struct ServicesStartArgs {
    #[serde(default)]
    project_dir: Option<String>,
    #[serde(default)]
    dry_run: Option<bool>,
    #[serde(default)]
    detach: Option<bool>,
}

pub fn handle_services_start(arguments: Option<Value>) -> McpResult<Value> {
    let args: ServicesStartArgs = parse(arguments)?;
    let dir = args
        .project_dir
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let dry_run = args.dry_run.unwrap_or(true);
    let detach = args.detach.unwrap_or(true);
    let Some((backend, config_path)) = crate::services::detect_backend(&dir) else {
        return envelope(json!({
            "started": false,
            "project_dir": dir.display().to_string(),
            "message": "No docker-compose / Tilt config detected — nothing to start.",
        }));
    };
    let backend_impl = crate::services::get_backend(backend);
    if !backend_impl.is_installed() {
        return envelope(json!({
            "started": false,
            "backend": format!("{:?}", backend),
            "config_path": config_path.display().to_string(),
            "installed": false,
            "message": "Backend is not installed on this machine; run `jarvy setup` first.",
        }));
    }
    if dry_run {
        return envelope(json!({
            "dry_run": true,
            "backend": format!("{:?}", backend),
            "config_path": config_path.display().to_string(),
            "detach": detach,
            "notes": "Set dry_run to false to actually start. Mutating ops go through the host's stderr confirmation flow.",
        }));
    }
    match backend_impl.start(&config_path, detach) {
        Ok(result) => envelope(json!({
            "dry_run": false,
            "backend": format!("{:?}", result.backend),
            "config_path": config_path.display().to_string(),
            "success": result.success,
            "message": result.message,
        })),
        Err(e) => Err(McpError::internal_error(format!(
            "services::start failed: {e}"
        ))),
    }
}

// ---- Templates -------------------------------------------------------------

#[derive(Deserialize, Default)]
struct TemplatesListArgs {
    #[serde(default)]
    category: Option<String>,
}

pub fn handle_templates_list(arguments: Option<Value>) -> McpResult<Value> {
    let args: TemplatesListArgs = parse(arguments)?;
    let all = crate::templates::builtin::list_builtin_templates();
    let filtered: Vec<&crate::templates::builtin::BuiltinTemplate> = match args.category {
        Some(ref c) => all
            .iter()
            .filter(|t| t.category.eq_ignore_ascii_case(c))
            .collect(),
        None => all.iter().collect(),
    };
    let entries: Vec<Value> = filtered
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "category": t.category,
            })
        })
        .collect();
    envelope(json!({
        "count": entries.len(),
        "categories": crate::templates::builtin::all_categories(),
        "templates": entries,
    }))
}

#[derive(Deserialize)]
struct TemplatesShowArgs {
    name: String,
}

pub fn handle_templates_show(arguments: Option<Value>) -> McpResult<Value> {
    let args: TemplatesShowArgs = arguments
        .ok_or_else(|| McpError::invalid_params("Missing template name"))
        .and_then(|v| serde_json::from_value(v).map_err(McpError::from))?;
    let Some(template) = crate::templates::builtin::get_builtin_template(&args.name) else {
        return Err(McpError::invalid_params(format!(
            "Unknown template: {}",
            args.name
        )));
    };
    let full = template.to_template();
    envelope(json!({
        "name": template.name,
        "description": template.description,
        "category": template.category,
        "tools": full.tools.tools,
        "meta": full.template,
    }))
}

#[derive(Deserialize)]
struct TemplatesUseArgs {
    name: String,
    #[serde(default)]
    output_path: Option<String>,
    #[serde(default)]
    dry_run: Option<bool>,
    #[serde(default)]
    force: Option<bool>,
}

pub fn handle_templates_use(arguments: Option<Value>) -> McpResult<Value> {
    let args: TemplatesUseArgs = arguments
        .ok_or_else(|| McpError::invalid_params("Missing template name"))
        .and_then(|v| serde_json::from_value(v).map_err(McpError::from))?;
    let Some(template) = crate::templates::builtin::get_builtin_template(&args.name) else {
        return Err(McpError::invalid_params(format!(
            "Unknown template: {}",
            args.name
        )));
    };
    let dry_run = args.dry_run.unwrap_or(true);
    let force = args.force.unwrap_or(false);
    let output = args
        .output_path
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("jarvy.toml"));
    let content = template.to_template().to_jarvy_toml();
    if dry_run {
        return envelope(json!({
            "dry_run": true,
            "template": args.name,
            "output_path": output.display().to_string(),
            "tool_count": template.tools.len(),
            "would_overwrite": output.exists(),
            "content_preview": content,
        }));
    }
    if output.exists() && !force {
        return envelope(json!({
            "dry_run": false,
            "created": false,
            "output_path": output.display().to_string(),
            "error": "file already exists; pass force = true to overwrite",
        }));
    }
    match std::fs::write(&output, &content) {
        Ok(()) => envelope(json!({
            "dry_run": false,
            "created": true,
            "template": args.name,
            "output_path": output.display().to_string(),
            "tool_count": template.tools.len(),
            "bytes_written": content.len(),
        })),
        Err(e) => Err(McpError::internal_error(format!(
            "templates::use write failed: {e}"
        ))),
    }
}

// ---- Config validation -----------------------------------------------------

pub fn handle_validate_config(arguments: Option<Value>) -> McpResult<Value> {
    let args: PathArgs = parse(arguments)?;
    let file = config_path(&args);
    let path = Path::new(&file);
    if !path.exists() {
        return envelope(json!({
            "valid": false,
            "config_path": file,
            "error_type": "missing",
            "message": format!("File not found: {file}"),
        }));
    }
    let body = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return envelope(json!({
                "valid": false,
                "config_path": file,
                "error_type": "io",
                "message": e.to_string(),
            }));
        }
    };
    match toml::from_str::<crate::config::Config>(&body) {
        Ok(cfg) => envelope(json!({
            "valid": true,
            "config_path": file,
            "tool_count": cfg.tool_configs_len(),
            "has_ai_hooks": cfg.ai_hooks.is_some(),
            "has_mcp_register": cfg.mcp_register.is_some(),
            "has_git": cfg.git.is_some(),
            "has_npm": cfg.npm.is_some(),
            "has_pip": cfg.pip.is_some(),
            "has_cargo": cfg.cargo.is_some(),
            "has_drift": cfg.drift.is_some(),
        })),
        Err(e) => envelope(json!({
            "valid": false,
            "config_path": file,
            "error_type": "parse",
            "message": e.to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn ai_hooks_list_library_returns_curated_set() {
        let resp = handle_ai_hooks_list(Some(json!({ "library": true }))).unwrap();
        let text = resp["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("block-rm-rf"));
        assert!(text.contains("audit-log"));
    }

    #[test]
    fn ai_hooks_list_returns_not_configured_for_missing_file() {
        let resp =
            handle_ai_hooks_list(Some(json!({ "config_path": "/nonexistent.toml" }))).unwrap();
        let text = resp["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"configured\": false"));
    }

    #[test]
    fn validate_config_reports_missing_file() {
        let resp = handle_validate_config(Some(json!({ "config_path": "/nope.toml" }))).unwrap();
        let text = resp["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"valid\": false"));
        assert!(text.contains("missing"));
    }

    #[test]
    fn validate_config_parses_minimal_config() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("jarvy.toml");
        std::fs::write(
            &p,
            r#"[provisioner]
git = "latest"
"#,
        )
        .unwrap();
        let resp = handle_validate_config(Some(json!({
            "config_path": p.to_str().unwrap()
        })))
        .unwrap();
        let text = resp["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"valid\": true"));
        assert!(text.contains("\"tool_count\": 1"));
    }

    #[test]
    fn templates_list_returns_built_in_templates() {
        let resp = handle_templates_list(None).unwrap();
        let text = resp["content"][0]["text"].as_str().unwrap();
        // At least one well-known template should be present.
        assert!(text.contains("templates"));
        assert!(text.contains("\"count\":"));
    }

    #[test]
    fn drift_status_reports_no_baseline_when_absent() {
        let dir = TempDir::new().unwrap();
        let resp = handle_drift_status(Some(json!({
            "project_dir": dir.path().to_str().unwrap()
        })))
        .unwrap();
        let text = resp["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"baseline_exists\": false"));
    }

    #[test]
    fn services_status_reports_no_backend_in_empty_dir() {
        let dir = TempDir::new().unwrap();
        let resp = handle_services_status(Some(json!({
            "project_dir": dir.path().to_str().unwrap()
        })))
        .unwrap();
        let text = resp["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"backend\": null"));
    }

    #[test]
    fn services_start_in_empty_dir_reports_not_started() {
        let dir = TempDir::new().unwrap();
        let resp = handle_services_start(Some(json!({
            "project_dir": dir.path().to_str().unwrap()
        })))
        .unwrap();
        let text = resp["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"started\": false"));
    }

    #[test]
    fn templates_use_dry_run_returns_preview_without_writing() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("jarvy.toml");
        // Pick any name that's guaranteed in the built-in registry.
        let any_name = crate::templates::builtin::list_builtin_templates()
            .first()
            .map(|t| t.name.to_string())
            .expect("at least one built-in template");
        let resp = handle_templates_use(Some(json!({
            "name": any_name,
            "output_path": out.to_str().unwrap(),
            "dry_run": true
        })))
        .unwrap();
        let text = resp["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"dry_run\": true"));
        assert!(text.contains("\"content_preview\""));
        // No write should have occurred.
        assert!(!out.exists());
    }

    #[test]
    fn templates_use_refuses_to_overwrite_without_force() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("jarvy.toml");
        std::fs::write(&out, b"existing").unwrap();
        let any_name = crate::templates::builtin::list_builtin_templates()
            .first()
            .map(|t| t.name.to_string())
            .unwrap();
        let resp = handle_templates_use(Some(json!({
            "name": any_name,
            "output_path": out.to_str().unwrap(),
            "dry_run": false,
            "force": false
        })))
        .unwrap();
        let text = resp["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"created\": false"));
        assert!(text.contains("already exists"));
        // Existing file untouched.
        assert_eq!(std::fs::read(&out).unwrap(), b"existing");
    }

    #[test]
    fn templates_use_writes_when_force_is_set() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("jarvy.toml");
        std::fs::write(&out, b"existing").unwrap();
        let any_name = crate::templates::builtin::list_builtin_templates()
            .first()
            .map(|t| t.name.to_string())
            .unwrap();
        let resp = handle_templates_use(Some(json!({
            "name": any_name,
            "output_path": out.to_str().unwrap(),
            "dry_run": false,
            "force": true
        })))
        .unwrap();
        let text = resp["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"created\": true"));
        let body = std::fs::read_to_string(&out).unwrap();
        assert!(body.contains("[provisioner]") || body.contains("provisioner"));
    }

    #[test]
    fn templates_use_unknown_template_returns_error() {
        let resp = handle_templates_use(Some(json!({
            "name": "definitely-not-a-real-template"
        })));
        let err = resp.unwrap_err();
        assert!(err.to_string().contains("Unknown template"));
    }
}
