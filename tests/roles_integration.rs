//! Integration tests for role-based configurations (PRD-033)

use std::fs;
use std::process::Command;

/// Helper to run jarvy with arguments
fn jarvy_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_jarvy"))
}

/// Helper to create a temp directory with a config file
fn create_temp_config(content: &str) -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("jarvy.toml");
    fs::write(&config_path, content).expect("Failed to write config");
    temp_dir
}

#[test]
fn test_roles_list_no_roles() {
    let temp = create_temp_config(
        r#"
[provisioner]
git = "latest"
"#,
    );

    let output = jarvy_cmd()
        .args([
            "roles",
            "-f",
            temp.path().join("jarvy.toml").to_str().unwrap(),
            "list",
        ])
        .output()
        .expect("Failed to run jarvy");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No roles defined") || stdout.contains("no roles"),
        "Expected message about no roles, got: {}",
        stdout
    );
}

#[test]
fn test_roles_list_with_roles() {
    let temp = create_temp_config(
        r#"
[provisioner]
git = "latest"

[roles.frontend]
description = "Frontend development tools"
tools = ["node", "bun"]

[roles.backend]
description = "Backend development tools"
tools = ["rust", "go"]
"#,
    );

    let output = jarvy_cmd()
        .args([
            "roles",
            "-f",
            temp.path().join("jarvy.toml").to_str().unwrap(),
            "list",
        ])
        .output()
        .expect("Failed to run jarvy");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("frontend"), "Should list frontend role");
    assert!(stdout.contains("backend"), "Should list backend role");
}

#[test]
fn test_roles_show_basic() {
    let temp = create_temp_config(
        r#"
[provisioner]
git = "latest"

[roles.frontend]
description = "Frontend development tools"
tools = ["node", "bun"]
"#,
    );

    let output = jarvy_cmd()
        .args([
            "roles",
            "-f",
            temp.path().join("jarvy.toml").to_str().unwrap(),
            "show",
            "frontend",
        ])
        .output()
        .expect("Failed to run jarvy");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("frontend"), "Should show role name");
    assert!(
        stdout.contains("Frontend development") || stdout.contains("node"),
        "Should show role details"
    );
}

#[test]
fn test_roles_show_nonexistent() {
    let temp = create_temp_config(
        r#"
[provisioner]
git = "latest"

[roles.frontend]
tools = ["node"]
"#,
    );

    let output = jarvy_cmd()
        .args([
            "roles",
            "-f",
            temp.path().join("jarvy.toml").to_str().unwrap(),
            "show",
            "nonexistent",
        ])
        .output()
        .expect("Failed to run jarvy");

    assert!(!output.status.success(), "Should fail for nonexistent role");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stderr.contains("not found") || stderr.contains("Error") || stdout.contains("not found"),
        "Should show error message, stderr: {}, stdout: {}",
        stderr,
        stdout
    );
}

#[test]
fn test_roles_show_with_inheritance() {
    let temp = create_temp_config(
        r#"
[provisioner]
git = "latest"

[roles.base]
description = "Base tools"
tools = ["git", "docker"]

[roles.frontend]
description = "Frontend development"
extends = "base"
tools = ["node", "bun"]
"#,
    );

    let output = jarvy_cmd()
        .args([
            "roles",
            "-f",
            temp.path().join("jarvy.toml").to_str().unwrap(),
            "show",
            "frontend",
            "--resolved",
        ])
        .output()
        .expect("Failed to run jarvy");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should include inherited tools
    assert!(
        stdout.contains("git") || stdout.contains("docker"),
        "Should show inherited tools"
    );
    assert!(stdout.contains("node"), "Should show direct tools");
}

#[test]
fn test_roles_diff() {
    let temp = create_temp_config(
        r#"
[provisioner]
git = "latest"

[roles.frontend]
tools = ["node", "bun", "git"]

[roles.backend]
tools = ["rust", "go", "git"]
"#,
    );

    let output = jarvy_cmd()
        .args([
            "roles",
            "-f",
            temp.path().join("jarvy.toml").to_str().unwrap(),
            "diff",
            "frontend",
            "backend",
        ])
        .output()
        .expect("Failed to run jarvy");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("frontend"), "Should mention first role");
    assert!(stdout.contains("backend"), "Should mention second role");
}

#[test]
fn test_roles_list_json_output() {
    let temp = create_temp_config(
        r#"
[provisioner]
git = "latest"

[roles.frontend]
description = "Frontend tools"
tools = ["node"]
"#,
    );

    let output = jarvy_cmd()
        .args([
            "roles",
            "-f",
            temp.path().join("jarvy.toml").to_str().unwrap(),
            "list",
            "-F",
            "json",
        ])
        .output()
        .expect("Failed to run jarvy");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");
}

#[test]
fn test_role_assignment_in_config() {
    let temp = create_temp_config(
        r#"
role = "frontend"

[provisioner]
vim = "latest"

[roles.frontend]
tools = ["node", "bun"]
"#,
    );

    // This tests that the config parses correctly with role assignment
    // The actual tool merging is tested in unit tests
    let output = jarvy_cmd()
        .args([
            "roles",
            "-f",
            temp.path().join("jarvy.toml").to_str().unwrap(),
            "show",
            "frontend",
        ])
        .output()
        .expect("Failed to run jarvy");

    assert!(
        output.status.success(),
        "Should succeed with role assignment"
    );
}

#[test]
fn test_multiple_role_assignment() {
    let temp = create_temp_config(
        r#"
role = ["frontend", "devops"]

[provisioner]
git = "latest"

[roles.frontend]
tools = ["node"]

[roles.devops]
tools = ["docker", "kubectl"]
"#,
    );

    // Verify config parses correctly with multiple roles
    let output = jarvy_cmd()
        .args([
            "roles",
            "-f",
            temp.path().join("jarvy.toml").to_str().unwrap(),
            "list",
        ])
        .output()
        .expect("Failed to run jarvy");

    assert!(
        output.status.success(),
        "Should succeed with multiple role assignment"
    );
}

#[test]
fn test_role_inheritance_chain() {
    let temp = create_temp_config(
        r#"
[provisioner]
git = "latest"

[roles.base]
tools = ["git"]

[roles.developer]
extends = "base"
tools = ["docker"]

[roles.senior]
extends = "developer"
tools = ["kubectl"]
"#,
    );

    let output = jarvy_cmd()
        .args([
            "roles",
            "-f",
            temp.path().join("jarvy.toml").to_str().unwrap(),
            "show",
            "senior",
            "--inheritance",
            "--resolved",
        ])
        .output()
        .expect("Failed to run jarvy");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show full inheritance chain
    assert!(
        stdout.contains("base") || stdout.contains("developer"),
        "Should show inheritance chain"
    );
}

#[test]
fn test_setup_role_override_flag() {
    let temp = create_temp_config(
        r#"
role = "frontend"

[provisioner]
git = "latest"

[roles.frontend]
tools = ["node"]

[roles.backend]
tools = ["rust"]
"#,
    );

    // Test that --role flag is accepted (dry run to not actually install)
    let output = jarvy_cmd()
        .args([
            "setup",
            "--dry-run",
            "--role",
            "backend",
            "-f",
            temp.path().join("jarvy.toml").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run jarvy");

    // The command should at least parse without error
    // (actual role override behavior depends on setup implementation)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Either succeeds or fails gracefully (not a parsing error)
    assert!(
        output.status.success() || !stderr.contains("unexpected argument"),
        "Should accept --role flag, stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}
