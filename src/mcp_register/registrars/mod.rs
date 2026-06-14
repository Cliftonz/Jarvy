//! Per-agent MCP server registrars.
//!
//! Each agent stores MCP server entries in its own settings file with a
//! slightly different schema (object vs array vs TOML). The trait keeps
//! the runner agnostic; per-file implementations encode the agent's
//! specifics.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::config::{McpAgentTarget, McpRegistrationScope, McpServerTransport};
use super::error::McpRegisterError;

pub mod claude_code;
pub mod cline;
pub mod codex;
pub mod continue_dev;
pub mod cursor;
pub mod windsurf;

/// One resolved MCP server entry, ready for a registrar to serialize.
/// The runner produces these by combining the built-in Jarvy entry
/// (always present) with any user-supplied custom servers that passed
/// the trust gate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolvedServer {
    pub name: String,
    pub transport: McpServerTransport,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env: BTreeMap<String, String>,
    /// Whether this entry is the built-in Jarvy server. Used by `check`
    /// to know which entry should always be present.
    pub is_jarvy: bool,
}

#[derive(Debug, Clone)]
pub struct ApplyOutcome {
    pub agent: &'static str,
    pub path: PathBuf,
    pub applied: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RemoveOutcome {
    pub agent: &'static str,
    pub path: PathBuf,
    pub removed: usize,
    #[allow(dead_code)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CheckOutcome {
    pub agent: &'static str,
    pub path: PathBuf,
    pub missing: Vec<String>,
    pub extra_jarvy: Vec<String>,
}

impl CheckOutcome {
    pub fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.extra_jarvy.is_empty()
    }
}

pub trait AgentRegistrar: Sync {
    /// Stable identifier — read by tests and integration harnesses.
    #[allow(dead_code)]
    fn slug(&self) -> &'static str;

    fn settings_path(&self, scope: McpRegistrationScope) -> Result<PathBuf, McpRegisterError>;

    fn apply(
        &self,
        servers: &[ResolvedServer],
        scope: McpRegistrationScope,
    ) -> Result<ApplyOutcome, McpRegisterError>;

    fn check(
        &self,
        servers: &[ResolvedServer],
        scope: McpRegistrationScope,
    ) -> Result<CheckOutcome, McpRegisterError>;

    fn remove(&self, scope: McpRegistrationScope) -> Result<RemoveOutcome, McpRegisterError>;
}

// Stateless ZSTs — `&'static dyn` dispatch, no Box allocation.
static CLAUDE_CODE: claude_code::ClaudeCodeRegistrar = claude_code::ClaudeCodeRegistrar;
static CURSOR: cursor::CursorRegistrar = cursor::CursorRegistrar;
static CODEX: codex::CodexRegistrar = codex::CodexRegistrar;
static WINDSURF: windsurf::WindsurfRegistrar = windsurf::WindsurfRegistrar;
static CLINE: cline::ClineRegistrar = cline::ClineRegistrar;
static CONTINUE: continue_dev::ContinueRegistrar = continue_dev::ContinueRegistrar;

pub fn registrar_for(target: McpAgentTarget) -> &'static dyn AgentRegistrar {
    match target {
        McpAgentTarget::ClaudeCode => &CLAUDE_CODE,
        McpAgentTarget::Cursor => &CURSOR,
        McpAgentTarget::Codex => &CODEX,
        McpAgentTarget::Windsurf => &WINDSURF,
        McpAgentTarget::Cline => &CLINE,
        McpAgentTarget::Continue => &CONTINUE,
    }
}
