//! Continue.dev registrar.
//!
//! Continue uses a directory of per-server YAML files under
//! `.continue/mcpServers/<name>.yaml`. Each file Jarvy writes is named
//! `<name>.jarvy.yaml` so the marker is the filename itself —
//! ownership is unambiguous, removal is just file deletion.
//!
//! Only project-scope is well-documented; user-scope MCP comes from
//! Continue Hub assistants. If the caller passes `User` scope we fall
//! back to project scope with a warning.

use std::collections::BTreeSet;
use std::path::PathBuf;

use super::{AgentRegistrar, ApplyOutcome, CheckOutcome, RemoveOutcome, ResolvedServer};
use crate::ai_hooks::agents::io::write_text_atomic;
use crate::mcp_register::config::{McpRegistrationScope, McpServerTransport};
use crate::mcp_register::error::McpRegisterError;

pub struct ContinueRegistrar;

impl ContinueRegistrar {
    const SLUG: &'static str = "continue";
    const SUFFIX: &'static str = ".jarvy.yaml";

    fn server_dir() -> PathBuf {
        PathBuf::from(".continue").join("mcpServers")
    }

    fn fragment_path(name: &str) -> PathBuf {
        Self::server_dir().join(format!("{name}{}", Self::SUFFIX))
    }

    fn parse_jarvy_filename(name: &str) -> Option<String> {
        name.strip_suffix(Self::SUFFIX).map(String::from)
    }

    fn serialize(server: &ResolvedServer) -> String {
        let mut out = String::new();
        out.push_str("name: ");
        out.push_str(&yaml_quote(&format!("Jarvy-managed: {}", server.name)));
        out.push_str("\nversion: 0.0.1\nschema: v1\nmcpServers:\n  - name: ");
        out.push_str(&yaml_quote(&server.name));
        match server.transport {
            McpServerTransport::Stdio => {
                if let Some(cmd) = &server.command {
                    out.push_str("\n    command: ");
                    out.push_str(&yaml_quote(cmd));
                }
                if !server.args.is_empty() {
                    out.push_str("\n    args:");
                    for a in &server.args {
                        out.push_str("\n      - ");
                        out.push_str(&yaml_quote(a));
                    }
                }
                out.push_str("\n    type: stdio");
            }
            McpServerTransport::Http => {
                out.push_str("\n    type: streamable-http");
                if let Some(url) = &server.url {
                    out.push_str("\n    url: ");
                    out.push_str(&yaml_quote(url));
                }
            }
        }
        if !server.env.is_empty() {
            out.push_str("\n    env:");
            for (k, v) in &server.env {
                out.push_str("\n      ");
                out.push_str(&yaml_quote(k));
                out.push_str(": ");
                out.push_str(&yaml_quote(v));
            }
        }
        out.push('\n');
        out
    }

    fn list_owned(dir: &std::path::Path) -> std::io::Result<BTreeSet<String>> {
        let mut out = BTreeSet::new();
        if !dir.exists() {
            return Ok(out);
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str()
                && let Some(server_name) = Self::parse_jarvy_filename(name)
            {
                out.insert(server_name);
            }
        }
        Ok(out)
    }
}

fn yaml_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\\\""))
}

impl AgentRegistrar for ContinueRegistrar {
    fn slug(&self) -> &'static str {
        Self::SLUG
    }

    fn settings_path(&self, _scope: McpRegistrationScope) -> Result<PathBuf, McpRegisterError> {
        // Conceptually a directory; report the dir path itself.
        Ok(Self::server_dir())
    }

    fn apply(
        &self,
        servers: &[ResolvedServer],
        scope: McpRegistrationScope,
    ) -> Result<ApplyOutcome, McpRegisterError> {
        let dir = Self::server_dir();
        let mut applied = 0usize;
        let mut warnings = Vec::new();
        if scope == McpRegistrationScope::User {
            warnings.push(
                "continue.dev user-scope MCP lives in Continue Hub assistants; \
                 writing project-scope file instead"
                    .to_string(),
            );
        }
        // Sweep stale jarvy files before writing the desired set so a
        // server removed from `jarvy.toml` actually leaves on next apply.
        let existing = Self::list_owned(&dir).unwrap_or_default();
        let desired: BTreeSet<String> = servers.iter().map(|s| s.name.clone()).collect();
        for stale in existing.difference(&desired) {
            let _ = std::fs::remove_file(Self::fragment_path(stale));
        }
        for server in servers {
            let path = Self::fragment_path(&server.name);
            write_text_atomic(&path, &Self::serialize(server))?;
            applied += 1;
        }
        Ok(ApplyOutcome {
            agent: Self::SLUG,
            path: dir,
            applied,
            warnings,
        })
    }

    fn check(
        &self,
        servers: &[ResolvedServer],
        _scope: McpRegistrationScope,
    ) -> Result<CheckOutcome, McpRegisterError> {
        let dir = Self::server_dir();
        let mut outcome = CheckOutcome {
            agent: Self::SLUG,
            path: dir.clone(),
            ..CheckOutcome::default()
        };
        let existing = Self::list_owned(&dir).unwrap_or_default();
        let desired: BTreeSet<String> = servers.iter().map(|s| s.name.clone()).collect();
        outcome.missing = desired.difference(&existing).cloned().collect();
        outcome.extra_jarvy = existing.difference(&desired).cloned().collect();
        outcome.missing.sort();
        outcome.extra_jarvy.sort();
        Ok(outcome)
    }

    fn remove(&self, _scope: McpRegistrationScope) -> Result<RemoveOutcome, McpRegisterError> {
        let dir = Self::server_dir();
        let existing = Self::list_owned(&dir).unwrap_or_default();
        let mut removed = 0usize;
        for name in existing {
            if std::fs::remove_file(Self::fragment_path(&name)).is_ok() {
                removed += 1;
            }
        }
        Ok(RemoveOutcome {
            agent: Self::SLUG,
            path: dir,
            removed,
            warnings: Vec::new(),
        })
    }
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
    fn serialize_round_trips_to_valid_yaml_ish() {
        let yaml = ContinueRegistrar::serialize(&jarvy_server());
        assert!(yaml.contains("name: \""));
        assert!(yaml.contains("schema: v1"));
        assert!(yaml.contains("- name: \"jarvy\""));
        assert!(yaml.contains("command: \"jarvy\""));
        assert!(yaml.contains("type: stdio"));
    }

    #[test]
    fn parse_jarvy_filename_recognizes_suffix() {
        assert_eq!(
            ContinueRegistrar::parse_jarvy_filename("jarvy.jarvy.yaml"),
            Some("jarvy".to_string())
        );
        assert!(ContinueRegistrar::parse_jarvy_filename("user.yaml").is_none());
    }
}
