use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;

#[test]
fn configure_writes_default_config_in_cwd() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Command::cargo_bin("jarvy").unwrap();
    c.env("JARVY_TEST_MODE", "1");
    c.current_dir(dir.path()).arg("configure");
    c.assert().success();

    let path = dir.path().join("jarvy.toml");
    let contents = fs::read_to_string(path).unwrap();
    // Ensure the default sections are present
    assert!(contents.contains("[privileges]"));
    assert!(contents.contains("[provisioner]"));
}
