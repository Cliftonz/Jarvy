//! MCP Error Types
//!
//! Defines error types for the MCP server with JSON-RPC 2.0 error codes.
//!
//! ## Standard JSON-RPC Error Codes
//! - -32700: Parse error
//! - -32600: Invalid request
//! - -32601: Method not found
//! - -32602: Invalid params
//! - -32603: Internal error
//!
//! ## Custom Error Codes (application-specific, -32000 to -32099)
//! - -32001: Unknown tool
//! - -32002: Tool denied (denylist)
//! - -32003: Tool not allowed (allowlist)
//! - -32004: Rate limited
//! - -32005: User cancelled
//! - -32006: Installation failed
//! - -32007: Configuration error

use serde::{Deserialize, Serialize};
use std::fmt;

/// Result type for MCP operations
pub type McpResult<T> = Result<T, McpError>;

/// MCP error with JSON-RPC compatible error codes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    /// JSON-RPC error code
    pub code: i32,
    /// Human-readable error message
    pub message: String,
    /// Optional additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl McpError {
    /// Create a new MCP error
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create an error with additional data
    pub fn with_data(code: i32, message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    // Standard JSON-RPC errors

    /// Parse error (-32700): Invalid JSON was received
    pub fn parse_error(details: impl Into<String>) -> Self {
        Self::new(-32700, format!("Parse error: {}", details.into()))
    }

    /// Invalid request (-32600): The JSON is not a valid Request object
    pub fn invalid_request(details: impl Into<String>) -> Self {
        Self::new(-32600, format!("Invalid request: {}", details.into()))
    }

    /// Method not found (-32601): The method does not exist
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self::new(-32601, format!("Method not found: {}", method.into()))
    }

    /// Invalid params (-32602): Invalid method parameters
    pub fn invalid_params(details: impl Into<String>) -> Self {
        Self::new(-32602, format!("Invalid params: {}", details.into()))
    }

    /// Internal error (-32603): Internal JSON-RPC error
    pub fn internal_error(details: impl Into<String>) -> Self {
        Self::new(-32603, format!("Internal error: {}", details.into()))
    }

    // Custom application errors (-32000 to -32099)

    /// Unknown tool (-32001): The requested tool is not in Jarvy's registry
    pub fn unknown_tool(tool_name: impl Into<String>) -> Self {
        let name = tool_name.into();
        Self::with_data(
            -32001,
            format!("Unknown tool: '{}' is not in Jarvy's tool registry", name),
            serde_json::json!({ "tool": name }),
        )
    }

    /// Tool denied (-32002): Tool is in the denylist
    pub fn tool_denied(tool_name: impl Into<String>) -> Self {
        let name = tool_name.into();
        Self::with_data(
            -32002,
            format!(
                "Tool denied: '{}' is in the MCP denylist and cannot be installed",
                name
            ),
            serde_json::json!({ "tool": name }),
        )
    }

    /// Tool not allowed (-32003): Tool is not in the allowlist (when allowlist is configured)
    pub fn tool_not_allowed(tool_name: impl Into<String>) -> Self {
        let name = tool_name.into();
        Self::with_data(
            -32003,
            format!("Tool not allowed: '{}' is not in the MCP allowlist", name),
            serde_json::json!({ "tool": name }),
        )
    }

    /// Rate limited (-32004): Too many requests
    pub fn rate_limited(details: impl Into<String>) -> Self {
        Self::new(-32004, format!("Rate limited: {}", details.into()))
    }

    /// User cancelled (-32005): User cancelled the operation
    pub fn user_cancelled() -> Self {
        Self::new(-32005, "User cancelled the operation")
    }

    /// Installation failed (-32006): Tool installation failed
    pub fn installation_failed(tool_name: impl Into<String>, reason: impl Into<String>) -> Self {
        let name = tool_name.into();
        Self::with_data(
            -32006,
            format!("Installation failed for '{}': {}", name, reason.into()),
            serde_json::json!({ "tool": name }),
        )
    }

    /// Configuration error (-32007): MCP configuration error
    pub fn config_error(details: impl Into<String>) -> Self {
        Self::new(-32007, format!("Configuration error: {}", details.into()))
    }
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for McpError {}

impl From<std::io::Error> for McpError {
    fn from(err: std::io::Error) -> Self {
        McpError::internal_error(err.to_string())
    }
}

impl From<serde_json::Error> for McpError {
    fn from(err: serde_json::Error) -> Self {
        McpError::parse_error(err.to_string())
    }
}

impl From<toml::de::Error> for McpError {
    fn from(err: toml::de::Error) -> Self {
        McpError::config_error(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(McpError::parse_error("test").code, -32700);
        assert_eq!(McpError::invalid_request("test").code, -32600);
        assert_eq!(McpError::method_not_found("test").code, -32601);
        assert_eq!(McpError::invalid_params("test").code, -32602);
        assert_eq!(McpError::internal_error("test").code, -32603);

        assert_eq!(McpError::unknown_tool("git").code, -32001);
        assert_eq!(McpError::tool_denied("brew").code, -32002);
        assert_eq!(McpError::tool_not_allowed("vim").code, -32003);
        assert_eq!(McpError::rate_limited("too fast").code, -32004);
        assert_eq!(McpError::user_cancelled().code, -32005);
        assert_eq!(McpError::installation_failed("git", "error").code, -32006);
        assert_eq!(McpError::config_error("bad config").code, -32007);
    }

    #[test]
    fn test_error_display() {
        let err = McpError::unknown_tool("foobar");
        assert!(err.to_string().contains("-32001"));
        assert!(err.to_string().contains("foobar"));
    }

    #[test]
    fn test_error_with_data() {
        let err = McpError::unknown_tool("mytools");
        assert!(err.data.is_some());
        let data = err.data.unwrap();
        assert_eq!(data["tool"], "mytools");
    }
}
