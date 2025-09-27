use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn missing_config_exits_with_failure_and_message() {
    let mut c = Command::cargo_bin("jarvy").unwrap();
    c.env("JARVY_TEST_MODE", "1");
    c.args(["get", "--file", "/definitely/missing/file.toml"]);
    // Message is printed to stdout in current implementation
    c.assert()
        .failure()
        .stdout(predicate::str::contains("Failed to read config file"));
}

#[test]
fn malformed_config_exits_with_failure_and_parse_message() {
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), "[provisioner]\nthis = not_toml\n").unwrap();

    let mut c = Command::cargo_bin("jarvy").unwrap();
    c.env("JARVY_TEST_MODE", "1");
    c.args(["get", "--file"]).arg(tmp.path());
    c.assert()
        .failure()
        .stdout(predicate::str::contains("Failed to parse config file"));
}
