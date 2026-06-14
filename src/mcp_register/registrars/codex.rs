//! Codex CLI registrar.
//!
//! Writes to `~/.codex/config.toml` (user) or `.codex/config.toml`
//! (project — only loaded when the project is trusted).
//!
//! Each entry lives at `[mcp_servers.<name>]`. Marker scheme:
//! `_jarvy_managed_servers = ["jarvy", ...]` array at the root tracks
//! which `mcp_servers.*` tables Jarvy owns.

use std::collections::BTreeSet;
use std::path::PathBuf;

use toml::Value;

use super::{AgentRegistrar, ApplyOutcome, CheckOutcome, RemoveOutcome, ResolvedServer};
use crate::ai_hooks::agents::io::{home_or_err, write_text_atomic};
use crate::mcp_register::config::{McpRegistrationScope, McpServerTransport};
use crate::mcp_register::error::McpRegisterError;

pub struct CodexRegistrar;

impl CodexRegistrar {
    const SLUG: &'static str = "codex";
    const MARKER_KEY: &'static str = "_jarvy_managed_servers";
    const SERVERS_KEY: &'static str = "mcp_servers";

    fn read_toml(path: &std::path::Path) -> Result<Value, McpRegisterError> {
        if !path.exists() {
            return Ok(Value::Table(Default::default()));
        }
        let body = std::fs::read_to_string(path).map_err(|e| McpRegisterError::io(path, e))?;
        if body.trim().is_empty() {
            return Ok(Value::Table(Default::default()));
        }
        toml::from_str::<Value>(&body).map_err(|source| McpRegisterError::ParseToml {
            path: path.to_path_buf(),
            source,
        })
    }
}

impl AgentRegistrar for CodexRegistrar {
    fn slug(&self) -> &'static str {
        Self::SLUG
    }

    fn settings_path(&self, scope: McpRegistrationScope) -> Result<PathBuf, McpRegisterError> {
        match scope {
            McpRegistrationScope::User => Ok(home_or_err()?.join(".codex").join("config.toml")),
            McpRegistrationScope::Project => Ok(PathBuf::from(".codex").join("config.toml")),
        }
    }

    fn apply(
        &self,
        servers: &[ResolvedServer],
        scope: McpRegistrationScope,
    ) -> Result<ApplyOutcome, McpRegisterError> {
        let path = self.settings_path(scope)?;
        let mut root = Self::read_toml(&path)?;
        let root_table = root
            .as_table_mut()
            .ok_or_else(|| McpRegisterError::InvalidEntry {
                name: path.display().to_string(),
                reason: "config.toml root must be a table".to_string(),
            })?;

        let mut managed = current_managed_names(root_table);

        let mcp_servers_value = root_table
            .entry(Self::SERVERS_KEY.to_string())
            .or_insert_with(|| Value::Table(toml::Table::new()));
        let Value::Table(mcp_tbl) = mcp_servers_value else {
            return Err(McpRegisterError::InvalidEntry {
                name: Self::SERVERS_KEY.to_string(),
                reason: format!("existing `{}` is not a table", Self::SERVERS_KEY),
            });
        };

        let mut applied = 0usize;
        let desired: BTreeSet<String> = servers.iter().map(|s| s.name.clone()).collect();
        for name in managed.difference(&desired).cloned().collect::<Vec<_>>() {
            mcp_tbl.remove(&name);
            managed.remove(&name);
        }
        for server in servers {
            mcp_tbl.insert(server.name.clone(), to_server_value(server));
            managed.insert(server.name.clone());
            applied += 1;
        }
        let names: Vec<Value> = managed.iter().map(|n| Value::String(n.clone())).collect();
        root_table.insert(Self::MARKER_KEY.to_string(), Value::Array(names));

        let serialized =
            toml::to_string_pretty(&root).map_err(|e| McpRegisterError::InvalidEntry {
                name: "serialize".to_string(),
                reason: e.to_string(),
            })?;
        write_text_atomic(&path, &serialized)?;
        Ok(ApplyOutcome {
            agent: Self::SLUG,
            path,
            applied,
            warnings: Vec::new(),
        })
    }

    fn check(
        &self,
        servers: &[ResolvedServer],
        scope: McpRegistrationScope,
    ) -> Result<CheckOutcome, McpRegisterError> {
        let path = self.settings_path(scope)?;
        let root = Self::read_toml(&path)?;
        let mut outcome = CheckOutcome {
            agent: Self::SLUG,
            path,
            ..CheckOutcome::default()
        };
        let root_tbl = match root.as_table() {
            Some(t) => t,
            None => {
                outcome.missing = servers.iter().map(|s| s.name.clone()).collect();
                return Ok(outcome);
            }
        };
        let mcp = match root_tbl.get(Self::SERVERS_KEY).and_then(|v| v.as_table()) {
            Some(t) => t,
            None => {
                outcome.missing = servers.iter().map(|s| s.name.clone()).collect();
                return Ok(outcome);
            }
        };
        let managed = current_managed_names(root_tbl);
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
        let mut root = Self::read_toml(&path)?;
        let root_tbl = root
            .as_table_mut()
            .ok_or_else(|| McpRegisterError::InvalidEntry {
                name: path.display().to_string(),
                reason: "config.toml root must be a table".to_string(),
            })?;
        let managed = current_managed_names(root_tbl);
        let mut removed = 0usize;
        if let Some(Value::Table(mcp)) = root_tbl.get_mut(Self::SERVERS_KEY) {
            for name in &managed {
                if mcp.remove(name).is_some() {
                    removed += 1;
                }
            }
        }
        root_tbl.remove(Self::MARKER_KEY);
        let serialized =
            toml::to_string_pretty(&root).map_err(|e| McpRegisterError::InvalidEntry {
                name: "serialize".to_string(),
                reason: e.to_string(),
            })?;
        write_text_atomic(&path, &serialized)?;
        Ok(RemoveOutcome {
            agent: Self::SLUG,
            path,
            removed,
            warnings: Vec::new(),
        })
    }
}

fn current_managed_names(root: &toml::Table) -> BTreeSet<String> {
    root.get(CodexRegistrar::MARKER_KEY)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn to_server_value(server: &ResolvedServer) -> Value {
    let mut tbl = toml::Table::new();
    match server.transport {
        McpServerTransport::Stdio => {
            if let Some(cmd) = &server.command {
                tbl.insert("command".to_string(), Value::String(cmd.clone()));
            }
            if !server.args.is_empty() {
                tbl.insert(
                    "args".to_string(),
                    Value::Array(server.args.iter().cloned().map(Value::String).collect()),
                );
            }
        }
        McpServerTransport::Http => {
            if let Some(url) = &server.url {
                tbl.insert("url".to_string(), Value::String(url.clone()));
            }
        }
    }
    if !server.env.is_empty() {
        let mut env_tbl = toml::Table::new();
        for (k, v) in &server.env {
            env_tbl.insert(k.clone(), Value::String(v.clone()));
        }
        tbl.insert("env".to_string(), Value::Table(env_tbl));
    }
    Value::Table(tbl)
}
