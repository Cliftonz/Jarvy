# PRD-010: CI/CD Detection and Integration

## Overview

Automatically detect CI/CD environments and adjust Jarvy behavior for non-interactive, headless execution in pipelines.

## Problem Statement

Jarvy is designed for interactive local use, but teams need the same configuration to work in CI/CD:
- Interactive prompts block pipelines
- Sudo elevation may not be available
- Terminal colors break log parsing
- Some tools have CI-specific installation methods

Currently, users must manually set environment variables and work around issues.

## Evidence

- README mentions CI integration but lacks specifics
- Interactive prompts in `inquire` block GitHub Actions
- No detection of `CI=true` environment variable
- setup.rs has hardcoded behaviors that break in containers

## Requirements

### Functional Requirements

1. **Auto-detect CI environment**: Recognize GitHub Actions, GitLab CI, etc.
2. **Non-interactive mode**: Skip all prompts, use defaults
3. **CI-specific behavior**: Adjust installation methods
4. **Output formatting**: Machine-readable logs for CI
5. **Exit code handling**: Clear success/failure for pipelines
6. **Cache integration**: Support CI caching mechanisms
7. **CI config generation**: Output GitHub Actions/GitLab CI snippets

### Non-Functional Requirements

1. Zero configuration needed for common CI providers
2. Override detection with `--ci` or `--no-ci` flags
3. Same jarvy.toml works locally and in CI
4. Fast execution (minimize redundant checks)

## CI Environment Detection

### Detected Environments

| Provider | Detection Variables | Notes |
|----------|---------------------|-------|
| GitHub Actions | `GITHUB_ACTIONS=true` | Most common |
| GitLab CI | `GITLAB_CI=true` | |
| CircleCI | `CIRCLECI=true` | |
| Travis CI | `TRAVIS=true` | |
| Azure DevOps | `TF_BUILD=True` | |
| Jenkins | `JENKINS_URL` set | |
| Bitbucket | `BITBUCKET_BUILD_NUMBER` set | |
| Generic | `CI=true` | Catch-all |

### Detection Logic

```rust
// src/ci.rs
#[derive(Debug, Clone, PartialEq)]
pub enum CiProvider {
    GitHubActions,
    GitLabCi,
    CircleCi,
    TravisCi,
    AzureDevOps,
    Jenkins,
    Bitbucket,
    Generic,
    None,
}

impl CiProvider {
    pub fn detect() -> Self {
        if std::env::var("GITHUB_ACTIONS").is_ok() {
            return Self::GitHubActions;
        }
        if std::env::var("GITLAB_CI").is_ok() {
            return Self::GitLabCi;
        }
        if std::env::var("CIRCLECI").is_ok() {
            return Self::CircleCi;
        }
        if std::env::var("TRAVIS").is_ok() {
            return Self::TravisCi;
        }
        if std::env::var("TF_BUILD").is_ok() {
            return Self::AzureDevOps;
        }
        if std::env::var("JENKINS_URL").is_ok() {
            return Self::Jenkins;
        }
        if std::env::var("BITBUCKET_BUILD_NUMBER").is_ok() {
            return Self::Bitbucket;
        }
        if std::env::var("CI").map(|v| v == "true").unwrap_or(false) {
            return Self::Generic;
        }
        Self::None
    }

    pub fn is_ci(&self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn supports_groups(&self) -> bool {
        matches!(self, Self::GitHubActions | Self::GitLabCi | Self::AzureDevOps)
    }

    pub fn cache_dir(&self) -> Option<PathBuf> {
        match self {
            Self::GitHubActions => std::env::var("RUNNER_TOOL_CACHE").ok().map(PathBuf::from),
            Self::GitLabCi => Some(PathBuf::from("/cache")),
            _ => None,
        }
    }
}
```

## CI Mode Behavior Changes

### Prompt Handling

```rust
// src/prompts.rs
pub fn confirm(message: &str, default: bool) -> Result<bool, Error> {
    if CiProvider::detect().is_ci() {
        eprintln!("[CI] Auto-answering '{}': {}", message, default);
        return Ok(default);
    }

    dialoguer::Confirm::new()
        .with_prompt(message)
        .default(default)
        .interact()
}

pub fn select<T: Display>(message: &str, options: &[T], default: usize) -> Result<usize, Error> {
    if CiProvider::detect().is_ci() {
        eprintln!("[CI] Auto-selecting '{}': {}", message, options[default]);
        return Ok(default);
    }

    dialoguer::Select::new()
        .with_prompt(message)
        .items(options)
        .default(default)
        .interact()
}
```

### Output Formatting

```rust
// src/ci/output.rs
pub struct CiOutput {
    provider: CiProvider,
}

impl CiOutput {
    pub fn group_start(&self, name: &str) {
        match self.provider {
            CiProvider::GitHubActions => println!("::group::{}", name),
            CiProvider::GitLabCi => println!("\e[0Ksection_start:{}:{}", timestamp(), name),
            CiProvider::AzureDevOps => println!("##[group]{}", name),
            _ => println!("=== {} ===", name),
        }
    }

    pub fn group_end(&self, name: &str) {
        match self.provider {
            CiProvider::GitHubActions => println!("::endgroup::"),
            CiProvider::GitLabCi => println!("\e[0Ksection_end:{}:{}", timestamp(), name),
            CiProvider::AzureDevOps => println!("##[endgroup]"),
            _ => println!("=== End {} ===", name),
        }
    }

    pub fn warning(&self, message: &str) {
        match self.provider {
            CiProvider::GitHubActions => println!("::warning::{}", message),
            CiProvider::GitLabCi => eprintln!("\x1b[33mWarning: {}\x1b[0m", message),
            CiProvider::AzureDevOps => println!("##[warning]{}", message),
            _ => eprintln!("Warning: {}", message),
        }
    }

    pub fn error(&self, message: &str) {
        match self.provider {
            CiProvider::GitHubActions => println!("::error::{}", message),
            CiProvider::GitLabCi => eprintln!("\x1b[31mError: {}\x1b[0m", message),
            CiProvider::AzureDevOps => println!("##[error]{}", message),
            _ => eprintln!("Error: {}", message),
        }
    }

    pub fn set_output(&self, name: &str, value: &str) {
        match self.provider {
            CiProvider::GitHubActions => {
                if let Ok(path) = std::env::var("GITHUB_OUTPUT") {
                    let _ = std::fs::OpenOptions::new()
                        .append(true)
                        .open(path)
                        .and_then(|mut f| writeln!(f, "{}={}", name, value));
                }
            }
            CiProvider::AzureDevOps => println!("##vso[task.setvariable variable={}]{}", name, value),
            _ => {}
        }
    }
}
```

### CI-Specific Tool Installation

```rust
// src/tools/common.rs
pub fn install_tool(tool: &str, version: &str) -> Result<(), InstallError> {
    let ci = CiProvider::detect();

    // Some tools have CI-optimized installers
    match (tool, &ci) {
        ("node", CiProvider::GitHubActions) => {
            // Use setup-node cache if available
            if std::env::var("RUNNER_TOOL_CACHE").is_ok() {
                return install_from_cache(tool, version);
            }
        }
        ("rust", _) if ci.is_ci() => {
            // Use minimal profile in CI
            return install_rust_minimal(version);
        }
        _ => {}
    }

    // Default installation
    install_default(tool, version)
}
```

## CI Configuration Examples

### GitHub Actions

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  setup:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Cache Jarvy tools
        uses: actions/cache@v4
        with:
          path: |
            ~/.jarvy
            ~/.cargo
            ~/.nvm
          key: jarvy-${{ hashFiles('jarvy.toml') }}

      - name: Install Jarvy
        run: curl -fsSL https://jarvy.dev/install.sh | bash

      - name: Setup environment
        run: jarvy setup
        # CI auto-detected, no prompts

      - name: Verify
        run: jarvy get --format json
```

### GitLab CI

```yaml
# .gitlab-ci.yml
stages:
  - setup
  - test

setup:
  stage: setup
  image: ubuntu:22.04
  cache:
    key: jarvy-${CI_COMMIT_REF_SLUG}
    paths:
      - .jarvy-cache/
  before_script:
    - curl -fsSL https://jarvy.dev/install.sh | bash
  script:
    - jarvy setup
    - jarvy get
  artifacts:
    reports:
      dotenv: jarvy.env

test:
  stage: test
  dependencies:
    - setup
  script:
    - npm test
```

## Config Override for CI

```toml
# jarvy.toml

[tools]
node = "20"
docker = "latest"

# CI-specific overrides
[ci]
# Skip tools that don't work in containers
skip_tools = ["docker"]  # Docker-in-Docker is complex

# Override versions for CI
[ci.tools]
node = "20.10.0"  # Pin exact version for reproducibility

# CI-specific environment
[ci.env]
NODE_ENV = "ci"
CI = "true"

# Disable interactive features
[ci.config]
no_color = true
no_progress = true
```

## CLI Flags

```bash
# Force CI mode
jarvy setup --ci

# Force interactive mode (override detection)
jarvy setup --no-ci

# Output in machine-readable format
jarvy get --format json

# Generate CI config
jarvy ci-config github  # Outputs .github/workflows/jarvy.yml
jarvy ci-config gitlab  # Outputs .gitlab-ci.yml snippet
```

## Implementation Steps

1. Create `src/ci.rs` with provider detection
2. Add CI output formatting helpers
3. Modify prompt functions to auto-answer in CI
4. Add `[ci]` section to config parsing
5. Implement CI-specific tool installation paths
6. Add `--ci` and `--no-ci` CLI flags
7. Create `jarvy ci-config` command for config generation
8. Update all interactive code paths
9. Add integration tests with CI env vars
10. Document CI usage in guides

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| CI auto-detection | None | 8 providers |
| Interactive prompts in CI | Block | Auto-answered |
| CI-specific docs | Minimal | Comprehensive |
| Config generation | None | 4 providers |

## Risks

1. **False positive detection**: Non-CI env sets `CI=true`
   - Mitigation: Allow `--no-ci` override
2. **Provider-specific bugs**: Each CI has quirks
   - Mitigation: Test on each major provider
3. **Breaking changes**: CI behavior differs from local
   - Mitigation: Document differences clearly

## Dependencies

None - uses only environment variable detection.

## Effort Estimate

- Provider detection: 0.5 days
- Output formatting: 0.5 days
- Prompt handling: 0.5 days
- Config override: 0.5 days
- CI config generation: 1 day
- Testing: 1 day
- Documentation: 0.5 days

## Files to Create/Modify

- `src/ci.rs` - New module for CI detection
- `src/ci/output.rs` - CI-specific output formatting
- `src/ci/config.rs` - CI config generation
- `src/config.rs` - Add [ci] section parsing
- `src/prompts.rs` - Auto-answer in CI
- `src/main.rs` - Add --ci flags
- `docs/guides/ci-integration.md` - Documentation
