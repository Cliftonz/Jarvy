# PRD-039: Language Package Dependencies

## Overview

Enable Jarvy to manage language-specific package dependencies (npm, pip, cargo, gem, go modules) alongside CLI tools, providing complete development environment reproducibility from a single `jarvy.toml` configuration.

## Problem Statement

Jarvy excels at provisioning CLI tools, but the majority of "works on my machine" issues stem from differing language-specific dependencies:

- Node projects have different `node_modules` versions
- Python projects have conflicting virtual environment packages
- Rust projects may have different cargo-installed binaries
- Ruby projects need specific gem versions
- Go projects require particular module versions

Currently, developers must manually run `npm install`, `pip install -r requirements.txt`, etc., after Jarvy setup. This creates a gap in environment reproducibility.

## Evidence

- Teams report tool setup works, but project dependencies still vary
- Multiple package managers (npm, yarn, pnpm) cause version drift
- Virtual environment setup is error-prone and forgotten
- "Did you run npm install?" is a common onboarding question
- Lock file conflicts are frequent in team environments

## Requirements

### Functional Requirements

1. **npm/yarn/pnpm support**: Install Node.js packages from package.json or explicit list
2. **pip/uv support**: Install Python packages, optionally into virtual environments
3. **cargo support**: Install Rust binaries via cargo install
4. **gem/bundler support**: Install Ruby gems
5. **go modules support**: Install Go binaries via go install
6. **Version pinning**: Support exact versions, ranges, and lock files
7. **Virtual environment management**: Create and manage Python venvs
8. **Package manager selection**: Allow choosing between npm/yarn/pnpm, pip/uv

### Non-Functional Requirements

1. **Idempotent**: Running twice produces same result
2. **Parallel installation**: Install packages concurrently where safe
3. **Cross-platform**: Work on macOS, Linux, Windows
4. **Fail gracefully**: Continue setup if optional packages fail
5. **Lock file respect**: Use existing lock files when present
6. **Offline support**: Use cached packages when available

## Non-Goals

- Package publishing/deployment
- Dependency vulnerability scanning (defer to cargo-audit, npm audit)
- Complex monorepo package management
- Container/Docker image building
- Private registry authentication (future enhancement)

## Feature Specifications

### 1. Configuration Syntax

```toml
# jarvy.toml

[provisioner]
node = "20"
python = "3.12"
rust = "latest"
ruby = "3.3"
go = "1.22"

# Node.js packages
[npm]
typescript = "^5.0"
eslint = "latest"
prettier = "latest"
# Use specific package manager (default: auto-detect from lock file)
package_manager = "pnpm"  # npm, yarn, pnpm

# Python packages
[pip]
pytest = ">=7.0"
black = "latest"
mypy = "latest"
# Virtual environment configuration
venv = ".venv"  # Create/use venv at this path
python_version = "3.12"  # Optional, uses [provisioner] python if not set

# Rust cargo binaries
[cargo]
cargo-watch = "latest"
cargo-nextest = "latest"
bacon = "latest"

# Ruby gems
[gem]
bundler = "latest"
rubocop = "latest"
solargraph = "latest"

# Go binaries
[go]
golangci-lint = "latest"
gopls = "latest"
air = "latest"
```

### 2. Alternative: List Syntax

```toml
# Simpler syntax for latest versions
[npm]
packages = ["typescript", "eslint", "prettier"]

[pip]
packages = ["pytest", "black", "mypy"]
venv = ".venv"

[cargo]
packages = ["cargo-watch", "cargo-nextest", "bacon"]
```

### 3. Lock File Integration

```toml
# Use existing project lock files
[npm]
from_lockfile = true  # Use package-lock.json, yarn.lock, or pnpm-lock.yaml
install_dev = true    # Include devDependencies

[pip]
from_lockfile = true  # Use requirements.txt or pyproject.toml
lockfile = "requirements-dev.txt"  # Specify custom lock file
```

### 4. Virtual Environment Management

```toml
[pip]
venv = ".venv"                    # Path to virtual environment
create_venv = true                # Create if doesn't exist (default: true)
activate_hint = true              # Show activation command after setup
system_site_packages = false      # Include system packages (default: false)

# Multiple environments for different purposes
[pip.envs.dev]
packages = ["pytest", "black", "mypy"]
venv = ".venv-dev"

[pip.envs.docs]
packages = ["sphinx", "sphinx-rtd-theme"]
venv = ".venv-docs"
```

### 5. Package Manager Selection

```toml
[npm]
# Auto-detect from lock file, or specify explicitly
package_manager = "pnpm"  # npm, yarn, pnpm

# Jarvy detects:
# - package-lock.json → npm
# - yarn.lock → yarn
# - pnpm-lock.yaml → pnpm
# - No lock file → uses specified or defaults to npm
```

## Technical Approach

### Module Structure

```
src/
  packages/
    mod.rs           # Public API
    config.rs        # Package configuration parsing
    npm.rs           # npm/yarn/pnpm handler
    pip.rs           # pip/uv handler
    cargo.rs         # cargo install handler
    gem.rs           # gem/bundler handler
    go.rs            # go install handler
    venv.rs          # Python virtual environment management
    common.rs        # Shared utilities
```

### Configuration Types

```rust
// src/packages/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PackagesConfig {
    pub npm: Option<NpmConfig>,
    pub pip: Option<PipConfig>,
    pub cargo: Option<CargoConfig>,
    pub gem: Option<GemConfig>,
    pub go: Option<GoConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PackageSpec {
    Version(String),
    Detailed {
        version: String,
        #[serde(default)]
        optional: bool,
        #[serde(default)]
        features: Vec<String>,  // For cargo
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct NpmConfig {
    #[serde(flatten)]
    pub packages: HashMap<String, PackageSpec>,
    #[serde(default)]
    pub package_manager: Option<NpmPackageManager>,
    #[serde(default)]
    pub from_lockfile: bool,
    #[serde(default = "default_true")]
    pub install_dev: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NpmPackageManager {
    #[default]
    Npm,
    Yarn,
    Pnpm,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PipConfig {
    #[serde(flatten)]
    pub packages: HashMap<String, PackageSpec>,
    pub venv: Option<String>,
    #[serde(default = "default_true")]
    pub create_venv: bool,
    #[serde(default)]
    pub from_lockfile: bool,
    pub lockfile: Option<String>,
    #[serde(default = "default_true")]
    pub activate_hint: bool,
    #[serde(default)]
    pub system_site_packages: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CargoConfig {
    #[serde(flatten)]
    pub packages: HashMap<String, PackageSpec>,
    #[serde(default)]
    pub locked: bool,  // Use --locked flag
}

fn default_true() -> bool { true }
```

### npm Handler

```rust
// src/packages/npm.rs
use std::process::Command;
use std::path::Path;

pub struct NpmHandler {
    config: NpmConfig,
    project_dir: PathBuf,
}

impl NpmHandler {
    pub fn install(&self) -> Result<(), PackageError> {
        let pm = self.detect_package_manager();

        if self.config.from_lockfile {
            self.install_from_lockfile(pm)?;
        } else {
            self.install_packages(pm)?;
        }

        Ok(())
    }

    fn detect_package_manager(&self) -> NpmPackageManager {
        if let Some(pm) = self.config.package_manager {
            return pm;
        }

        // Auto-detect from lock files
        if self.project_dir.join("pnpm-lock.yaml").exists() {
            NpmPackageManager::Pnpm
        } else if self.project_dir.join("yarn.lock").exists() {
            NpmPackageManager::Yarn
        } else {
            NpmPackageManager::Npm
        }
    }

    fn install_from_lockfile(&self, pm: NpmPackageManager) -> Result<(), PackageError> {
        let (cmd, args) = match pm {
            NpmPackageManager::Npm => ("npm", vec!["ci"]),
            NpmPackageManager::Yarn => ("yarn", vec!["install", "--frozen-lockfile"]),
            NpmPackageManager::Pnpm => ("pnpm", vec!["install", "--frozen-lockfile"]),
        };

        run_command(cmd, &args, &self.project_dir)
    }

    fn install_packages(&self, pm: NpmPackageManager) -> Result<(), PackageError> {
        let packages: Vec<String> = self.config.packages.iter()
            .map(|(name, spec)| format_package_spec(name, spec))
            .collect();

        if packages.is_empty() {
            return Ok(());
        }

        let cmd = match pm {
            NpmPackageManager::Npm => "npm",
            NpmPackageManager::Yarn => "yarn",
            NpmPackageManager::Pnpm => "pnpm",
        };

        let mut args = vec!["add"];
        args.extend(packages.iter().map(|s| s.as_str()));

        run_command(cmd, &args, &self.project_dir)
    }
}

fn format_package_spec(name: &str, spec: &PackageSpec) -> String {
    match spec {
        PackageSpec::Version(v) if v == "latest" => name.to_string(),
        PackageSpec::Version(v) => format!("{}@{}", name, v),
        PackageSpec::Detailed { version, .. } => format!("{}@{}", name, version),
    }
}
```

### pip Handler with Virtual Environment

```rust
// src/packages/pip.rs
use std::path::PathBuf;

pub struct PipHandler {
    config: PipConfig,
    project_dir: PathBuf,
}

impl PipHandler {
    pub fn install(&self) -> Result<(), PackageError> {
        // Create virtual environment if configured
        let venv_path = if let Some(ref venv) = self.config.venv {
            let path = self.project_dir.join(venv);
            if self.config.create_venv && !path.exists() {
                self.create_venv(&path)?;
            }
            Some(path)
        } else {
            None
        };

        // Determine pip executable
        let pip = if let Some(ref venv) = venv_path {
            venv.join("bin/pip")
        } else {
            PathBuf::from("pip")
        };

        if self.config.from_lockfile {
            self.install_from_lockfile(&pip)?;
        } else {
            self.install_packages(&pip)?;
        }

        // Show activation hint
        if self.config.activate_hint {
            if let Some(ref venv) = venv_path {
                println!("\nVirtual environment created at: {}", venv.display());
                println!("Activate with: source {}/bin/activate", venv.display());
            }
        }

        Ok(())
    }

    fn create_venv(&self, path: &Path) -> Result<(), PackageError> {
        let mut args = vec!["-m", "venv"];

        if self.config.system_site_packages {
            args.push("--system-site-packages");
        }

        args.push(path.to_str().unwrap());

        run_command("python3", &args, &self.project_dir)
    }

    fn install_from_lockfile(&self, pip: &Path) -> Result<(), PackageError> {
        let lockfile = self.config.lockfile.as_deref()
            .unwrap_or("requirements.txt");

        let lockfile_path = self.project_dir.join(lockfile);
        if !lockfile_path.exists() {
            return Err(PackageError::LockfileNotFound(lockfile.to_string()));
        }

        run_command(
            pip.to_str().unwrap(),
            &["install", "-r", lockfile],
            &self.project_dir,
        )
    }

    fn install_packages(&self, pip: &Path) -> Result<(), PackageError> {
        let packages: Vec<String> = self.config.packages.iter()
            .map(|(name, spec)| format_pip_spec(name, spec))
            .collect();

        if packages.is_empty() {
            return Ok(());
        }

        let mut args = vec!["install"];
        args.extend(packages.iter().map(|s| s.as_str()));

        run_command(pip.to_str().unwrap(), &args, &self.project_dir)
    }
}
```

### cargo Handler

```rust
// src/packages/cargo.rs

pub struct CargoHandler {
    config: CargoConfig,
}

impl CargoHandler {
    pub fn install(&self) -> Result<(), PackageError> {
        for (name, spec) in &self.config.packages {
            self.install_crate(name, spec)?;
        }
        Ok(())
    }

    fn install_crate(&self, name: &str, spec: &PackageSpec) -> Result<(), PackageError> {
        let mut args = vec!["install", name];

        match spec {
            PackageSpec::Version(v) if v != "latest" => {
                args.extend(["--version", v]);
            }
            PackageSpec::Detailed { version, features, .. } => {
                if version != "latest" {
                    args.extend(["--version", version]);
                }
                if !features.is_empty() {
                    args.push("--features");
                    args.push(&features.join(","));
                }
            }
            _ => {}
        }

        if self.config.locked {
            args.push("--locked");
        }

        run_command("cargo", &args, &std::env::current_dir()?)
    }
}
```

### Integration with Setup Command

```rust
// src/commands/setup_cmd.rs additions

pub fn run_setup(config: &Config) -> Result<()> {
    // ... existing tool installation ...

    // Install language packages
    if let Some(ref packages) = config.packages {
        println!("\n{}", "Installing language packages...".bold());

        if let Some(ref npm) = packages.npm {
            print_status("npm", "Installing packages...");
            NpmHandler::new(npm.clone(), &config.project_dir)
                .install()
                .map_err(|e| println!("  Warning: npm install failed: {}", e))
                .ok();
        }

        if let Some(ref pip) = packages.pip {
            print_status("pip", "Installing packages...");
            PipHandler::new(pip.clone(), &config.project_dir)
                .install()
                .map_err(|e| println!("  Warning: pip install failed: {}", e))
                .ok();
        }

        if let Some(ref cargo) = packages.cargo {
            print_status("cargo", "Installing binaries...");
            CargoHandler::new(cargo.clone())
                .install()
                .map_err(|e| println!("  Warning: cargo install failed: {}", e))
                .ok();
        }

        // ... gem and go handlers ...
    }

    Ok(())
}
```

## Implementation Steps

1. Create packages module structure
2. Implement PackagesConfig parsing in config.rs
3. Implement NpmHandler with package manager detection
4. Implement PipHandler with virtual environment support
5. Implement CargoHandler for cargo binaries
6. Implement GemHandler for Ruby gems
7. Implement GoHandler for Go binaries
8. Integrate with setup command
9. Add validation for package configurations
10. Implement parallel installation where safe
11. Add progress reporting for package installation
12. Write unit tests for each handler
13. Write integration tests for package installation
14. Update documentation

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Post-setup manual steps | 3-4 commands | 0 commands |
| "Run npm install" questions | Frequent | Eliminated |
| Environment reproducibility | ~70% | >95% |
| First-run success rate | ~80% | >95% |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Package manager not installed | Medium | High | Check for package manager, suggest installation |
| Network failures | Medium | Medium | Retry logic, offline cache support |
| Version conflicts | Medium | Medium | Use lock files, warn on conflicts |
| Platform differences | Low | Medium | Test on all platforms, document limitations |
| Long installation times | High | Low | Parallel installation, progress reporting |

## Dependencies

### New Dependencies
- None required (uses external package managers)

### Prerequisite Tools
- npm/yarn/pnpm for [npm] section
- pip/uv for [pip] section
- cargo for [cargo] section
- gem/bundler for [gem] section
- go for [go] section

## Effort Estimate

| Task | Effort |
|------|--------|
| Module structure and config parsing | 1 day |
| npm handler | 1.5 days |
| pip handler with venv | 2 days |
| cargo handler | 0.5 days |
| gem handler | 1 day |
| go handler | 0.5 days |
| Setup integration | 1 day |
| Validation and error handling | 1 day |
| Testing | 2 days |
| Documentation | 0.5 days |
| **Total** | **11 days** |

## Files to Create/Modify

### New Files
- `src/packages/mod.rs`
- `src/packages/config.rs`
- `src/packages/npm.rs`
- `src/packages/pip.rs`
- `src/packages/cargo.rs`
- `src/packages/gem.rs`
- `src/packages/go.rs`
- `src/packages/venv.rs`
- `src/packages/common.rs`
- `tests/packages_integration.rs`

### Modified Files
- `src/config.rs` - Add packages parsing
- `src/lib.rs` - Export packages module
- `src/commands/setup_cmd.rs` - Integrate package installation
- `src/commands/validate.rs` - Add package validation
- `CLAUDE.md` - Document package sections

---

*PRD-039 v1.0 | Language Package Dependencies | Priority: High*
