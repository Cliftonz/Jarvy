//! CLI e2e tests for `jarvy mcp-register`.
//!
//! Uses `assert_cmd` to spawn the built binary so the same parser +
//! dispatch wiring that ships to users is exercised. HOME is redirected
//! at a tempdir so the developer's real agent configs are never touched.

use assert_cmd::cargo::CommandCargoExt;
use assert_cmd::prelude::OutputAssertExt;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

struct HomeGuard {
    _tmp: TempDir,
    _previous: Vec<(String, Option<String>)>,
}

impl HomeGuard {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        let keys = ["HOME", "USERPROFILE"];
        let mut previous = Vec::new();
        for key in keys {
            previous.push((key.to_string(), std::env::var(key).ok()));
            #[allow(unsafe_code)]
            unsafe {
                std::env::set_var(key, &path);
            }
        }
        HomeGuard {
            _tmp: tmp,
            _previous: previous,
        }
    }

    fn path(&self) -> &std::path::Path {
        self._tmp.path()
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        for (key, value) in &self._previous {
            #[allow(unsafe_code)]
            unsafe {
                match value {
                    Some(v) => std::env::set_var(key, v),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

fn write_config(dir: &TempDir, body: &str) -> String {
    let p = dir.path().join("jarvy.toml");
    fs::write(&p, body).unwrap();
    p.to_string_lossy().into_owned()
}

fn cmd(file: &str, args: &[&str]) -> Command {
    let mut c = Command::cargo_bin("jarvy").unwrap();
    c.arg("mcp-register");
    c.arg("--file").arg(file);
    c.args(args);
    c.env("JARVY_TEST_MODE", "1");
    // Inherit the redirected HOME so the binary sees the sandbox.
    if let Ok(home) = std::env::var("HOME") {
        c.env("HOME", home);
    }
    if let Ok(up) = std::env::var("USERPROFILE") {
        c.env("USERPROFILE", up);
    }
    c
}

#[test]
#[serial_test::serial(home_env)]
fn list_with_no_section_returns_0() {
    let _guard = HomeGuard::new();
    let dir = TempDir::new().unwrap();
    let f = write_config(&dir, "[provisioner]\ngit = \"latest\"\n");
    cmd(&f, &["list"])
        .assert()
        .success()
        .stderr(predicate::str::contains("No [mcp_register] section"));
}

#[test]
#[serial_test::serial(home_env)]
fn list_with_section_prints_jarvy_built_in() {
    let _guard = HomeGuard::new();
    let dir = TempDir::new().unwrap();
    let f = write_config(
        &dir,
        r#"[provisioner]
git = "latest"

[mcp_register]
agents = ["claude-code"]
"#,
    );
    cmd(&f, &["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("jarvy: built-in"));
}

#[test]
#[serial_test::serial(home_env)]
fn apply_writes_settings_and_exits_0() {
    let guard = HomeGuard::new();
    let dir = TempDir::new().unwrap();
    let f = write_config(
        &dir,
        r#"[provisioner]
git = "latest"

[mcp_register]
agents = ["claude-code"]
"#,
    );
    cmd(&f, &["apply"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Registered 1 server"));
    let body = fs::read_to_string(guard.path().join(".claude.json")).expect("written");
    assert!(body.contains("\"jarvy\""));
}

#[test]
#[serial_test::serial(home_env)]
fn check_returns_1_when_drift_detected() {
    let guard = HomeGuard::new();
    let dir = TempDir::new().unwrap();
    let f = write_config(
        &dir,
        r#"[provisioner]
git = "latest"

[mcp_register]
agents = ["claude-code"]
"#,
    );
    // Apply, then corrupt the settings to simulate drift.
    cmd(&f, &["apply"]).assert().success();
    let settings = guard.path().join(".claude.json");
    fs::write(&settings, b"{}").unwrap();
    cmd(&f, &["check"])
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("DRIFT"));
}

#[test]
#[serial_test::serial(home_env)]
fn check_returns_0_when_clean() {
    let _guard = HomeGuard::new();
    let dir = TempDir::new().unwrap();
    let f = write_config(
        &dir,
        r#"[provisioner]
git = "latest"

[mcp_register]
agents = ["claude-code"]
"#,
    );
    cmd(&f, &["apply"]).assert().success();
    cmd(&f, &["check"]).assert().success();
}

#[test]
#[serial_test::serial(home_env)]
fn remove_strips_jarvy_entry() {
    let guard = HomeGuard::new();
    let dir = TempDir::new().unwrap();
    let f = write_config(
        &dir,
        r#"[provisioner]
git = "latest"

[mcp_register]
agents = ["claude-code"]
"#,
    );
    cmd(&f, &["apply"]).assert().success();
    cmd(&f, &["remove"]).assert().success();
    let body = fs::read_to_string(guard.path().join(".claude.json")).unwrap();
    assert!(!body.contains("\"jarvy\""));
}

#[test]
#[serial_test::serial(home_env)]
fn scope_override_via_cli_takes_precedence() {
    let _guard = HomeGuard::new();
    let dir = TempDir::new().unwrap();
    let f = write_config(
        &dir,
        r#"[provisioner]
git = "latest"

[mcp_register]
agents = ["claude-code"]
scope = "user"
"#,
    );
    // Run the binary with cwd set to a fresh tempdir so project-scope
    // writes don't land in the repo root and trip up git status.
    let cwd = TempDir::new().unwrap();
    let mut c = cmd(&f, &["apply", "--scope", "project"]);
    c.current_dir(cwd.path());
    c.assert().success();
    assert!(
        cwd.path().join(".mcp.json").exists(),
        "project-scope override should have produced .mcp.json in the cwd"
    );
}
