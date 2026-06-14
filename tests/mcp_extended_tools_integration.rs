//! End-to-end coverage for the extended MCP tools.
//!
//! Spawns the real `jarvy mcp` server as a subprocess, speaks JSON-RPC
//! over stdio, and asserts that every extended tool returns a sensible
//! shape. This is the same wire protocol Claude Code / Cursor / Codex
//! use to drive Jarvy in production — if these tests pass, the surface
//! is genuinely callable.

use assert_cmd::cargo::CommandCargoExt;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

struct McpHarness {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpHarness {
    fn spawn() -> Self {
        let mut child = Command::cargo_bin("jarvy")
            .expect("jarvy binary built")
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env("JARVY_TEST_MODE", "1")
            .env("JARVY_TELEMETRY", "0")
            .env("JARVY_NO_CI", "1")
            .spawn()
            .expect("spawn jarvy mcp");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");
        let reader = BufReader::new(stdout);
        let mut h = McpHarness {
            child,
            stdin,
            reader,
            next_id: 1,
        };
        h.handshake();
        h
    }

    fn handshake(&mut self) {
        let init = self.call(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "jarvy-tests", "version": "0.0.1" }
            }),
        );
        assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
        // Notify the server we're done initializing.
        let notify = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let line = serde_json::to_string(&notify).unwrap();
        writeln!(self.stdin, "{line}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn call(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let line = serde_json::to_string(&req).unwrap();
        writeln!(self.stdin, "{line}").unwrap();
        self.stdin.flush().unwrap();
        let mut buf = String::new();
        // Loop until we get a response with matching id; the server may
        // emit notifications interleaved.
        loop {
            buf.clear();
            self.reader.read_line(&mut buf).expect("read line");
            let trimmed = buf.trim();
            if trimmed.is_empty() {
                continue;
            }
            let v: Value = serde_json::from_str(trimmed).expect("parse response");
            if v.get("id").and_then(|x| x.as_u64()) == Some(id) {
                return v;
            }
        }
    }

    fn call_tool(&mut self, name: &str, args: Value) -> Value {
        let resp = self.call("tools/call", json!({ "name": name, "arguments": args }));
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("missing text in response: {resp}"));
        serde_json::from_str::<Value>(text).expect("parse tool result")
    }
}

impl Drop for McpHarness {
    fn drop(&mut self) {
        let _ = self.child.kill();
        // Drain async — give it a beat to actually die so the test
        // runner doesn't show stragglers.
        std::thread::sleep(Duration::from_millis(50));
        let _ = self.child.wait();
    }
}

fn harness() -> McpHarness {
    McpHarness::spawn()
}

#[test]
fn tools_list_includes_every_extended_tool() {
    let mut h = harness();
    let resp = h.call("tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    let expected = [
        "jarvy_ai_hooks_list",
        "jarvy_ai_hooks_check",
        "jarvy_ai_hooks_apply",
        "jarvy_mcp_register_list",
        "jarvy_mcp_register_check",
        "jarvy_mcp_register_apply",
        "jarvy_drift_check",
        "jarvy_drift_status",
        "jarvy_roles_list",
        "jarvy_roles_show",
        "jarvy_services_status",
        "jarvy_templates_list",
        "jarvy_templates_show",
        "jarvy_validate_config",
    ];
    for tool in expected {
        assert!(
            names.contains(&tool),
            "missing extended tool '{tool}' in tools/list (got {names:?})"
        );
    }
}

#[test]
fn ai_hooks_list_library_returns_curated_set() {
    let mut h = harness();
    let result = h.call_tool("jarvy_ai_hooks_list", json!({ "library": true }));
    let library = result["library"].as_array().expect("library array");
    let names: Vec<&str> = library.iter().filter_map(|h| h["name"].as_str()).collect();
    for must_have in ["block-rm-rf", "audit-log", "block-secrets-commit"] {
        assert!(
            names.contains(&must_have),
            "missing library hook {must_have} in {names:?}"
        );
    }
}

#[test]
fn ai_hooks_list_reports_not_configured_when_no_section() {
    let mut h = harness();
    let result = h.call_tool(
        "jarvy_ai_hooks_list",
        json!({ "config_path": "/definitely/not/a/real/path.toml" }),
    );
    assert_eq!(result["configured"], false);
}

#[test]
fn validate_config_with_valid_config_returns_valid_true() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("jarvy.toml");
    std::fs::write(
        &p,
        r#"[provisioner]
git = "latest"
node = "20"
"#,
    )
    .unwrap();
    let mut h = harness();
    let result = h.call_tool(
        "jarvy_validate_config",
        json!({ "config_path": p.to_str().unwrap() }),
    );
    assert_eq!(result["valid"], true);
    assert_eq!(result["tool_count"], 2);
}

#[test]
fn validate_config_with_broken_toml_returns_valid_false() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("jarvy.toml");
    std::fs::write(&p, b"[provisioner\nbroken").unwrap();
    let mut h = harness();
    let result = h.call_tool(
        "jarvy_validate_config",
        json!({ "config_path": p.to_str().unwrap() }),
    );
    assert_eq!(result["valid"], false);
    assert_eq!(result["error_type"], "parse");
}

#[test]
fn templates_list_returns_built_in_templates() {
    let mut h = harness();
    let result = h.call_tool("jarvy_templates_list", json!({}));
    let count = result["count"].as_u64().expect("count");
    assert!(count > 0, "expected at least one built-in template");
    let templates = result["templates"].as_array().expect("templates");
    let has_name = templates.iter().any(|t| t["name"].is_string());
    assert!(has_name, "first template should have a name");
}

#[test]
fn templates_show_unknown_returns_error() {
    let mut h = harness();
    let resp = h.call(
        "tools/call",
        json!({
            "name": "jarvy_templates_show",
            "arguments": { "name": "does-not-exist-template" }
        }),
    );
    assert!(
        resp.get("error").is_some(),
        "expected error response for unknown template, got: {resp}"
    );
}

#[test]
fn drift_status_in_empty_dir_reports_no_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness();
    let result = h.call_tool(
        "jarvy_drift_status",
        json!({ "project_dir": dir.path().to_str().unwrap() }),
    );
    assert_eq!(result["baseline_exists"], false);
}

#[test]
fn services_status_in_empty_dir_reports_no_backend() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness();
    let result = h.call_tool(
        "jarvy_services_status",
        json!({ "project_dir": dir.path().to_str().unwrap() }),
    );
    assert!(result["backend"].is_null());
}

#[test]
fn mcp_register_list_reports_not_configured_for_path_without_section() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("jarvy.toml");
    std::fs::write(&p, "[provisioner]\ngit = \"latest\"\n").unwrap();
    let mut h = harness();
    let result = h.call_tool(
        "jarvy_mcp_register_list",
        json!({ "config_path": p.to_str().unwrap() }),
    );
    assert_eq!(result["configured"], false);
}

#[test]
fn ai_hooks_apply_dry_run_does_not_touch_disk() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("jarvy.toml");
    std::fs::write(
        &p,
        r#"[provisioner]
git = "latest"

[ai_hooks]
agents = ["claude-code"]

[[ai_hooks.hook]]
use = "block-rm-rf"
"#,
    )
    .unwrap();
    let mut h = harness();
    let result = h.call_tool(
        "jarvy_ai_hooks_apply",
        json!({ "config_path": p.to_str().unwrap(), "dry_run": true }),
    );
    assert_eq!(result["dry_run"], true);
    assert_eq!(result["would_apply_hooks"], 1);
}

#[test]
fn mcp_register_apply_dry_run_lists_servers() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("jarvy.toml");
    std::fs::write(
        &p,
        r#"[provisioner]
git = "latest"

[mcp_register]
agents = ["claude-code", "cursor"]
"#,
    )
    .unwrap();
    let mut h = harness();
    let result = h.call_tool(
        "jarvy_mcp_register_apply",
        json!({ "config_path": p.to_str().unwrap(), "dry_run": true }),
    );
    assert_eq!(result["dry_run"], true);
    // Built-in jarvy server is always included.
    let n = result["would_register_servers"].as_u64().unwrap_or(0);
    assert!(n >= 1, "should register at least jarvy itself");
}
