# PRD-048: Pre-Commit Hook Installation

## Overview

Enable Jarvy to install and configure Git pre-commit hooks as part of environment setup, ensuring consistent code quality checks across team members without manual hook installation.

## Problem Statement

Pre-commit hooks are essential for code quality but often neglected:

- Developers must manually run `pre-commit install` after cloning
- New team members don't know hooks exist
- Hooks drift between team members (different versions)
- CI catches issues that should have been caught locally
- Teams use different hook frameworks (pre-commit, husky, lefthook)

Automating hook installation ensures consistent quality gates from day one.

## Evidence

- "Install pre-commit hooks" in every onboarding doc
- CI fails for formatting issues that hooks would catch
- "Works on my machine" due to different hook configurations
- Manual hook installation often forgotten or skipped
- Hook-related issues discovered late in review cycle

## Requirements

### Functional Requirements

1. **Framework detection**: Detect pre-commit, husky, lefthook configs
2. **Auto-installation**: Install hooks during setup
3. **Multi-framework**: Support multiple hook frameworks
4. **Version management**: Ensure consistent hook versions
5. **Update hooks**: Update existing hooks when configs change
6. **Skip option**: Allow skipping hook installation

### Non-Functional Requirements

1. **Non-destructive**: Don't overwrite custom hooks
2. **Idempotent**: Safe to run multiple times
3. **Fast**: Hook installation <5 seconds
4. **Offline capable**: Work with cached hook configs
5. **Framework agnostic**: Work with any git hooks

## Non-Goals

- Writing hook configurations (use .pre-commit-config.yaml)
- Custom hook logic (use existing frameworks)
- CI-specific hook behavior
- Commit message validation (framework responsibility)
- Hook execution optimization

## Feature Specifications

### 1. Configuration Syntax

```toml
# jarvy.toml

[hooks]
# Enable git hook management
git_hooks = true

# Hook framework to use
framework = "pre-commit"  # pre-commit, husky, lefthook, native

# Auto-install hooks during setup
auto_install = true

# Update hooks when config changes
auto_update = true

# Run hooks after installation (for initial check)
run_after_install = false

[hooks.pre-commit]
# pre-commit specific settings
version = "3.6.0"         # Pin pre-commit version
config = ".pre-commit-config.yaml"  # Config file path
install_hooks = true      # Run pre-commit install --install-hooks

[hooks.husky]
# husky specific settings
version = "9.0.0"
package_manager = "npm"   # npm, yarn, pnpm

[hooks.lefthook]
# lefthook specific settings
version = "1.6.0"
config = "lefthook.yml"
```

### 2. Framework-Specific Configurations

```yaml
# .pre-commit-config.yaml (existing format, not jarvy.toml)
repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.5.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
      - id: check-yaml

  - repo: https://github.com/psf/black
    rev: 24.1.0
    hooks:
      - id: black

  - repo: local
    hooks:
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt --
        language: system
        types: [rust]
```

```json
// package.json (husky setup)
{
  "scripts": {
    "prepare": "husky"
  },
  "devDependencies": {
    "husky": "^9.0.0"
  }
}
```

```yaml
# lefthook.yml
pre-commit:
  parallel: true
  commands:
    lint:
      run: npm run lint
    format:
      run: cargo fmt --check
```

### 3. CLI Commands

```bash
# Install hooks (automatic during setup)
jarvy setup

# Output:
# ...tool installation...
#
# Git Hooks
# =========
# Detected: pre-commit (from .pre-commit-config.yaml)
# Installing pre-commit hooks...
#   ✓ pre-commit installed (3.6.0)
#   ✓ Hooks installed to .git/hooks/
#   ✓ Hook environments cached
#
# Installed hooks:
#   pre-commit: trailing-whitespace, end-of-file-fixer, check-yaml, black, cargo-fmt

# Explicit hook installation
jarvy hooks install

# Update hooks
jarvy hooks update

# Output:
# Updating git hooks...
#   ✓ pre-commit updated (3.5.0 → 3.6.0)
#   ✓ Hook repos updated
#   ✓ Environments refreshed

# List installed hooks
jarvy hooks list

# Output:
# Installed Git Hooks
# ===================
# Framework: pre-commit (3.6.0)
# Config: .pre-commit-config.yaml
#
# pre-commit:
#   trailing-whitespace    (pre-commit-hooks v4.5.0)
#   end-of-file-fixer      (pre-commit-hooks v4.5.0)
#   check-yaml             (pre-commit-hooks v4.5.0)
#   black                  (black 24.1.0)
#   cargo-fmt              (local)
#
# commit-msg:
#   commitlint             (commitlint v18.0.0)

# Check hook status
jarvy hooks status

# Output:
# Git Hooks Status
# ================
# Framework: pre-commit
# Installed: ✓ Yes
# Up to date: ✓ Yes
# Config: .pre-commit-config.yaml (unchanged)
# Last update: 2024-01-15

# Uninstall hooks
jarvy hooks uninstall

# Run hooks manually
jarvy hooks run
jarvy hooks run --all-files  # Run on all files
jarvy hooks run --hook black  # Run specific hook
```

### 4. Framework Detection

```rust
// Detection priority
enum HookFramework {
    PreCommit,  // .pre-commit-config.yaml
    Husky,      // .husky/ directory or package.json husky config
    Lefthook,   // lefthook.yml or lefthook.yaml
    Native,     // Raw .git/hooks scripts
}

// Detection rules
fn detect_framework(project_dir: &Path) -> Option<HookFramework> {
    if project_dir.join(".pre-commit-config.yaml").exists() {
        return Some(HookFramework::PreCommit);
    }

    if project_dir.join(".husky").is_dir() {
        return Some(HookFramework::Husky);
    }

    let package_json = project_dir.join("package.json");
    if package_json.exists() {
        if let Ok(content) = std::fs::read_to_string(&package_json) {
            if content.contains("\"husky\"") {
                return Some(HookFramework::Husky);
            }
        }
    }

    if project_dir.join("lefthook.yml").exists()
        || project_dir.join("lefthook.yaml").exists()
    {
        return Some(HookFramework::Lefthook);
    }

    None
}
```

## Technical Approach

### Module Structure

```
src/
  git_hooks/
    mod.rs           # Public API
    config.rs        # Hook configuration
    detection.rs     # Framework detection
    precommit.rs     # pre-commit framework
    husky.rs         # husky framework
    lefthook.rs      # lefthook framework
    native.rs        # Native git hooks
    commands.rs      # CLI command handlers
```

### Configuration Types

```rust
// src/git_hooks/config.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HooksConfig {
    /// Enable git hook management
    #[serde(default = "default_true")]
    pub git_hooks: bool,

    /// Hook framework to use
    pub framework: Option<HookFramework>,

    /// Auto-install during setup
    #[serde(default = "default_true")]
    pub auto_install: bool,

    /// Auto-update when config changes
    #[serde(default)]
    pub auto_update: bool,

    /// Run hooks after installation
    #[serde(default)]
    pub run_after_install: bool,

    /// Framework-specific configs
    #[serde(default)]
    pub pre_commit: Option<PreCommitConfig>,
    #[serde(default)]
    pub husky: Option<HuskyConfig>,
    #[serde(default)]
    pub lefthook: Option<LefthookConfig>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum HookFramework {
    PreCommit,
    Husky,
    Lefthook,
    Native,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PreCommitConfig {
    /// Pin pre-commit version
    pub version: Option<String>,

    /// Config file path
    #[serde(default = "default_precommit_config")]
    pub config: String,

    /// Run pre-commit install --install-hooks
    #[serde(default = "default_true")]
    pub install_hooks: bool,
}

fn default_precommit_config() -> String {
    ".pre-commit-config.yaml".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HuskyConfig {
    pub version: Option<String>,
    #[serde(default = "default_npm")]
    pub package_manager: String,
}

fn default_npm() -> String {
    "npm".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LefthookConfig {
    pub version: Option<String>,
    #[serde(default = "default_lefthook_config")]
    pub config: String,
}

fn default_lefthook_config() -> String {
    "lefthook.yml".to_string()
}
```

### Pre-commit Handler

```rust
// src/git_hooks/precommit.rs
use std::process::Command;

pub struct PreCommitHandler {
    config: PreCommitConfig,
    project_dir: PathBuf,
}

impl PreCommitHandler {
    pub fn install(&self) -> Result<(), HookError> {
        // Check if pre-commit is installed
        if !self.is_installed() {
            return Err(HookError::FrameworkNotInstalled("pre-commit".to_string()));
        }

        // Check version if pinned
        if let Some(ref required) = self.config.version {
            let installed = self.get_version()?;
            if !version_satisfies(&installed, required) {
                println!("  Upgrading pre-commit {} → {}", installed, required);
                self.upgrade(required)?;
            }
        }

        // Run pre-commit install
        println!("  Installing pre-commit hooks...");
        let mut cmd = Command::new("pre-commit");
        cmd.arg("install");
        cmd.current_dir(&self.project_dir);

        if self.config.install_hooks {
            cmd.arg("--install-hooks");
        }

        let status = cmd.status()?;
        if !status.success() {
            return Err(HookError::InstallFailed("pre-commit install failed".to_string()));
        }

        println!("  ✓ pre-commit hooks installed");
        Ok(())
    }

    pub fn update(&self) -> Result<(), HookError> {
        println!("  Updating pre-commit hooks...");

        // Update hook repos
        let status = Command::new("pre-commit")
            .args(["autoupdate"])
            .current_dir(&self.project_dir)
            .status()?;

        if !status.success() {
            return Err(HookError::UpdateFailed("pre-commit autoupdate failed".to_string()));
        }

        // Reinstall hooks
        let status = Command::new("pre-commit")
            .args(["install", "--install-hooks"])
            .current_dir(&self.project_dir)
            .status()?;

        if !status.success() {
            return Err(HookError::UpdateFailed("pre-commit install failed".to_string()));
        }

        println!("  ✓ Hooks updated");
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<HookInfo>, HookError> {
        // Parse .pre-commit-config.yaml to list hooks
        let config_path = self.project_dir.join(&self.config.config);
        let content = std::fs::read_to_string(&config_path)?;
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;

        let mut hooks = Vec::new();

        if let Some(repos) = yaml.get("repos").and_then(|r| r.as_sequence()) {
            for repo in repos {
                let repo_url = repo.get("repo").and_then(|r| r.as_str()).unwrap_or("local");
                let rev = repo.get("rev").and_then(|r| r.as_str()).unwrap_or("");

                if let Some(repo_hooks) = repo.get("hooks").and_then(|h| h.as_sequence()) {
                    for hook in repo_hooks {
                        if let Some(id) = hook.get("id").and_then(|i| i.as_str()) {
                            hooks.push(HookInfo {
                                id: id.to_string(),
                                repo: repo_url.to_string(),
                                version: rev.to_string(),
                                hook_type: "pre-commit".to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(hooks)
    }

    pub fn run(&self, all_files: bool, hook_id: Option<&str>) -> Result<(), HookError> {
        let mut cmd = Command::new("pre-commit");
        cmd.arg("run");
        cmd.current_dir(&self.project_dir);

        if all_files {
            cmd.arg("--all-files");
        }

        if let Some(id) = hook_id {
            cmd.arg(id);
        } else {
            cmd.arg("--all");
        }

        let status = cmd.status()?;
        if !status.success() {
            return Err(HookError::RunFailed("Hooks failed".to_string()));
        }

        Ok(())
    }

    fn is_installed(&self) -> bool {
        Command::new("pre-commit")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn get_version(&self) -> Result<String, HookError> {
        let output = Command::new("pre-commit")
            .args(["--version"])
            .output()?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        // Parse "pre-commit 3.6.0"
        let version = version_str
            .split_whitespace()
            .nth(1)
            .unwrap_or("unknown")
            .to_string();

        Ok(version)
    }

    fn upgrade(&self, version: &str) -> Result<(), HookError> {
        let status = Command::new("pip")
            .args(["install", "--upgrade", &format!("pre-commit=={}", version)])
            .status()?;

        if !status.success() {
            return Err(HookError::UpgradeFailed(format!("Failed to upgrade pre-commit to {}", version)));
        }

        Ok(())
    }
}
```

### Husky Handler

```rust
// src/git_hooks/husky.rs
use std::process::Command;

pub struct HuskyHandler {
    config: HuskyConfig,
    project_dir: PathBuf,
}

impl HuskyHandler {
    pub fn install(&self) -> Result<(), HookError> {
        let pm = &self.config.package_manager;

        // Check if husky is in package.json
        if !self.has_husky_dependency()? {
            println!("  Adding husky to devDependencies...");
            let version = self.config.version.as_deref().unwrap_or("latest");
            let status = Command::new(pm)
                .args(["add", "-D", &format!("husky@{}", version)])
                .current_dir(&self.project_dir)
                .status()?;

            if !status.success() {
                return Err(HookError::InstallFailed("Failed to add husky".to_string()));
            }
        }

        // Run husky install (via npm prepare script or directly)
        println!("  Installing husky hooks...");

        // Modern husky (v9+) uses npx husky init or husky
        let status = Command::new("npx")
            .args(["husky"])
            .current_dir(&self.project_dir)
            .status()?;

        if !status.success() {
            // Fallback to older husky install
            let status = Command::new("npx")
                .args(["husky", "install"])
                .current_dir(&self.project_dir)
                .status()?;

            if !status.success() {
                return Err(HookError::InstallFailed("husky install failed".to_string()));
            }
        }

        println!("  ✓ husky hooks installed");
        Ok(())
    }

    fn has_husky_dependency(&self) -> Result<bool, HookError> {
        let package_json = self.project_dir.join("package.json");
        if !package_json.exists() {
            return Ok(false);
        }

        let content = std::fs::read_to_string(&package_json)?;
        Ok(content.contains("\"husky\""))
    }
}
```

### Lefthook Handler

```rust
// src/git_hooks/lefthook.rs
use std::process::Command;

pub struct LefthookHandler {
    config: LefthookConfig,
    project_dir: PathBuf,
}

impl LefthookHandler {
    pub fn install(&self) -> Result<(), HookError> {
        // Check if lefthook is installed
        if !self.is_installed() {
            return Err(HookError::FrameworkNotInstalled("lefthook".to_string()));
        }

        println!("  Installing lefthook hooks...");
        let status = Command::new("lefthook")
            .arg("install")
            .current_dir(&self.project_dir)
            .status()?;

        if !status.success() {
            return Err(HookError::InstallFailed("lefthook install failed".to_string()));
        }

        println!("  ✓ lefthook hooks installed");
        Ok(())
    }

    fn is_installed(&self) -> bool {
        Command::new("lefthook")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
```

## Implementation Steps

1. Create git_hooks module structure
2. Implement HooksConfig parsing
3. Implement framework detection
4. Implement pre-commit handler
5. Implement husky handler
6. Implement lefthook handler
7. Implement native hook support
8. Integrate with setup command
9. Implement `hooks install` command
10. Implement `hooks update` command
11. Implement `hooks list` command
12. Implement `hooks status` command
13. Implement `hooks run` command
14. Write tests for hook installation
15. Update documentation

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Manual hook installation | 100% | <5% |
| Hook-related CI failures | Common | Rare |
| Time to productive setup | +5 minutes for hooks | 0 extra time |
| Hook version consistency | Variable | Uniform |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Framework not installed | Medium | Medium | Clear error, link to install docs |
| Hook conflicts | Low | Medium | Don't overwrite existing hooks |
| Network required | Medium | Low | Cache hook repos, offline mode |
| Framework version mismatch | Low | Low | Pin versions in config |
| Python/Node not available | Medium | Low | Detect and warn early |

## Dependencies

### New Dependencies
- `serde_yaml` - Parse pre-commit config (if not already present)

### Prerequisite Tools
- `pre-commit` for [hooks.pre-commit]
- `node/npm` for [hooks.husky]
- `lefthook` for [hooks.lefthook]

### Existing Dependencies
- `serde` - Configuration parsing

## Effort Estimate

| Task | Effort |
|------|--------|
| Module structure and config | 0.5 days |
| Framework detection | 0.5 days |
| pre-commit handler | 1 day |
| husky handler | 1 day |
| lefthook handler | 0.5 days |
| Native hook support | 0.5 days |
| Setup integration | 0.5 days |
| hooks install command | 0.25 days |
| hooks update command | 0.25 days |
| hooks list command | 0.25 days |
| hooks status command | 0.25 days |
| hooks run command | 0.25 days |
| Testing | 1 day |
| Documentation | 0.5 days |
| **Total** | **7 days** |

## Files to Create/Modify

### New Files
- `src/git_hooks/mod.rs`
- `src/git_hooks/config.rs`
- `src/git_hooks/detection.rs`
- `src/git_hooks/precommit.rs`
- `src/git_hooks/husky.rs`
- `src/git_hooks/lefthook.rs`
- `src/git_hooks/native.rs`
- `src/git_hooks/commands.rs`
- `tests/git_hooks_integration.rs`

### Modified Files
- `src/config.rs` - Add hooks config parsing
- `src/lib.rs` - Export git_hooks module
- `src/main.rs` - Add hooks subcommand
- `src/commands/setup_cmd.rs` - Integrate hook installation
- `Cargo.toml` - Add serde_yaml if not present
- `CLAUDE.md` - Document [hooks.git_hooks] section

---

*PRD-048 v1.0 | Pre-Commit Hook Installation | Priority: Low*
