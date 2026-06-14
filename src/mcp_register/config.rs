//! `[mcp_register]` schema for `jarvy.toml`.
//!
//! ```toml
//! [mcp_register]
//! agents = ["claude-code", "cursor", "codex", "windsurf", "cline", "continue"]
//! scope = "user"                      # user | project (project ignored where unsupported)
//! allow_custom_servers = false        # gate raw `[[mcp_register.server]]` entries
//!
//! # Built-in Jarvy server registers by default (no entry needed).
//! # To override the binary path or args:
//! # [mcp_register.jarvy]
//! # command = "/usr/local/bin/jarvy"
//! # args = ["mcp"]
//! ```

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::ai_hooks::ConfigOrigin;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct McpRegisterConfig {
    /// Which agents to register with. Empty = no-op.
    pub agents: Vec<McpAgentTarget>,

    /// Where to write registration entries. Some agents (Windsurf, Cline)
    /// only support user-scope.
    pub scope: McpRegistrationScope,

    /// Gate for raw `[[mcp_register.server]]` entries. Library-style: only
    /// the built-in Jarvy server registers by default; custom entries must
    /// opt in AND come from a `ConfigOrigin::Local` source.
    pub allow_custom_servers: bool,

    /// Override the Jarvy server entry (binary path, args, env). Optional —
    /// if omitted, Jarvy registers itself with `command = "jarvy", args =
    /// ["mcp"]`.
    pub jarvy: Option<JarvyServerOverride>,

    /// Additional MCP server entries to register alongside Jarvy. Empty
    /// by default — populated entries flow through the
    /// `allow_custom_servers` + origin gate.
    #[serde(rename = "server", default)]
    pub servers: Vec<McpServerSpec>,

    /// Origin tag. Set by the loader — `Local` for `Config::new`,
    /// `Remote` for `--from <url>`. Not serialized.
    #[serde(skip)]
    pub origin: ConfigOrigin,
}

impl McpRegisterConfig {
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    pub fn unique_agents(&self) -> Vec<McpAgentTarget> {
        let set: BTreeSet<_> = self.agents.iter().copied().collect();
        set.into_iter().collect()
    }
}

/// Override for the built-in `jarvy` MCP server entry. Local-only;
/// remote configs are refused as a trust-boundary violation.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct JarvyServerOverride {
    /// Override the binary path (defaults to bare `jarvy` so PATH lookup).
    pub command: Option<String>,
    /// Override the args (defaults to `["mcp"]`).
    pub args: Option<Vec<String>>,
    /// Optional env vars to attach.
    pub env: std::collections::BTreeMap<String, String>,
}

/// One additional (non-Jarvy) MCP server entry.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct McpServerSpec {
    pub name: String,
    pub transport: McpServerTransport,
    /// stdio: required command. http: ignored.
    pub command: Option<String>,
    /// stdio: optional args.
    #[serde(default)]
    pub args: Vec<String>,
    /// http: required URL.
    pub url: Option<String>,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    /// Restrict this entry to a subset of the top-level `agents` list.
    /// Empty = apply to all configured agents.
    #[serde(default)]
    pub agents: Vec<McpAgentTarget>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpServerTransport {
    #[default]
    Stdio,
    Http,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpRegistrationScope {
    #[default]
    User,
    Project,
}

/// Which agent to register with. Mirrors `AgentTarget` in ai_hooks but
/// kept separate so the two subsystems don't accidentally diverge on
/// support matrices.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[repr(u8)]
pub enum McpAgentTarget {
    ClaudeCode = 0,
    Cursor = 1,
    Codex = 2,
    Windsurf = 3,
    Cline = 4,
    Continue = 5,
}

impl McpAgentTarget {
    /// Every variant in stable order. Used by tests; reserved for
    /// future per-agent CLI flags.
    #[allow(dead_code)]
    pub const ALL: &'static [McpAgentTarget] = &[
        McpAgentTarget::ClaudeCode,
        McpAgentTarget::Cursor,
        McpAgentTarget::Codex,
        McpAgentTarget::Windsurf,
        McpAgentTarget::Cline,
        McpAgentTarget::Continue,
    ];

    #[allow(dead_code)]
    pub const COUNT: usize = 6;

    pub fn slug(self) -> &'static str {
        match self {
            McpAgentTarget::ClaudeCode => "claude-code",
            McpAgentTarget::Cursor => "cursor",
            McpAgentTarget::Codex => "codex",
            McpAgentTarget::Windsurf => "windsurf",
            McpAgentTarget::Cline => "cline",
            McpAgentTarget::Continue => "continue",
        }
    }

    /// Whether this agent supports project-scope registration. Windsurf,
    /// Cline, and Continue (in its current single-file mode) do not.
    /// Currently informational — registrars fall back to user scope
    /// with a warning when project is requested.
    #[allow(dead_code)]
    pub fn supports_project_scope(self) -> bool {
        matches!(
            self,
            McpAgentTarget::ClaudeCode | McpAgentTarget::Cursor | McpAgentTarget::Codex
        )
    }
}

impl std::fmt::Display for McpAgentTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.slug())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let toml = r#"
            agents = ["claude-code", "cursor"]
        "#;
        let cfg: McpRegisterConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.agents.len(), 2);
        assert!(cfg.servers.is_empty());
        assert!(!cfg.allow_custom_servers);
        assert_eq!(cfg.origin, ConfigOrigin::Local);
    }

    #[test]
    fn parses_jarvy_override_and_custom_server() {
        let toml = r#"
            agents = ["claude-code"]
            allow_custom_servers = true

            [jarvy]
            command = "/usr/local/bin/jarvy"
            args = ["mcp", "--verbose"]

            [[server]]
            name = "github"
            transport = "stdio"
            command = "gh-mcp-server"
        "#;
        let cfg: McpRegisterConfig = toml::from_str(toml).unwrap();
        let jarvy = cfg.jarvy.expect("jarvy override");
        assert_eq!(jarvy.command.as_deref(), Some("/usr/local/bin/jarvy"));
        assert_eq!(jarvy.args.unwrap().len(), 2);
        assert_eq!(cfg.servers[0].name, "github");
    }

    #[test]
    fn rejects_unknown_fields() {
        let toml = r#"
            agents = ["cursor"]
            mystery = true
        "#;
        assert!(toml::from_str::<McpRegisterConfig>(toml).is_err());
    }

    #[test]
    fn agent_project_scope_support_matrix() {
        assert!(McpAgentTarget::ClaudeCode.supports_project_scope());
        assert!(McpAgentTarget::Cursor.supports_project_scope());
        assert!(McpAgentTarget::Codex.supports_project_scope());
        assert!(!McpAgentTarget::Windsurf.supports_project_scope());
        assert!(!McpAgentTarget::Cline.supports_project_scope());
        assert!(!McpAgentTarget::Continue.supports_project_scope());
    }
}
