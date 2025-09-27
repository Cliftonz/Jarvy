use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::io::Write;
use std::process::Command;
use tempfile::{NamedTempFile, tempdir};

fn make_config() -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(
        f,
        r#"[privileges]
use_sudo = false

[provisioner]
git = "1.0.0"
"#
    )
    .unwrap();
    f
}

#[test]
fn get_with_output_suppresses_stdout_and_writes_file() {
    let cfg = make_config();
    let out = NamedTempFile::new().unwrap();
    let path = out.path().to_path_buf();
    drop(out); // allow jarvy to open it itself

    let mut c = Command::cargo_bin("jarvy").unwrap();
    c.env("JARVY_TEST_MODE", "1");
    c.args(["get", "--file"])
        .arg(cfg.path())
        .args(["--format", "json", "--output"])
        .arg(&path);
    let assert = c.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    assert!(
        stdout.trim().is_empty(),
        "stdout should be empty when --output is used, got: {}",
        stdout
    );

    let content = std::fs::read_to_string(&path).unwrap();
    serde_json::from_str::<serde_json::Value>(&content).unwrap();
}

#[test]
fn get_write_failure_emits_error_and_exits_success() {
    let cfg = make_config();
    let temp = tempdir().unwrap();
    let dir_path = temp.path();

    let mut c = Command::cargo_bin("jarvy").unwrap();
    c.env("JARVY_TEST_MODE", "1");
    c.args(["get", "--file"])
        .arg(cfg.path())
        .args(["--format", "json", "--output"])
        .arg(dir_path);
    c.assert()
        .success()
        .stderr(predicate::str::contains("Failed to write output:"));
}
