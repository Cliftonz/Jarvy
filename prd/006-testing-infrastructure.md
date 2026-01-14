# PRD-006: Testing Infrastructure Improvements

## Overview

Build a robust testing infrastructure with mocking capabilities, increase functional coverage from ~40% to 80%, and enable per-platform CI testing.

## Problem Statement

Current testing has significant gaps:

1. **Shallow tests**: Most tool tests only check "no panic", not correctness
2. **No mocking layer**: Can't test without running real package managers
3. **Missing coverage**: Version matching, sudo escalation, per-distro behavior untested
4. **Limited CI**: No matrix testing across platforms

## Evidence

Current test pattern (repeated 42 times):
```rust
#[test]
fn ensure_git_no_panic() {
    let res = ensure("");
    assert!(res.is_ok() || res.is_err());  // Always true!
}
```

Test inventory:
- 119 total tests
- ~52 are panic-safety only
- Estimated 40-50% functional coverage

## Requirements

### Functional Requirements

1. **Command mocking**: Intercept subprocess calls in tests
2. **Package manager simulation**: Fake apt, brew, winget responses
3. **Version output fixtures**: Inject specific version strings
4. **Filesystem mocking**: Test config file handling without real I/O
5. **Error path testing**: Verify error messages and recovery
6. **Integration tests**: End-to-end setup verification

### Non-Functional Requirements

1. Tests complete in <30 seconds for unit, <5 minutes for integration
2. Tests are deterministic and reproducible
3. CI runs on macOS, Linux (Ubuntu, Alpine), Windows
4. Code coverage reports generated

## Mocking Architecture

### Command Mock Layer

```rust
// src/test_utils/mock_command.rs
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    static ref MOCK_COMMANDS: Mutex<HashMap<String, MockResponse>> = Mutex::new(HashMap::new());
}

pub struct MockResponse {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub fn mock_command(cmd: &str, args: &[&str], response: MockResponse) {
    let key = format!("{} {}", cmd, args.join(" "));
    MOCK_COMMANDS.lock().unwrap().insert(key, response);
}

pub fn clear_mocks() {
    MOCK_COMMANDS.lock().unwrap().clear();
}

// Used by common.rs when JARVY_MOCK_COMMANDS=1
pub fn get_mock_response(cmd: &str, args: &[&str]) -> Option<MockResponse> {
    let key = format!("{} {}", cmd, args.join(" "));
    MOCK_COMMANDS.lock().unwrap().remove(&key)
}
```

### Integration with common.rs

```rust
// src/tools/common.rs
pub fn run(cmd: &str, args: &[&str]) -> Result<Output, InstallError> {
    #[cfg(test)]
    if std::env::var("JARVY_MOCK_COMMANDS").is_ok() {
        if let Some(mock) = test_utils::get_mock_response(cmd, args) {
            return Ok(Output {
                stdout: mock.stdout.into_bytes(),
                stderr: mock.stderr.into_bytes(),
                status: ExitStatus::from_raw(mock.exit_code),
            });
        }
    }

    // Real command execution
    Command::new(cmd).args(args).output()...
}
```

### Version Output Fixtures

```rust
// src/test_utils/fixtures.rs
pub fn git_version(version: &str) -> MockResponse {
    MockResponse {
        stdout: format!("git version {}", version),
        stderr: String::new(),
        exit_code: 0,
    }
}

pub fn node_version(version: &str) -> MockResponse {
    MockResponse {
        stdout: format!("v{}", version),
        stderr: String::new(),
        exit_code: 0,
    }
}

pub fn python_version(version: &str) -> MockResponse {
    MockResponse {
        stdout: format!("Python {}", version),
        stderr: String::new(),
        exit_code: 0,
    }
}

pub fn command_not_found(cmd: &str) -> MockResponse {
    MockResponse {
        stdout: String::new(),
        stderr: format!("{}: command not found", cmd),
        exit_code: 127,
    }
}

pub fn permission_denied() -> MockResponse {
    MockResponse {
        stdout: String::new(),
        stderr: "Permission denied".into(),
        exit_code: 1,
    }
}
```

## Test Categories

### 1. Version Matching Tests

```rust
// tests/unit/version_matching.rs
#[test]
fn version_prefix_matching() {
    mock_command("git", &["--version"], git_version("2.44.0"));
    assert!(cmd_satisfies("git", "2.44"));
    assert!(cmd_satisfies("git", "2"));
    assert!(!cmd_satisfies("git", "2.45"));
}

#[test]
fn version_range_matching() {
    mock_command("python", &["--version"], python_version("3.11.5"));
    assert!(version_satisfies("python", ">= 3.10, < 4.0"));
    assert!(!version_satisfies("python", ">= 3.12"));
}

#[test]
fn version_false_positive_prevention() {
    // This was a bug: "2.4" shouldn't match "12.40"
    mock_command("tool", &["--version"], MockResponse {
        stdout: "version 12.40.0".into(),
        ..Default::default()
    });
    assert!(!cmd_satisfies("tool", "2.4"));
}
```

### 2. Package Manager Tests

```rust
// tests/unit/package_managers.rs
#[test]
fn detect_apt_on_debian() {
    mock_command("apt", &["--version"], MockResponse {
        stdout: "apt 2.4.10".into(),
        ..Default::default()
    });
    assert_eq!(detect_linux_pm(), Some(PackageManager::Apt));
}

#[test]
fn apt_install_calls_correct_command() {
    let calls = capture_commands(|| {
        PkgOps::install(PackageManager::Apt, "git", None)
    });

    assert!(calls.contains(&("apt", vec!["update"])));
    assert!(calls.contains(&("apt", vec!["install", "-y", "git"])));
}

#[test]
fn dnf_uses_different_package_names() {
    // Docker is "docker" on dnf but "docker.io" on apt
    let calls = capture_commands(|| {
        tools::docker::ensure("")
    });

    assert!(calls.contains(&("dnf", vec!["install", "-y", "docker"])));
}
```

### 3. Sudo Escalation Tests

```rust
// tests/unit/sudo_escalation.rs
#[test]
fn sudo_fallback_on_permission_denied() {
    mock_command("apt", &["update"], permission_denied());
    mock_command("sudo", &["apt", "update"], MockResponse::success());

    let result = PkgOps::update(PackageManager::Apt, None);
    assert!(result.is_ok());
}

#[test]
fn no_sudo_fallback_when_disabled() {
    mock_command("apt", &["update"], permission_denied());

    let result = PkgOps::update(PackageManager::Apt, Some(false));
    assert!(matches!(result, Err(InstallError::InvalidPermissions(_))));
}

#[test]
fn per_tool_sudo_override() {
    let config = Config::from_str(r#"
        [privileges]
        use_sudo = true

        [tools.git]
        version = "latest"
        use_sudo = false
    "#);

    let git_sudo = config.get_tool_sudo("git");
    assert_eq!(git_sudo, Some(false));
}
```

### 4. Config Parsing Tests

```rust
// tests/unit/config_parsing.rs
#[test]
fn missing_provisioner_section_uses_empty() {
    let config = Config::from_str("[privileges]\nuse_sudo = true");
    assert!(config.tools.is_empty());
}

#[test]
fn unknown_tool_reports_error() {
    let result = Config::from_str(r#"
        [tools]
        nonexistent_tool = "1.0"
    "#);

    // Should parse but warn, or error based on design choice
}

#[test]
fn toml_parse_error_includes_line_number() {
    let result = Config::load(Path::new("tests/fixtures/malformed.toml"));
    match result {
        Err(JarvyError::ConfigError { line, column, message, .. }) => {
            assert_eq!(line, 5);
            assert!(message.contains("expected"));
        }
        _ => panic!("Expected ConfigError"),
    }
}
```

### 5. Integration Tests

```rust
// tests/integration/full_setup.rs
#[test]
#[ignore] // Requires real package managers
fn setup_installs_multiple_tools() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("jarvy.toml"), r#"
        [tools]
        jq = "latest"
        ripgrep = "latest"
    "#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_jarvy"))
        .arg("setup")
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(which::which("jq").is_ok());
    assert!(which::which("rg").is_ok());
}
```

## CI Matrix Configuration

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --lib --no-fail-fast

  integration-tests:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --test '*' --no-fail-fast
        env:
          JARVY_TEST_MODE: 1

  linux-distro-tests:
    strategy:
      matrix:
        distro: [ubuntu, fedora, alpine, arch]
    runs-on: ubuntu-latest
    container: ${{ matrix.distro }}:latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
      - run: cargo test --test 'linux_*'

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/install-action@cargo-llvm-cov
      - run: cargo llvm-cov --lcov --output-path lcov.info
      - uses: codecov/codecov-action@v3
        with:
          files: lcov.info
```

## Test Directory Structure

```
tests/
├── unit/
│   ├── mod.rs
│   ├── version_matching.rs
│   ├── package_managers.rs
│   ├── sudo_escalation.rs
│   ├── config_parsing.rs
│   └── error_handling.rs
├── integration/
│   ├── mod.rs
│   ├── cli_commands.rs
│   ├── full_setup.rs
│   └── parallel_install.rs
├── platform/
│   ├── linux_apt.rs
│   ├── linux_dnf.rs
│   ├── macos_brew.rs
│   └── windows_winget.rs
├── fixtures/
│   ├── valid_config.toml
│   ├── malformed.toml
│   ├── complex_config.toml
│   └── version_outputs/
│       ├── git.txt
│       ├── node.txt
│       └── python.txt
└── common/
    └── mod.rs  # Shared test utilities

src/test_utils/
├── mod.rs
├── mock_command.rs
├── fixtures.rs
└── capture.rs
```

## Implementation Steps

1. Create `src/test_utils/` module with mocking infrastructure
2. Add command interception to `common.rs`
3. Create version output fixtures
4. Write version matching unit tests (20+ tests)
5. Write package manager unit tests (15+ tests)
6. Write sudo escalation tests (10+ tests)
7. Write config parsing edge case tests (10+ tests)
8. Add CI matrix for multi-platform testing
9. Add code coverage reporting
10. Document testing patterns in CONTRIBUTING.md

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Unit tests | 52 | 150+ |
| Integration tests | 15 | 40+ |
| Functional coverage | 40% | 80% |
| Platform CI | 1 (Linux) | 4 (Linux, macOS, Windows, Alpine) |

## Risks

1. **Mock complexity**: Mocking may not match real behavior
   - Mitigation: Also run real integration tests in CI
2. **Flaky tests**: Parallel tests may conflict
   - Mitigation: Use isolated mock state per test
3. **Slow CI**: Matrix testing takes time
   - Mitigation: Parallelize jobs, cache dependencies

## Dependencies

- `mockall` or custom mocking (custom recommended for control)
- `cargo-llvm-cov` for coverage
- GitHub Actions for CI

## Effort Estimate

- Mock infrastructure: 2 days
- Version matching tests: 1 day
- Package manager tests: 1.5 days
- Sudo tests: 0.5 days
- Config tests: 0.5 days
- CI setup: 1 day
- Documentation: 0.5 days

## Files to Create/Modify

- `src/test_utils/mod.rs` - New module
- `src/test_utils/mock_command.rs` - Command mocking
- `src/test_utils/fixtures.rs` - Test fixtures
- `src/tools/common.rs` - Add mock integration
- `tests/unit/*.rs` - New unit tests
- `tests/fixtures/*.toml` - Test config files
- `.github/workflows/test.yml` - CI configuration
- `CONTRIBUTING.md` - Testing documentation
