//! Cline registrar (VS Code globalStorage, user-scope only).
//!
//! Cline lives as a VS Code extension; the canonical MCP config path is:
//!
//! - macOS:   `~/Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json`
//! - Linux:   `~/.config/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json`
//! - Windows: `%APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\settings\cline_mcp_settings.json`
//!
//! No project-scope as of mid-2026 (tracked in cline/cline#2418).

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::{Map, Value, json};

use super::{AgentRegistrar, ApplyOutcome, CheckOutcome, RemoveOutcome, ResolvedServer};
use crate::ai_hooks::agents::io::{home_or_err, read_or_default_object, write_json};
use crate::mcp_register::config::{McpRegistrationScope, McpServerTransport};
use crate::mcp_register::error::McpRegisterError;

pub struct ClineRegistrar;

impl ClineRegistrar {
    const SLUG: &'static str = "cline";
    const MARKER_KEY: &'static str = "_jarvy_managed_servers";
    const SERVERS_KEY: &'static str = "mcpServers";

    fn vscode_global_storage() -> Result<PathBuf, McpRegisterError> {
        let home = home_or_err()?;
        let base = if cfg!(target_os = "macos") {
            home.join("Library")
                .join("Application Support")
                .join("Code")
        } else if cfg!(target_os = "windows") {
            // %APPDATA% — fall back to %USERPROFILE%\AppData\Roaming.
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join("AppData").join("Roaming"))
                .join("Code")
        } else {
            home.join(".config").join("Code")
        };
        Ok(base
            .join("User")
            .join("globalStorage")
            .join("saoudrizwan.claude-dev")
            .join("settings")
            .join("cline_mcp_settings.json"))
    }
}

impl AgentRegistrar for ClineRegistrar {
    fn slug(&self) -> &'static str {
        Self::SLUG
    }

    fn settings_path(&self, _scope: McpRegistrationScope) -> Result<PathBuf, McpRegisterError> {
        Self::vscode_global_storage()
    }

    fn apply(
        &self,
        servers: &[ResolvedServer],
        scope: McpRegistrationScope,
    ) -> Result<ApplyOutcome, McpRegisterError> {
        let path = self.settings_path(scope)?;
        let mut root = read_or_default_object(&path)?;
        let mut managed = current_managed_names(&root);
        let mcp_servers = root
            .entry(Self::SERVERS_KEY)
            .or_insert_with(|| Value::Object(Map::new()));
        let Value::Object(mcp_obj) = mcp_servers else {
            return Err(McpRegisterError::InvalidEntry {
                name: Self::SERVERS_KEY.to_string(),
                reason: format!("existing `{}` is not an object", Self::SERVERS_KEY),
            });
        };
        let mut applied = 0usize;
        let desired: BTreeSet<String> = servers.iter().map(|s| s.name.clone()).collect();
        for name in managed.difference(&desired).cloned().collect::<Vec<_>>() {
            mcp_obj.remove(&name);
            managed.remove(&name);
        }
        for server in servers {
            mcp_obj.insert(server.name.clone(), to_server_value(server));
            managed.insert(server.name.clone());
            applied += 1;
        }
        let names: Vec<Value> = managed.iter().map(|n| Value::String(n.clone())).collect();
        root.insert(Self::MARKER_KEY.to_string(), Value::Array(names));
        write_json(&path, &Value::Object(root))?;

        let mut warnings = Vec::new();
        if scope == McpRegistrationScope::Project {
            warnings.push(
                "cline does not support project-scope MCP config; wrote to user scope instead"
                    .to_string(),
            );
        }
        Ok(ApplyOutcome {
            agent: Self::SLUG,
            path,
            applied,
            warnings,
        })
    }

    fn check(
        &self,
        servers: &[ResolvedServer],
        scope: McpRegistrationScope,
    ) -> Result<CheckOutcome, McpRegisterError> {
        let path = self.settings_path(scope)?;
        let root = read_or_default_object(&path)?;
        let mut outcome = CheckOutcome {
            agent: Self::SLUG,
            path,
            ..CheckOutcome::default()
        };
        let mcp = match root.get(Self::SERVERS_KEY) {
            Some(Value::Object(m)) => m,
            _ => {
                outcome.missing = servers.iter().map(|s| s.name.clone()).collect();
                return Ok(outcome);
            }
        };
        let managed = current_managed_names(&root);
        let desired: BTreeSet<String> = servers.iter().map(|s| s.name.clone()).collect();
        outcome.missing = desired
            .iter()
            .filter(|d| !mcp.contains_key(d.as_str()))
            .cloned()
            .collect();
        outcome.extra_jarvy = managed
            .iter()
            .filter(|m| !desired.contains(m.as_str()))
            .cloned()
            .collect();
        outcome.missing.sort();
        outcome.extra_jarvy.sort();
        Ok(outcome)
    }

    fn remove(&self, scope: McpRegistrationScope) -> Result<RemoveOutcome, McpRegisterError> {
        let path = self.settings_path(scope)?;
        let mut root = read_or_default_object(&path)?;
        let managed = current_managed_names(&root);
        let mut removed = 0usize;
        if let Some(Value::Object(mcp)) = root.get_mut(Self::SERVERS_KEY) {
            for name in &managed {
                if mcp.remove(name).is_some() {
                    removed += 1;
                }
            }
        }
        root.remove(Self::MARKER_KEY);
        write_json(&path, &Value::Object(root))?;
        Ok(RemoveOutcome {
            agent: Self::SLUG,
            path,
            removed,
            warnings: Vec::new(),
        })
    }
}

fn current_managed_names(root: &Map<String, Value>) -> BTreeSet<String> {
    root.get(ClineRegistrar::MARKER_KEY)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn to_server_value(server: &ResolvedServer) -> Value {
    let mut obj = Map::new();
    match server.transport {
        McpServerTransport::Stdio => {
            if let Some(cmd) = &server.command {
                obj.insert("command".to_string(), Value::String(cmd.clone()));
            }
            if !server.args.is_empty() {
                obj.insert(
                    "args".to_string(),
                    Value::Array(server.args.iter().cloned().map(Value::String).collect()),
                );
            }
        }
        McpServerTransport::Http => {
            if let Some(url) = &server.url {
                obj.insert("url".to_string(), Value::String(url.clone()));
            }
        }
    }
    if !server.env.is_empty() {
        obj.insert("env".to_string(), json!(server.env));
    }
    obj.insert("disabled".to_string(), Value::Bool(false));
    Value::Object(obj)
}
