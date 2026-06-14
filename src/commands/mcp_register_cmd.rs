//! `jarvy mcp-register` command handler.

use std::fs;
use std::time::Instant;

use crate::ai_hooks::ConfigOrigin;
use crate::cli::McpRegisterAction;
use crate::config::Config;
use crate::mcp_register::{McpRegisterConfig, McpRegistrationScope, apply, check, remove, runner};
use crate::telemetry;

pub fn run_mcp_register(action: &McpRegisterAction, file: &str) -> i32 {
    match action {
        McpRegisterAction::List {} => run_list(file),
        McpRegisterAction::Apply { scope } => run_apply(scope.as_deref(), file),
        McpRegisterAction::Check { scope } => run_check(scope.as_deref(), file),
        McpRegisterAction::Remove { scope } => run_remove(scope.as_deref(), file),
    }
}

fn run_list(file: &str) -> i32 {
    let Some(cfg) = load_with_scope(file, None) else {
        eprintln!("No [mcp_register] section in {file}");
        return 0;
    };
    println!("MCP registration configuration ({file}):");
    println!("  agents: {:?}", cfg.unique_agents());
    println!("  scope:  {:?}", cfg.scope);
    println!("  allow_custom_servers: {}", cfg.allow_custom_servers);
    println!("  origin: {:?}", cfg.origin);
    println!("  jarvy: built-in (always registered)");
    if !cfg.servers.is_empty() {
        println!("  custom servers:");
        for s in &cfg.servers {
            println!("    - {} ({:?})", s.name, s.transport);
        }
    }
    let refused = runner::audit_custom_servers(&cfg);
    if !refused.is_empty() {
        println!("\nCustom servers refused (allow_custom_servers = false or remote origin):");
        for r in refused {
            println!("  - {r}");
        }
    }
    0
}

fn run_apply(scope: Option<&str>, file: &str) -> i32 {
    let Some(cfg) = load_with_scope(file, scope) else {
        eprintln!("No [mcp_register] section in {file}");
        return 0;
    };
    if cfg.is_empty() {
        eprintln!("Nothing to register: no agents configured.");
        return 0;
    }
    let started = Instant::now();
    telemetry::mcp_register_phase_started(
        cfg.unique_agents().len(),
        cfg.servers.len() + 1,
        scope_label(cfg.scope),
    );
    match apply(&cfg) {
        Ok(report) => {
            println!(
                "Registered {} server(s) across {} agent(s).",
                report.total_applied(),
                report.successes.len()
            );
            for o in &report.successes {
                println!(
                    "  {:<13} {} ({} applied)",
                    o.agent,
                    o.path.display(),
                    o.applied
                );
                for w in &o.warnings {
                    println!("      warning: {w}");
                }
                telemetry::mcp_register_agent_applied(o.agent, o.applied, &o.path);
            }
            for (target, e) in &report.failures {
                eprintln!("  {:<13} FAILED ({}): {}", target.slug(), e.kind(), e);
                telemetry::mcp_register_agent_failed(target.slug(), e.kind());
            }
            if !report.refused_custom.is_empty() {
                println!(
                    "\nRefused {} custom server(s) (allow_custom_servers = false):",
                    report.refused_custom.len()
                );
                for r in &report.refused_custom {
                    println!("  - {r}");
                }
            }
            if !report.remote_refused.is_empty() {
                println!(
                    "\nRefused {} custom server(s) from remote-fetched config:",
                    report.remote_refused.len()
                );
                for r in &report.remote_refused {
                    println!("  - {r}");
                }
            }
            telemetry::mcp_register_phase_completed(
                report.total_applied(),
                report.agents_touched(),
                report.refused_custom.len(),
                report.remote_refused.len(),
                report.failures.len(),
                started.elapsed(),
            );
            if report.has_failures() {
                crate::error_codes::HOOK_FAILED
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("mcp-register apply failed: {e}");
            telemetry::mcp_register_agent_failed("global", e.kind());
            crate::error_codes::HOOK_FAILED
        }
    }
}

fn run_check(scope: Option<&str>, file: &str) -> i32 {
    let Some(cfg) = load_with_scope(file, scope) else {
        eprintln!("No [mcp_register] section in {file}");
        return 0;
    };
    let outcomes = check(&cfg);
    let mut drift = false;
    let mut errors = false;
    for r in &outcomes {
        match r {
            Ok(o) => {
                if o.is_clean() {
                    println!("  {:<13} {} OK", o.agent, o.path.display());
                } else {
                    drift = true;
                    println!("  {:<13} {} DRIFT", o.agent, o.path.display());
                    for m in &o.missing {
                        println!("      missing: {m}");
                    }
                    for x in &o.extra_jarvy {
                        println!("      extra jarvy-managed: {x}");
                    }
                }
            }
            Err((agent, e)) => {
                errors = true;
                eprintln!("  {:<13} FAILED ({}): {}", agent.slug(), e.kind(), e);
                telemetry::mcp_register_agent_failed(agent.slug(), e.kind());
            }
        }
    }
    if errors {
        crate::error_codes::HOOK_FAILED
    } else if drift {
        1
    } else {
        0
    }
}

fn run_remove(scope: Option<&str>, file: &str) -> i32 {
    let Some(cfg) = load_with_scope(file, scope) else {
        eprintln!("No [mcp_register] section in {file}");
        return 0;
    };
    let report = remove(&cfg);
    for o in &report.successes {
        println!(
            "  {:<13} {} removed {}",
            o.agent,
            o.path.display(),
            o.removed
        );
    }
    for (target, e) in &report.failures {
        eprintln!("  {:<13} FAILED ({}): {}", target.slug(), e.kind(), e);
        telemetry::mcp_register_agent_failed(target.slug(), e.kind());
    }
    if report.failures.is_empty() {
        0
    } else {
        crate::error_codes::HOOK_FAILED
    }
}

fn load_with_scope(file: &str, scope: Option<&str>) -> Option<McpRegisterConfig> {
    let body = fs::read_to_string(file).ok()?;
    let cfg: Config = toml::from_str(&body).ok()?;
    let mut mcp = cfg.mcp_register?;
    mcp.origin = ConfigOrigin::Local;
    if let Some(s) = scope_from_str(scope) {
        mcp.scope = s;
    }
    Some(mcp)
}

fn scope_from_str(s: Option<&str>) -> Option<McpRegistrationScope> {
    match s {
        Some("user") => Some(McpRegistrationScope::User),
        Some("project") => Some(McpRegistrationScope::Project),
        _ => None,
    }
}

fn scope_label(scope: McpRegistrationScope) -> &'static str {
    match scope {
        McpRegistrationScope::User => "user",
        McpRegistrationScope::Project => "project",
    }
}
