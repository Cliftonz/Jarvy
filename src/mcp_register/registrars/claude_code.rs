//! Claude Code registrar.
//!
//! Writes to `~/.claude.json` (user) or `.mcp.json` (project). `~/.claude.json`
//! also stores Claude Code's general settings — we JSON-merge, never overwrite.
//!
//! Marker scheme: a parallel `_jarvy_managed_servers: ["jarvy", ...]`
//! array at the root tracks which `mcpServers` keys Jarvy owns. The
//! server entries themselves stay schema-clean so Claude Code's
//! validator never complains.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::{Map, Value, json};

use super::{AgentRegistrar, ApplyOutcome, CheckOutcome, RemoveOutcome, ResolvedServer};
use crate::ai_hooks::agents::io::{home_or_err, read_or_default_object, write_json};
use crate::mcp_register::config::{McpRegistrationScope, McpServerTransport};
use crate::mcp_register::error::McpRegisterError;

pub struct ClaudeCodeRegistrar;

impl ClaudeCodeRegistrar {
    const SLUG: &'static str = "claude-code";
    const MARKER_KEY: &'static str = "_jarvy_managed_servers";
    const SERVERS_KEY: &'static str = "mcpServers";
}

impl AgentRegistrar for ClaudeCodeRegistrar {
    fn slug(&self) -> &'static str {
        Self::SLUG
    }

    fn settings_path(&self, scope: McpRegistrationScope) -> Result<PathBuf, McpRegisterError> {
        match scope {
            McpRegistrationScope::User => Ok(home_or_err()?.join(".claude.json")),
            McpRegistrationScope::Project => Ok(PathBuf::from(".mcp.json")),
        }
    }

    fn apply(
        &self,
        servers: &[ResolvedServer],
        scope: McpRegistrationScope,
    ) -> Result<ApplyOutcome, McpRegisterError> {
        let path = self.settings_path(scope)?;
        let mut root = read_or_default_object(&path)?;

        // Read the marker BEFORE taking a mutable borrow on the
        // `mcpServers` table (rustc otherwise sees overlapping borrows).
        let mut managed: BTreeSet<String> = current_managed_names(&root);

        let mcp_servers = root
            .entry(Self::SERVERS_KEY)
            .or_insert_with(|| Value::Object(Map::new()));
        let Value::Object(mcp_obj) = mcp_servers else {
            return Err(McpRegisterError::InvalidEntry {
                name: Self::SERVERS_KEY.to_string(),
                reason: format!("existing `{}` field is not an object", Self::SERVERS_KEY),
            });
        };

        let mut applied = 0usize;
        // Strip the keys we previously owned but no longer want.
        let desired_names: BTreeSet<String> = servers.iter().map(|s| s.name.clone()).collect();
        let stale: Vec<String> = managed.difference(&desired_names).cloned().collect();
        for name in stale {
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
        for d in &desired {
            if !mcp.contains_key(d) {
                outcome.missing.push(d.clone());
            }
        }
        for m in &managed {
            if !desired.contains(m) {
                outcome.extra_jarvy.push(m.clone());
            }
        }
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
    root.get(ClaudeCodeRegistrar::MARKER_KEY)
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
            obj.insert("type".to_string(), Value::String("http".to_string()));
            if let Some(url) = &server.url {
                obj.insert("url".to_string(), Value::String(url.clone()));
            }
        }
    }
    if !server.env.is_empty() {
        obj.insert("env".to_string(), json!(server.env));
    }
    Value::Object(obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn jarvy_server() -> ResolvedServer {
        ResolvedServer {
            name: "jarvy".to_string(),
            transport: McpServerTransport::Stdio,
            command: Some("jarvy".to_string()),
            args: vec!["mcp".to_string()],
            url: None,
            env: BTreeMap::new(),
            is_jarvy: true,
        }
    }

    #[test]
    fn to_server_value_stdio_shape() {
        let v = to_server_value(&jarvy_server());
        assert_eq!(v["command"], "jarvy");
        assert_eq!(v["args"][0], "mcp");
    }
}
