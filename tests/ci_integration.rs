//! Integration tests for CI/CD detection and integration features.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

/// Helper to run jarvy with test mode enabled
fn jarvy_cmd() -> Command {
    let mut c = Command::cargo_bin("jarvy").unwrap();
    c.env("JARVY_TEST_MODE", "1");
    c
}

// =====================================================================
// CI Info Command Tests
// =====================================================================

#[test]
fn ci_info_shows_not_in_ci_when_no_env_vars() {
    let mut c = jarvy_cmd();
    c.arg("ci-info");
    // Clear CI env vars
    c.env_remove("CI");
    c.env_remove("GITHUB_ACTIONS");
    c.env_remove("GITLAB_CI");
    c.env_remove("CIRCLECI");
    c.env_remove("TRAVIS");
    c.env_remove("TF_BUILD");
    c.env_remove("JENKINS_URL");
    c.env_remove("BITBUCKET_BUILD_NUMBER");
    c.env_remove("JARVY_CI");

    c.assert()
        .success()
        .stdout(predicate::str::contains("Not running in a CI environment"));
}

#[test]
fn ci_info_detects_github_actions() {
    let mut c = jarvy_cmd();
    c.arg("ci-info");
    c.env("GITHUB_ACTIONS", "true");
    c.env_remove("JARVY_NO_CI");

    c.assert()
        .success()
        .stdout(predicate::str::contains("GitHub Actions"));
}

#[test]
fn ci_info_detects_gitlab_ci() {
    let mut c = jarvy_cmd();
    c.arg("ci-info");
    c.env("GITLAB_CI", "true");
    c.env_remove("GITHUB_ACTIONS");
    c.env_remove("JARVY_NO_CI");

    c.assert()
        .success()
        .stdout(predicate::str::contains("GitLab CI"));
}

#[test]
fn ci_info_detects_circleci() {
    let mut c = jarvy_cmd();
    c.arg("ci-info");
    c.env("CIRCLECI", "true");
    c.env_remove("GITHUB_ACTIONS");
    c.env_remove("GITLAB_CI");
    c.env_remove("JARVY_NO_CI");

    c.assert()
        .success()
        .stdout(predicate::str::contains("CircleCI"));
}

#[test]
fn ci_info_detects_azure_devops() {
    let mut c = jarvy_cmd();
    c.arg("ci-info");
    c.env("TF_BUILD", "True");
    c.env_remove("GITHUB_ACTIONS");
    c.env_remove("GITLAB_CI");
    c.env_remove("CIRCLECI");
    c.env_remove("JARVY_NO_CI");

    c.assert()
        .success()
        .stdout(predicate::str::contains("Azure DevOps"));
}

#[test]
fn ci_info_detects_jenkins() {
    let mut c = jarvy_cmd();
    c.arg("ci-info");
    c.env("JENKINS_URL", "http://jenkins.example.com");
    c.env_remove("GITHUB_ACTIONS");
    c.env_remove("GITLAB_CI");
    c.env_remove("CIRCLECI");
    c.env_remove("TF_BUILD");
    c.env_remove("JARVY_NO_CI");

    c.assert()
        .success()
        .stdout(predicate::str::contains("Jenkins"));
}

#[test]
fn ci_info_detects_generic_ci() {
    let mut c = jarvy_cmd();
    c.arg("ci-info");
    c.env("CI", "true");
    c.env_remove("GITHUB_ACTIONS");
    c.env_remove("GITLAB_CI");
    c.env_remove("CIRCLECI");
    c.env_remove("TF_BUILD");
    c.env_remove("JENKINS_URL");
    c.env_remove("BITBUCKET_BUILD_NUMBER");
    c.env_remove("JARVY_NO_CI");

    c.assert()
        .success()
        .stdout(predicate::str::contains("Generic CI"));
}

#[test]
fn ci_info_shows_features_for_github_actions() {
    let mut c = jarvy_cmd();
    c.arg("ci-info");
    c.env("GITHUB_ACTIONS", "true");
    c.env_remove("JARVY_NO_CI");

    c.assert()
        .success()
        .stdout(predicate::str::contains("Log groups: true"))
        .stdout(predicate::str::contains("Output vars: true"))
        .stdout(predicate::str::contains("Caching: true"));
}

// =====================================================================
// CI Config Generation Tests
// =====================================================================

#[test]
fn ci_config_generates_github_actions_dry_run() {
    let mut c = jarvy_cmd();
    c.args(["ci-config", "github", "--dry-run"]);

    c.assert()
        .success()
        .stdout(predicate::str::contains(".github/workflows/jarvy.yml"))
        .stdout(predicate::str::contains("actions/checkout"));
}

#[test]
fn ci_config_generates_gitlab_dry_run() {
    let mut c = jarvy_cmd();
    c.args(["ci-config", "gitlab", "--dry-run"]);

    c.assert()
        .success()
        .stdout(predicate::str::contains(".gitlab-ci.yml"))
        .stdout(predicate::str::contains("stages:"));
}

#[test]
fn ci_config_generates_circleci_dry_run() {
    let mut c = jarvy_cmd();
    c.args(["ci-config", "circleci", "--dry-run"]);

    c.assert()
        .success()
        .stdout(predicate::str::contains(".circleci/config.yml"))
        .stdout(predicate::str::contains("version: 2.1"));
}

#[test]
fn ci_config_generates_azure_dry_run() {
    let mut c = jarvy_cmd();
    c.args(["ci-config", "azure", "--dry-run"]);

    c.assert()
        .success()
        .stdout(predicate::str::contains("azure-pipelines.yml"))
        .stdout(predicate::str::contains("stages:"));
}

#[test]
fn ci_config_generates_bitbucket_dry_run() {
    let mut c = jarvy_cmd();
    c.args(["ci-config", "bitbucket", "--dry-run"]);

    c.assert()
        .success()
        .stdout(predicate::str::contains("bitbucket-pipelines.yml"))
        .stdout(predicate::str::contains("pipelines:"));
}

#[test]
fn ci_config_rejects_unsupported_provider() {
    let mut c = jarvy_cmd();
    c.args(["ci-config", "invalid-provider"]);

    c.assert()
        .failure()
        .stderr(predicate::str::contains("Unknown CI provider"));
}

#[test]
fn ci_config_accepts_provider_aliases() {
    // Test that various aliases work
    let aliases = [
        ("gha", "actions/checkout"),
        ("github-actions", "actions/checkout"),
        ("gitlab-ci", "stages:"),
        ("circle", "version: 2.1"),
        ("ado", "azure-pipelines"),
        ("azure-devops", "azure-pipelines"),
    ];

    for (alias, expected_content) in aliases {
        let mut c = jarvy_cmd();
        c.args(["ci-config", alias, "--dry-run"]);
        c.assert()
            .success()
            .stdout(predicate::str::contains(expected_content));
    }
}

// =====================================================================
// Setup Command with CI Flags Tests
// =====================================================================

#[test]
fn setup_help_shows_ci_flags() {
    let mut c = jarvy_cmd();
    c.args(["setup", "--help"]);

    c.assert()
        .success()
        .stdout(predicate::str::contains("--ci"))
        .stdout(predicate::str::contains("--no-ci"));
}

#[test]
fn setup_dry_run_shows_ci_mode() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a temporary config file
    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(config_file, "[provisioner]").unwrap();
    writeln!(config_file, "git = \"*\"").unwrap();

    let mut c = jarvy_cmd();
    c.args(["setup", "--dry-run", "--ci", "-f"]);
    c.arg(config_file.path());
    c.env_remove("GITHUB_ACTIONS");
    c.env_remove("GITLAB_CI");
    c.env_remove("CI");

    c.assert()
        .success()
        .stdout(predicate::str::contains("CI mode"));
}

// =====================================================================
// JARVY_CI and JARVY_NO_CI Environment Variable Tests
// =====================================================================

#[test]
fn jarvy_ci_env_forces_ci_mode() {
    let mut c = jarvy_cmd();
    c.arg("ci-info");
    c.env("JARVY_CI", "1");
    c.env_remove("CI");
    c.env_remove("GITHUB_ACTIONS");

    c.assert()
        .success()
        .stdout(predicate::str::contains("Forced: true"));
}

#[test]
fn jarvy_no_ci_env_disables_ci_detection() {
    let mut c = jarvy_cmd();
    c.arg("ci-info");
    c.env("CI", "true");
    c.env("JARVY_NO_CI", "1");

    c.assert()
        .success()
        .stdout(predicate::str::contains("Not running in a CI environment"));
}
