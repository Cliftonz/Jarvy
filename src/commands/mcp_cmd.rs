//! MCP command handler - start the Model Context Protocol server

use std::path::PathBuf;

use crate::mcp;

/// Run the MCP server
pub fn run_mcp(config: Option<PathBuf>) -> i32 {
    if let Err(e) = mcp::run(config) {
        eprintln!("MCP server error: {}", e);
        return 1;
    }

    0
}
