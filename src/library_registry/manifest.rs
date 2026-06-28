//! Manifest schema. One JSON document carries `ai_hook` / `mcp_server`
//! / `skill` items tagged by `kind`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bump in lockstep with breaking schema changes. v1 is the only
/// version this binary understands; mismatched manifests are rejected
/// with `LibraryError::UnsupportedSchema`.
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub publisher: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub homepage: String,
    /// ISO-8601 timestamp; informational only.
    #[serde(default)]
    pub generated_at: String,
    pub items: Vec<LibraryItem>,
}

/// Tagged union of every item kind a library can publish. New kinds
/// are added as new variants — no breaking change required as long as
/// `schema_version` stays at 1 (serde's `tag = "kind"` ignores unknown
/// variants only with extra wiring; v2 will gain that).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LibraryItem {
    AiHook(LibraryHookItem),
    McpServer(LibraryMcpItem),
    Skill(LibrarySkillItem),
}

/// AI hook entry. Mirrors `crate::ai_hooks::library::LibraryHook` but
/// owned (Strings instead of `&'static str`) so it can be loaded from
/// a fetched manifest at runtime.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LibraryHookItem {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub event: String,
    #[serde(default)]
    pub matcher: Option<String>,

    /// Inline bash script. Either `bash` or `bash_url` must be set;
    /// `bash_url` requires `bash_sha256` for tamper detection.
    #[serde(default)]
    pub bash: Option<String>,
    #[serde(default)]
    pub bash_url: Option<String>,
    #[serde(default)]
    pub bash_sha256: Option<String>,

    #[serde(default)]
    pub powershell: Option<String>,
    #[serde(default)]
    pub powershell_url: Option<String>,
    #[serde(default)]
    pub powershell_sha256: Option<String>,

    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

/// MCP server entry. Mirrors `crate::mcp_register::config::McpServerSpec`
/// in shape but additively flat for manifest consumption.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LibraryMcpItem {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Optional list of agent slugs this server is known to support.
    /// Informational — Jarvy registers with every configured agent
    /// regardless. Set to surface a warning when a user attempts to
    /// register `myorg-tickets` with cursor but the publisher only
    /// tested claude-code.
    #[serde(default)]
    pub supported_agents: Vec<String>,
}

/// Skill entry. The skill content (SKILL.md + any companion files) is
/// fetched from `skill_md_url` (and optionally `companion_urls`) when
/// `jarvy skills install` runs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LibrarySkillItem {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub skill_md_url: String,
    pub skill_md_sha256: String,
    /// Additional files to fetch alongside SKILL.md (templates,
    /// helper scripts). Each entry is `{ filename: "...", url: "...",
    /// sha256: "..." }`. Empty by default.
    #[serde(default)]
    pub companion_files: Vec<SkillCompanionFile>,
    /// Agents this skill is known to support. Used by `jarvy skills
    /// install` to skip incompatible agents.
    #[serde(default)]
    pub supported_agents: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SkillCompanionFile {
    pub filename: String,
    pub url: String,
    pub sha256: String,
}

fn default_timeout_ms() -> u64 {
    5_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_manifest() {
        let json = r#"
{
  "schema_version": 1,
  "publisher": "myorg",
  "description": "Internal stuff",
  "homepage": "https://github.com/myorg/jarvy-library",
  "generated_at": "2026-06-28T12:00:00Z",
  "items": [
    {
      "kind": "ai_hook",
      "name": "no-prod-deploys",
      "version": "1.0.0",
      "description": "Block kubectl apply against prod",
      "event": "pre_tool_use",
      "matcher": "Bash",
      "bash": "echo block",
      "powershell": "Write-Host block",
      "timeout_ms": 4000
    },
    {
      "kind": "mcp_server",
      "name": "myorg-tickets",
      "version": "0.3.0",
      "description": "Reads Linear",
      "command": "myorg-mcp",
      "args": ["serve"],
      "env": { "LINEAR_API_KEY": "${LINEAR_API_KEY}" },
      "supported_agents": ["claude-code"]
    },
    {
      "kind": "skill",
      "name": "myorg-code-review",
      "version": "2.1.0",
      "description": "Code review checklist",
      "skill_md_url": "https://cdn.myorg.com/jarvy/skills/code-review-2.1.0/SKILL.md",
      "skill_md_sha256": "abc123",
      "supported_agents": ["claude-code", "cursor"]
    }
  ]
}
"#;
        let m: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.schema_version, 1);
        assert_eq!(m.publisher, "myorg");
        assert_eq!(m.items.len(), 3);

        match &m.items[0] {
            LibraryItem::AiHook(h) => {
                assert_eq!(h.name, "no-prod-deploys");
                assert_eq!(h.timeout_ms, 4000);
                assert_eq!(h.bash.as_deref(), Some("echo block"));
            }
            other => panic!("expected AiHook, got {other:?}"),
        }
        match &m.items[1] {
            LibraryItem::McpServer(s) => {
                assert_eq!(s.name, "myorg-tickets");
                assert_eq!(s.args, vec!["serve"]);
                assert_eq!(
                    s.env.get("LINEAR_API_KEY").map(String::as_str),
                    Some("${LINEAR_API_KEY}")
                );
            }
            other => panic!("expected McpServer, got {other:?}"),
        }
        match &m.items[2] {
            LibraryItem::Skill(s) => {
                assert_eq!(s.name, "myorg-code-review");
                assert_eq!(s.skill_md_sha256, "abc123");
                assert!(s.companion_files.is_empty());
            }
            other => panic!("expected Skill, got {other:?}"),
        }
    }

    #[test]
    fn parses_minimal_hook_with_url_form() {
        let json = r#"
{
  "schema_version": 1,
  "publisher": "myorg",
  "items": [
    {
      "kind": "ai_hook",
      "name": "h",
      "version": "1.0.0",
      "event": "pre_tool_use",
      "bash_url": "https://cdn/h.sh",
      "bash_sha256": "deadbeef"
    }
  ]
}
"#;
        let m: Manifest = serde_json::from_str(json).unwrap();
        if let LibraryItem::AiHook(h) = &m.items[0] {
            assert!(h.bash.is_none());
            assert_eq!(h.bash_url.as_deref(), Some("https://cdn/h.sh"));
            assert_eq!(h.timeout_ms, 5000); // default
        } else {
            panic!("expected AiHook");
        }
    }

    #[test]
    fn rejects_unknown_kind() {
        let json = r#"
{
  "schema_version": 1,
  "publisher": "myorg",
  "items": [
    {
      "kind": "blackmagic",
      "name": "evil"
    }
  ]
}
"#;
        // Untagged enum without explicit "ignore unknown" — unknown
        // kinds fail parse. Documented in the v2 plan.
        let err = serde_json::from_str::<Manifest>(json).unwrap_err();
        assert!(format!("{err}").contains("blackmagic") || format!("{err}").contains("variant"));
    }
}
