use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn make_config() -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    // minimal valid config for this project
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
fn get_known_command_prints_json_yaml_toml_pretty() {
    let cfg = make_config();

    for fmt in ["json", "yaml", "toml", "pretty"] {
        let mut c = Command::cargo_bin("jarvy").unwrap();
        c.env("JARVY_TEST_MODE", "1"); // ensure no interactive prompts leak
        c.args(["get", "--file"])
            .arg(cfg.path())
            .args(["--format", fmt]);
        let assert = c.assert().success();
        match fmt {
            "json" => {
                assert.stdout(
                    predicate::str::contains("\"tools\"").and(predicate::str::contains("git")),
                );
            }
            "yaml" => {
                assert.stdout(
                    predicate::str::contains("tools:").and(predicate::str::contains("git")),
                );
            }
            "toml" => {
                assert.stdout(
                    predicate::str::contains("[tools]")
                        .or(predicate::str::contains("tools ="))
                        .or(predicate::str::contains("tools\n")),
                );
            }
            "pretty" => {
                assert.stdout(predicate::str::contains("Tools status"));
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn get_writes_output_file_when_requested() {
    let cfg = make_config();
    let outfile = NamedTempFile::new().unwrap();
    let pathbuf = outfile.path().to_path_buf();
    // Close and allow jarvy to write
    drop(outfile);

    let mut c = Command::cargo_bin("jarvy").unwrap();
    c.env("JARVY_TEST_MODE", "1");
    c.args(["get", "--file"])
        .arg(cfg.path())
        .args(["--format", "json", "--output"])
        .arg(&pathbuf);
    c.assert().success();

    let contents = std::fs::read_to_string(&pathbuf).unwrap();
    assert!(contents.contains("\"tools\""));
}

#[test]
fn unknown_never_writes_output_file_even_if_arg_present() {
    let outfile = NamedTempFile::new().unwrap();
    let pathbuf = outfile.path().to_path_buf();
    drop(outfile);

    let mut c = Command::cargo_bin("jarvy").unwrap();
    c.env("JARVY_TEST_MODE", "1");
    c.args(["frobnicate", "--output", pathbuf.to_str().unwrap()]);
    c.assert()
        .success()
        .stdout(predicate::str::contains("TEST: user_select invoked"));

    assert!(
        std::fs::read_to_string(&pathbuf).is_err(),
        "unknown path should not create files"
    );
}

#[test]
fn known_command_does_not_invoke_user_select() {
    let cfg = make_config();
    let mut c = Command::cargo_bin("jarvy").unwrap();
    c.env("JARVY_TEST_MODE", "1");
    c.args(["get", "--file"]).arg(cfg.path());
    c.assert()
        .success()
        .stdout(predicate::str::contains("TEST: user_select invoked").not());
}
