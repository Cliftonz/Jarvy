//! MCP (Model Context Protocol) Server for Jarvy
//!
//! This module exposes Jarvy as an MCP server, enabling LLMs (Claude, GPT, etc.)
//! to safely discover, verify, and install development tools with mandatory user confirmation.
//!
//! ## Architecture
//!
//! The MCP server uses JSON-RPC 2.0 over stdio transport:
//! - `server.rs` - Main server loop and request routing
//! - `transport.rs` - Stdio transport implementation
//! - `tools.rs` - MCP tool handlers (jarvy_list_tools, jarvy_check_tool, etc.)
//! - `resources.rs` - MCP resource handlers
//! - `prompts.rs` - MCP prompt handlers
//! - `safety.rs` - Rate limiting, allowlist/denylist, confirmation
//! - `config.rs` - MCP-specific configuration
//! - `audit.rs` - Audit logging
//! - `error.rs` - Error types with JSON-RPC error codes
//!
//! ## Safety
//!
//! The MCP server is designed with safety as the primary concern:
//! - `dry_run: true` by default for all installs
//! - Confirmation prompts via stderr (not MCP response)
//! - Rate limiting (10 checks/min, 3 installs/min)
//! - Configurable allowlist/denylist via ~/.jarvy/mcp-config.toml
//! - Audit logging to ~/.jarvy/mcp-audit.log

pub mod audit;
pub mod config;
pub mod error;
pub mod prompts;
pub mod resources;
pub mod safety;
pub mod server;
pub mod tools;
pub mod transport;

pub use config::McpConfig;
#[allow(unused_imports)]
pub use error::{McpError, McpResult};
pub use server::McpServer;

/// MCP protocol version supported by this implementation
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Server name for MCP identification
pub const SERVER_NAME: &str = "jarvy";

/// Server version (matches Cargo.toml version)
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the MCP server with the given configuration
pub fn run(config_path: Option<std::path::PathBuf>) -> McpResult<()> {
    let config = if let Some(path) = config_path {
        McpConfig::load_from(&path)?
    } else {
        McpConfig::load_default()?
    };

    let server = McpServer::new(config);
    server.run()
}
