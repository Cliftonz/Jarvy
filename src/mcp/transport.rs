//! MCP Transport Layer
//!
//! Implements the stdio transport for MCP JSON-RPC communication.
//! Messages are delimited by newlines, with each line being a complete JSON-RPC message.

use crate::mcp::error::{McpError, McpResult};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (must be "2.0")
    pub jsonrpc: String,
    /// Request ID (can be number, string, or null for notifications)
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    /// Method name
    pub method: String,
    /// Method parameters
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Request ID (matches the request)
    pub id: Option<serde_json::Value>,
    /// Result (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 Error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Optional additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    /// Create a success response
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: Option<serde_json::Value>, err: McpError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: err.code,
                message: err.message,
                data: err.data,
            }),
        }
    }
}

impl From<McpError> for JsonRpcError {
    fn from(err: McpError) -> Self {
        Self {
            code: err.code,
            message: err.message,
            data: err.data,
        }
    }
}

/// Stdio transport for MCP communication
pub struct StdioTransport {
    reader: BufReader<std::io::Stdin>,
}

impl StdioTransport {
    /// Create a new stdio transport
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(std::io::stdin()),
        }
    }

    /// Read a JSON-RPC request from stdin
    pub fn read_request(&mut self) -> McpResult<Option<JsonRpcRequest>> {
        let mut line = String::new();

        match self.reader.read_line(&mut line) {
            Ok(0) => {
                // EOF - client closed connection
                Ok(None)
            }
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    // Empty line, try again
                    return self.read_request();
                }

                let request: JsonRpcRequest = serde_json::from_str(line)
                    .map_err(|e| McpError::parse_error(format!("Invalid JSON: {}", e)))?;

                // Validate JSON-RPC version
                if request.jsonrpc != "2.0" {
                    return Err(McpError::invalid_request(format!(
                        "Expected JSON-RPC 2.0, got '{}'",
                        request.jsonrpc
                    )));
                }

                Ok(Some(request))
            }
            Err(e) => Err(McpError::internal_error(format!("Read error: {}", e))),
        }
    }

    /// Write a JSON-RPC response to stdout
    pub fn write_response(&self, response: &JsonRpcResponse) -> McpResult<()> {
        let json = serde_json::to_string(response)?;
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "{}", json)?;
        stdout.flush()?;
        Ok(())
    }

    /// Write a message to stderr (for confirmation prompts, logs, etc.)
    /// This does not interfere with the MCP protocol on stdout
    #[allow(dead_code)] // Public API for MCP transport
    pub fn write_stderr(&self, message: &str) -> McpResult<()> {
        let mut stderr = std::io::stderr().lock();
        writeln!(stderr, "{}", message)?;
        stderr.flush()?;
        Ok(())
    }

    /// Read a line from stdin (for confirmation prompts)
    /// Note: This should only be used when the terminal is interactive
    #[allow(dead_code)] // Public API for MCP transport
    pub fn read_confirmation(&mut self) -> McpResult<String> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        Ok(line.trim().to_string())
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_response() {
        let response = JsonRpcResponse::success(
            Some(serde_json::json!(1)),
            serde_json::json!({"status": "ok"}),
        );

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_error_response() {
        let err = McpError::unknown_tool("git");
        let response = JsonRpcResponse::error(Some(serde_json::json!(1)), err);

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32001);
    }

    #[test]
    fn test_request_parsing() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, Some(serde_json::json!(1)));
        assert_eq!(request.method, "tools/list");
    }

    #[test]
    fn test_notification_parsing() {
        // Notifications don't have an id
        let json = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.jsonrpc, "2.0");
        assert!(request.id.is_none());
        assert_eq!(request.method, "notifications/initialized");
    }
}
