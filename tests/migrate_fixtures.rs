//! Migration-fixture tests.
//!
//! Each subdirectory under `tests/migrate/fixtures/` represents a migration
//! source (Codespaces, DevPod, Brewfile, etc.) and ships a paired
//! `input.<source>` + `expected.jarvy.toml`. The expected file is a hand-
//! curated "gold standard" jarvy.toml that the matching migration guide
//! produces. This test asserts every gold-standard config validates.
//!
//! These fixtures are also consumed by the promptfoo eval harness
//! (`evals/migrate/`) for measuring LLM migration quality.

use std::path::{Path, PathBuf};
use std::process::Command;

const FIXTURES: &[&str] = &[
    "codespaces",
    "devpod",
    "gitpod",
    "dev-containers",
    "vagrant",
    "homebrew-bundle",
    "mise",
    "asdf",
    "nix",
];

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("migrate")
        .join("fixtures")
}

fn run_validate(config: &Path) {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("jarvy"));
    cmd.env("JARVY_TEST_MODE", "1")
        .args(["validate", "--file"])
        .arg(config);
    let output = cmd.output().expect("failed to run jarvy validate");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // jarvy validate currently returns exit 1 for warnings without --strict,
    // so we assert on the human-readable summary line instead. Migration
    // fixtures intentionally exercise warning paths (e.g. version-manager
    // dependencies absent from [provisioner]) — only errors should fail.
    assert!(
        stdout.contains("Validation passed:"),
        "jarvy validate failed for {}\nstdout:\n{}\nstderr:\n{}",
        config.display(),
        stdout,
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn every_expected_jarvy_toml_validates() {
    let root = fixtures_root();
    for name in FIXTURES {
        let expected = root.join(name).join("expected.jarvy.toml");
        assert!(expected.exists(), "missing fixture: {}", expected.display());
        run_validate(&expected);
    }
}

#[test]
fn every_fixture_has_an_input_file() {
    let root = fixtures_root();
    for name in FIXTURES {
        let dir = root.join(name);
        let mut found_input = false;
        for entry in std::fs::read_dir(&dir).expect("fixture dir missing") {
            let path = entry.unwrap().path();
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if file_name.starts_with("input.") {
                found_input = true;
                break;
            }
        }
        assert!(found_input, "no input.* file in {}", dir.display());
    }
}

#[test]
fn every_fixture_dir_listed_exists() {
    let root = fixtures_root();
    for name in FIXTURES {
        let dir = root.join(name);
        assert!(dir.is_dir(), "fixture dir missing: {}", dir.display());
    }
}
