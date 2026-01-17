//! MCP Server Integration Tests
//!
//! These tests verify the MCP server works end-to-end by spawning
//! the jarvy mcp process and communicating via JSON-RPC over stdio.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Helper to send a JSON-RPC request and get the response
fn send_request(stdin: &mut impl Write, stdout: &mut impl BufRead, request: Value) -> Value {
    let request_str = serde_json::to_string(&request).unwrap();
    writeln!(stdin, "{}", request_str).unwrap();
    stdin.flush().unwrap();

    let mut response_line = String::new();
    stdout.read_line(&mut response_line).unwrap();
    serde_json::from_str(&response_line).unwrap_or_else(|e| {
        panic!(
            "Failed to parse response: {}\nResponse was: {}",
            e, response_line
        )
    })
}

/// Spawn the MCP server process
fn spawn_mcp_server() -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_jarvy"))
        .args(["mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn jarvy mcp")
}

#[test]
fn test_mcp_initialize_handshake() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Send initialize request
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }),
    );

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let result = &response["result"];
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "jarvy");
    assert!(result["capabilities"]["tools"].is_object());
    assert!(result["capabilities"]["resources"].is_object());
    assert!(result["capabilities"]["prompts"].is_object());

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_tools_list() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize first
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Request tools list
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let tools = response["result"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty(), "Expected at least one tool");

    // Verify expected tools are present
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(tool_names.contains(&"jarvy_list_tools"));
    assert!(tool_names.contains(&"jarvy_get_tool"));
    assert!(tool_names.contains(&"jarvy_check_tool"));
    assert!(tool_names.contains(&"jarvy_check_multiple"));
    assert!(tool_names.contains(&"jarvy_install_tool"));

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_tools_call_list_tools() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Call jarvy_list_tools
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "jarvy_list_tools",
                "arguments": {}
            }
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let content = &response["result"]["content"];
    assert!(content.is_array());
    assert!(!content.as_array().unwrap().is_empty());

    // The content should contain tool information as text
    let text = content[0]["text"].as_str().unwrap();
    assert!(text.contains("count"), "Response should contain count");

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_tools_call_check_tool_git() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Call jarvy_check_tool for git (commonly installed)
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "jarvy_check_tool",
                "arguments": {
                    "name": "git"
                }
            }
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let content = &response["result"]["content"];
    let text = content[0]["text"].as_str().unwrap();

    // Parse the JSON response
    let check_result: Value = serde_json::from_str(text).unwrap();
    assert_eq!(check_result["name"], "git");
    assert!(check_result["installed"].is_boolean());

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_tools_call_check_unknown_tool() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Call jarvy_check_tool for unknown tool
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "jarvy_check_tool",
                "arguments": {
                    "name": "nonexistent-tool-xyz-12345"
                }
            }
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    // Unknown tools return an error with code -32001 (tool execution failed)
    assert!(
        !response["error"].is_null(),
        "Expected error for unknown tool"
    );
    assert_eq!(response["error"]["code"], -32001);
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Unknown tool")
    );

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_tools_call_get_tool() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Call jarvy_get_tool
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "jarvy_get_tool",
                "arguments": {
                    "name": "ripgrep"
                }
            }
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let content = &response["result"]["content"];
    let text = content[0]["text"].as_str().unwrap();
    let tool_info: Value = serde_json::from_str(text).unwrap();

    assert_eq!(tool_info["name"], "ripgrep");
    assert!(tool_info["command"].is_string());

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_tools_call_check_multiple() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Call jarvy_check_multiple
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "jarvy_check_multiple",
                "arguments": {
                    "tools": ["git", "nonexistent-xyz"]
                }
            }
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let content = &response["result"]["content"];
    let text = content[0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(text).unwrap();

    assert!(result["results"].is_array());
    let results = result["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_tools_call_install_dry_run() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Call jarvy_install_tool with dry_run (default)
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "jarvy_install_tool",
                "arguments": {
                    "name": "ripgrep",
                    "dry_run": true
                }
            }
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let content = &response["result"]["content"];
    let text = content[0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(text).unwrap();

    assert_eq!(result["dry_run"], true);
    assert_eq!(result["name"], "ripgrep");

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_resources_list() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Request resources list
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/list",
            "params": {}
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let resources = response["result"]["resources"].as_array().unwrap();
    assert!(!resources.is_empty());

    let uris: Vec<&str> = resources
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();

    assert!(uris.contains(&"jarvy://tools/index"));
    assert!(uris.contains(&"jarvy://platform/info"));

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_resources_read_platform_info() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Read platform info resource
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "jarvy://platform/info"
            }
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let contents = &response["result"]["contents"];
    assert!(contents.is_array());

    let text = contents[0]["text"].as_str().unwrap();
    let platform_info: Value = serde_json::from_str(text).unwrap();

    assert!(platform_info["os"].is_string());
    assert!(platform_info["arch"].is_string());

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_resources_read_tools_index() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Read tools index resource
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {
                "uri": "jarvy://tools/index"
            }
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let contents = &response["result"]["contents"];
    let text = contents[0]["text"].as_str().unwrap();
    let tools_index: Value = serde_json::from_str(text).unwrap();

    assert!(tools_index["tools"].is_array());
    assert!(!tools_index["tools"].as_array().unwrap().is_empty());

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_prompts_list() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Request prompts list
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "prompts/list",
            "params": {}
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let prompts = response["result"]["prompts"].as_array().unwrap();
    assert!(!prompts.is_empty());

    let names: Vec<&str> = prompts
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();

    assert!(names.contains(&"setup_dev_environment"));
    assert!(names.contains(&"diagnose_missing_tools"));

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_prompts_get_setup_dev_environment() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Get setup_dev_environment prompt
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "prompts/get",
            "params": {
                "name": "setup_dev_environment",
                "arguments": {
                    "project_type": "rust"
                }
            }
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let messages = response["result"]["messages"].as_array().unwrap();
    assert!(!messages.is_empty());

    // Should contain rust-related tools
    let content = messages[0]["content"]["text"].as_str().unwrap();
    assert!(content.to_lowercase().contains("rust"));

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_prompts_get_diagnose_missing_tools() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Get diagnose_missing_tools prompt
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "prompts/get",
            "params": {
                "name": "diagnose_missing_tools"
            }
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(
        response["error"].is_null(),
        "Got error: {:?}",
        response["error"]
    );

    let messages = response["result"]["messages"].as_array().unwrap();
    assert!(!messages.is_empty());

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_method_not_found() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Call unknown method
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "unknown/method",
            "params": {}
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(!response["error"].is_null());
    assert_eq!(response["error"]["code"], -32601); // Method not found

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_unknown_tool_call() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Initialize
    let _ = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05"}
        }),
    );

    // Call unknown tool
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "unknown_tool_xyz",
                "arguments": {}
            }
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(!response["error"].is_null());
    assert_eq!(response["error"]["code"], -32601); // Method not found

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

#[test]
fn test_mcp_ping() {
    let mut child = spawn_mcp_server();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Send ping
    let response = send_request(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping",
            "params": {}
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["error"].is_null());

    drop(stdin);
    let _ = child.wait_timeout(Duration::from_secs(1));
}

/// Trait to add wait_timeout to Child
trait ChildExt {
    fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl ChildExt for std::process::Child {
    fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>> {
        use std::thread;

        let start = std::time::Instant::now();
        loop {
            match self.try_wait()? {
                Some(status) => return Ok(Some(status)),
                None => {
                    if start.elapsed() >= timeout {
                        let _ = self.kill();
                        return Ok(None);
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }
}
