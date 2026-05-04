use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

fn cmd() -> Command {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin!("jarvy"));
    c.env("JARVY_TEST_MODE", "1");
    c
}

#[test]
fn multiple_unknown_tokens_fall_back_once() {
    let mut c = cmd();
    c.env("JARVY_INIT_PROBE", "1");
    c.args(["foo", "bar", "baz"]);
    c.assert()
        .success()
        .stderr(predicate::str::contains("Unrecognized command: 'foo'"))
        .stderr(predicate::str::contains("TEST: initialize called").not())
        .stdout(predicate::str::contains("TEST: user_select invoked"));
}

#[test]
fn top_level_unknown_flag_is_clap_error() {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin!("jarvy"));
    c.arg("--not-a-flag");
    c.assert()
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("Usage")));
}
