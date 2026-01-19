# PRD-035: Self-Updating

## Overview

Enable Jarvy to automatically check for and install updates to itself, keeping users on the latest version with security patches, bug fixes, and new features without manual intervention. Self-updating is enabled by default and controlled via global configuration.

## Problem Statement

CLI tools that don't auto-update create friction and risk:
- Users run outdated versions with known bugs or security issues
- New features and tool definitions aren't available
- Manual update process is easy to forget
- Support burden increases from stale version issues
- Security vulnerabilities persist longer than necessary

Self-updating solves this by proactively keeping Jarvy current while respecting user control.

## Evidence

- Users report bugs already fixed in newer versions
- "How do I update Jarvy?" is a common question
- Other modern CLI tools (Homebrew, rustup, npm) have built-in update mechanisms
- Security-conscious organizations require timely patching
- Package manager installations may lag behind releases

## Requirements

### Functional Requirements

1. **Automatic update checks**: Check for updates on startup (with throttling)
2. **Update notification**: Inform users when updates are available
3. **Installation method detection**: Detect how Jarvy was originally installed
4. **Same-method updates**: Update via the same method used for installation (brew, cargo, binary, etc.)
5. **Version pinning**: Allow users to stay on specific versions
6. **Update channels**: Support stable, beta, and nightly releases
7. **Rollback capability**: Ability to revert to previous version

### Non-Functional Requirements

1. **Enabled by default**: Auto-update on unless explicitly disabled
2. **Non-blocking**: Update checks happen asynchronously, don't slow startup
3. **Secure**: Verify signatures on downloaded binaries
4. **Graceful degradation**: Work offline, fail silently on network errors
5. **Cross-platform**: Work on macOS, Linux, and Windows
6. **Transparent**: Clear logging of update actions

## Non-Goals

- Automatic major version upgrades (require explicit opt-in)
- Update scheduling (cron-like features)
- Update proxying for air-gapped environments
- Plugin/extension updates (future PRD)
- Tool definition updates without binary update

## Feature Specifications

### 1. Global Configuration

Self-updating is configured in the global config file.

```toml
# ~/.jarvy/config.toml

[update]
# Master switch for self-updating (default: true)
enabled = true

# Update channel: "stable", "beta", or "nightly" (default: "stable")
channel = "stable"

# Check frequency in hours (default: 24)
check_interval = 24

# Automatically install updates (default: true for patch/minor, false for major)
auto_install = true

# Only auto-install patch versions (default: false)
patch_only = false

# Pin to specific version (overrides auto-update)
# pinned_version = "1.2.3"

# Notify about updates without installing (default: false)
notify_only = false
```

### 2. Environment Variable Overrides

```bash
# Disable auto-update entirely
JARVY_UPDATE=0

# Set update channel
JARVY_UPDATE_CHANNEL=beta

# Force update check on next run
JARVY_UPDATE_CHECK=1

# Pin to specific version
JARVY_PINNED_VERSION=1.2.3
```

### 3. Update Check Behavior

```
jarvy setup (or any command)
│
├─ Is update check needed? (based on check_interval)
│   └─ No → Continue with command
│   └─ Yes → Spawn background check
│
├─ Background check:
│   ├─ Fetch latest version from releases endpoint
│   ├─ Compare with current version
│   ├─ If newer version available:
│   │   ├─ auto_install=true → Download and stage update
│   │   └─ auto_install=false → Store notification
│   └─ Update last_checked timestamp
│
└─ Continue with command execution
```

### 4. Update Commands

```bash
# Check for updates manually
jarvy update check

# Output:
# Current version: 1.2.0
# Latest version:  1.3.0
#
# Update available! Run 'jarvy update' to install.
#
# What's new in 1.3.0:
#   - Added 15 new tool definitions
#   - Fixed: nvm installation on M1 Macs
#   - Improved: parallel installation speed

# Install available update
jarvy update

# Output:
# Downloading jarvy v1.3.0...
# Verifying signature...
# Installing update...
#
# Successfully updated from 1.2.0 to 1.3.0
#
# Run 'jarvy --version' to confirm.

# Update to specific version
jarvy update --version 1.2.5

# Update to latest in channel
jarvy update --channel beta

# Rollback to previous version
jarvy update --rollback

# Output:
# Rolling back from 1.3.0 to 1.2.0...
# Restored previous version.

# Show update history
jarvy update history

# Output:
# Update History
# ==============
#
# 2024-01-15  1.2.0 → 1.3.0  (auto)
# 2024-01-10  1.1.0 → 1.2.0  (manual)
# 2024-01-05  1.0.0 → 1.1.0  (auto)

# Configure update settings
jarvy update config

# Output:
# Update Configuration
# ====================
#
# Enabled:        true
# Channel:        stable
# Check interval: 24 hours
# Auto-install:   true (patch/minor)
# Last checked:   2 hours ago
# Current:        1.3.0
# Latest:         1.3.0 (up to date)

# Disable updates
jarvy update disable

# Enable updates
jarvy update enable
```

### 5. Update Notification

When an update is available but not auto-installed:

```bash
jarvy setup

# Output:
# [update] Jarvy 1.3.0 is available (current: 1.2.0)
# [update] Run 'jarvy update' to install, or 'jarvy update --info' for details
#
# Setting up development environment...
# ... normal output continues ...
```

Notification behavior:
- Shows once per session
- Can be suppressed with `--quiet` flag
- Respects `notify_only` config
- Does not interrupt command execution

### 6. Secure Update Process

```
1. Fetch release metadata from GitHub Releases API
2. Verify release is signed by Jarvy maintainers
3. Download binary for current platform/architecture
4. Verify SHA256 checksum matches manifest
5. Verify binary signature (if code signing available)
6. Stage new binary in temporary location
7. Replace current binary atomically
8. Preserve previous binary for rollback
```

### 7. CI/CD Behavior

Auto-update is disabled in CI environments:

```bash
# CI environments detected:
# - CI=true
# - GITHUB_ACTIONS=true
# - GITLAB_CI=true
# - JENKINS_URL set
# - CIRCLECI=true

# In CI, updates require explicit opt-in:
JARVY_UPDATE=1 jarvy update
```

### 8. Update Channels

```toml
# ~/.jarvy/config.toml

[update]
channel = "stable"  # Options: stable, beta, nightly
```

| Channel | Description | Update Frequency |
|---------|-------------|------------------|
| stable | Production releases | Every few weeks |
| beta | Pre-release testing | Weekly |
| nightly | Latest development | Daily |

```bash
# Switch channels
jarvy update --channel beta

# Check channel-specific version
jarvy update check --channel nightly

# Output:
# Current version: 1.3.0 (stable)
# Latest stable:   1.3.0
# Latest beta:     1.4.0-beta.2
# Latest nightly:  1.4.0-nightly.20240115
```

### 9. Installation Method Detection

Jarvy detects how it was installed and uses the same method for updates. This respects package manager ownership and avoids conflicts.

**Supported Installation Methods:**

| Method | Detection | Update Command |
|--------|-----------|----------------|
| Homebrew | Binary in `/opt/homebrew/` or `/usr/local/Cellar/` | `brew upgrade jarvy` |
| Cargo | Binary in `~/.cargo/bin/` | `cargo install jarvy` |
| apt/deb | `dpkg -S` finds package | `apt update && apt upgrade jarvy` |
| dnf/rpm | `rpm -qf` finds package | `dnf upgrade jarvy` |
| winget | Registry entry exists | `winget upgrade jarvy` |
| Chocolatey | `choco list` finds package | `choco upgrade jarvy` |
| Scoop | Binary in scoop directory | `scoop update jarvy` |
| Direct binary | None of the above | Direct GitHub release download |

**Detection Logic:**

```
1. Check binary path against known package manager locations
2. Query package managers to verify ownership
3. Store detected method in ~/.jarvy/install-method
4. Fall back to direct binary update if unknown
```

**Example Output:**

```bash
jarvy update config

# Output:
# Update Configuration
# ====================
#
# Enabled:          true
# Install method:   homebrew    ← Detected installation method
# Channel:          stable
# Check interval:   24 hours
# Auto-install:     true (patch/minor)
# Last checked:     2 hours ago
# Current:          1.3.0
# Latest:           1.4.0

jarvy update

# Output (Homebrew):
# Detected installation method: homebrew
# Running: brew upgrade jarvy
# ==> Upgrading 1 outdated package:
# jarvy 1.3.0 -> 1.4.0
# ==> Upgrading jarvy
# 🍺  jarvy was successfully upgraded!
#
# Successfully updated from 1.3.0 to 1.4.0

jarvy update

# Output (Cargo):
# Detected installation method: cargo
# Running: cargo install jarvy
#     Updating crates.io index
#   Installing jarvy v1.4.0
#    Compiling jarvy v1.4.0
#     Finished release [optimized] target(s)
#   Installed package `jarvy v1.4.0`
#
# Successfully updated from 1.3.0 to 1.4.0

jarvy update

# Output (Direct binary):
# Detected installation method: binary
# Downloading jarvy v1.4.0 for darwin-arm64...
# Verifying checksum...
# Replacing binary...
#
# Successfully updated from 1.3.0 to 1.4.0
```

**Override Installation Method:**

Users can override the detected method if needed:

```toml
# ~/.jarvy/config.toml

[update]
# Force specific update method (auto-detected if not set)
# Options: homebrew, cargo, apt, dnf, winget, chocolatey, scoop, binary
install_method = "cargo"
```

```bash
# One-time override
jarvy update --method binary
```

**Package Manager Considerations:**

- **Homebrew/apt/dnf**: May have older versions than GitHub releases
  - Show warning if package manager version lags behind
  - Offer `--method binary` as alternative for latest version
- **Cargo**: Always gets latest from crates.io (may lag GitHub by hours)
- **Direct binary**: Always gets latest from GitHub releases

## Acceptance Criteria

### Configuration
- [ ] `[update]` section in `~/.jarvy/config.toml`
- [ ] `enabled = true` by default
- [ ] Environment variables override config file
- [ ] `JARVY_UPDATE=0` disables updates
- [ ] `channel` setting with stable/beta/nightly

### Update Check
- [ ] Background check doesn't block command execution
- [ ] Respects `check_interval` setting
- [ ] Stores `last_checked` timestamp
- [ ] Works offline (fails gracefully)
- [ ] Throttles checks (no more than once per interval)

### Installation Method Detection
- [ ] Detects Homebrew installation (macOS)
- [ ] Detects Cargo installation
- [ ] Detects apt/deb installation (Linux)
- [ ] Detects dnf/rpm installation (Linux)
- [ ] Detects winget installation (Windows)
- [ ] Detects Chocolatey installation (Windows)
- [ ] Detects Scoop installation (Windows)
- [ ] Falls back to direct binary for unknown methods
- [ ] Caches detected method in `~/.jarvy/install-method`
- [ ] `install_method` config option overrides detection
- [ ] `--method` flag for one-time override

### Update Installation
- [ ] Updates via detected installation method
- [ ] `brew upgrade jarvy` for Homebrew installs
- [ ] `cargo install jarvy` for Cargo installs
- [ ] Package manager commands for apt/dnf/winget/choco/scoop
- [ ] Direct binary download for unknown methods
- [ ] `jarvy update --version X.Y.Z` installs specific version
- [ ] Verifies checksum before installation (binary method)
- [ ] Preserves previous version for rollback
- [ ] `jarvy update --rollback` restores previous

### Update Commands
- [ ] `jarvy update check` shows available updates
- [ ] `jarvy update history` shows update log
- [ ] `jarvy update config` shows settings
- [ ] `jarvy update enable/disable` toggles feature

### CI/CD
- [ ] Auto-disabled when CI=true
- [ ] Can be explicitly enabled with JARVY_UPDATE=1
- [ ] No update prompts in non-interactive mode

### Notifications
- [ ] Shows update available message (once per session)
- [ ] `--quiet` suppresses notifications
- [ ] `notify_only` mode shows but doesn't install

## Technical Approach

### Module Structure

```
src/
  update/
    mod.rs           # Public API
    config.rs        # Update configuration
    checker.rs       # Version checking logic
    installer.rs     # Download and install
    rollback.rs      # Version rollback
    commands.rs      # CLI commands
    release.rs       # GitHub releases API
    signature.rs     # Binary verification
    method.rs        # Installation method detection
```

### Update Configuration Types

```rust
// src/update/config.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct UpdateConfig {
    /// Master switch (default: true)
    pub enabled: bool,
    /// Release channel (default: "stable")
    pub channel: Channel,
    /// Check interval in hours (default: 24)
    pub check_interval: u32,
    /// Auto-install updates (default: true)
    pub auto_install: bool,
    /// Only auto-install patches (default: false)
    pub patch_only: bool,
    /// Pin to specific version
    pub pinned_version: Option<String>,
    /// Notify only, don't auto-install (default: false)
    pub notify_only: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,  // Enabled by default
            channel: Channel::Stable,
            check_interval: 24,
            auto_install: true,
            patch_only: false,
            pinned_version: None,
            notify_only: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Beta,
    Nightly,
}

impl UpdateConfig {
    /// Load from environment, overriding config file values
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(v) = std::env::var("JARVY_UPDATE") {
            config.enabled = matches!(v.as_str(), "1" | "true" | "yes");
        }

        if let Ok(v) = std::env::var("JARVY_UPDATE_CHANNEL") {
            config.channel = match v.to_lowercase().as_str() {
                "beta" => Channel::Beta,
                "nightly" => Channel::Nightly,
                _ => Channel::Stable,
            };
        }

        if let Ok(v) = std::env::var("JARVY_PINNED_VERSION") {
            config.pinned_version = Some(v);
        }

        // Disable in CI unless explicitly enabled
        if is_ci_environment() && std::env::var("JARVY_UPDATE").is_err() {
            config.enabled = false;
        }

        config
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled && self.pinned_version.is_none()
    }
}

fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("JENKINS_URL").is_ok()
        || std::env::var("CIRCLECI").is_ok()
}
```

### Installation Method Detection

```rust
// src/update/method.rs
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallMethod {
    Homebrew,
    Cargo,
    Apt,
    Dnf,
    Winget,
    Chocolatey,
    Scoop,
    Binary,  // Direct download, fallback
}

impl InstallMethod {
    /// Detect how Jarvy was installed based on binary location and package manager queries
    pub fn detect() -> Self {
        // Check cached value first
        if let Some(cached) = Self::load_cached() {
            return cached;
        }

        let method = Self::detect_from_path()
            .or_else(Self::detect_from_package_managers)
            .unwrap_or(InstallMethod::Binary);

        // Cache for future runs
        method.save_cache();
        method
    }

    fn detect_from_path() -> Option<Self> {
        let exe_path = std::env::current_exe().ok()?;
        let path_str = exe_path.to_string_lossy();

        // macOS Homebrew paths
        if path_str.contains("/opt/homebrew/") || path_str.contains("/usr/local/Cellar/") {
            return Some(InstallMethod::Homebrew);
        }

        // Cargo installation
        if path_str.contains(".cargo/bin") {
            return Some(InstallMethod::Cargo);
        }

        // Windows Scoop
        if path_str.contains("scoop") {
            return Some(InstallMethod::Scoop);
        }

        // Windows Chocolatey
        if path_str.contains("chocolatey") {
            return Some(InstallMethod::Chocolatey);
        }

        None
    }

    fn detect_from_package_managers() -> Option<Self> {
        let exe_path = std::env::current_exe().ok()?;

        // Check dpkg (Debian/Ubuntu)
        if Command::new("dpkg").arg("-S").arg(&exe_path)
            .output().map(|o| o.status.success()).unwrap_or(false)
        {
            return Some(InstallMethod::Apt);
        }

        // Check rpm (RHEL/Fedora)
        if Command::new("rpm").arg("-qf").arg(&exe_path)
            .output().map(|o| o.status.success()).unwrap_or(false)
        {
            return Some(InstallMethod::Dnf);
        }

        // Check winget (Windows)
        #[cfg(windows)]
        if Command::new("winget").args(["list", "--id", "jarvy"])
            .output().map(|o| o.status.success()).unwrap_or(false)
        {
            return Some(InstallMethod::Winget);
        }

        None
    }

    /// Execute update using the appropriate method
    pub fn update(&self, version: Option<&str>) -> Result<(), Error> {
        match self {
            InstallMethod::Homebrew => {
                println!("Detected installation method: homebrew");
                run_command("brew", &["upgrade", "jarvy"])
            }
            InstallMethod::Cargo => {
                println!("Detected installation method: cargo");
                let mut args = vec!["install", "jarvy"];
                if let Some(v) = version {
                    args.extend(["--version", v]);
                }
                run_command("cargo", &args)
            }
            InstallMethod::Apt => {
                println!("Detected installation method: apt");
                run_command("sudo", &["apt", "update"])?;
                run_command("sudo", &["apt", "install", "--only-upgrade", "jarvy"])
            }
            InstallMethod::Dnf => {
                println!("Detected installation method: dnf");
                run_command("sudo", &["dnf", "upgrade", "jarvy"])
            }
            InstallMethod::Winget => {
                println!("Detected installation method: winget");
                run_command("winget", &["upgrade", "jarvy"])
            }
            InstallMethod::Chocolatey => {
                println!("Detected installation method: chocolatey");
                run_command("choco", &["upgrade", "jarvy", "-y"])
            }
            InstallMethod::Scoop => {
                println!("Detected installation method: scoop");
                run_command("scoop", &["update", "jarvy"])
            }
            InstallMethod::Binary => {
                println!("Detected installation method: binary");
                // Use direct binary download and replacement
                self.update_binary(version)
            }
        }
    }

    fn update_binary(&self, version: Option<&str>) -> Result<(), Error> {
        // Download from GitHub releases, verify, and replace
        // (Implementation in installer.rs)
        todo!()
    }

    fn load_cached() -> Option<Self> {
        let path = dirs::home_dir()?.join(".jarvy/install-method");
        let contents = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    fn save_cache(&self) {
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".jarvy/install-method");
            let _ = std::fs::write(path, serde_json::to_string(self).unwrap_or_default());
        }
    }
}
```

### Version Checker

```rust
// src/update/checker.rs
use semver::Version;
use std::time::{Duration, SystemTime};

pub struct UpdateChecker {
    config: UpdateConfig,
    state: UpdateState,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UpdateState {
    pub last_checked: Option<SystemTime>,
    pub available_version: Option<String>,
    pub previous_version: Option<String>,
}

impl UpdateChecker {
    pub fn should_check(&self) -> bool {
        if !self.config.is_enabled() {
            return false;
        }

        match self.state.last_checked {
            None => true,
            Some(last) => {
                let interval = Duration::from_secs(
                    self.config.check_interval as u64 * 3600
                );
                SystemTime::now()
                    .duration_since(last)
                    .map(|d| d >= interval)
                    .unwrap_or(true)
            }
        }
    }

    pub async fn check_for_update(&mut self) -> Result<Option<Release>, Error> {
        let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
        let releases = fetch_releases(self.config.channel).await?;

        let latest = releases.into_iter()
            .filter(|r| !r.prerelease || self.config.channel != Channel::Stable)
            .max_by(|a, b| a.version.cmp(&b.version));

        if let Some(release) = latest {
            if release.version > current {
                if self.config.patch_only {
                    // Only update if it's a patch version
                    if release.version.major == current.major
                        && release.version.minor == current.minor
                    {
                        return Ok(Some(release));
                    }
                } else {
                    return Ok(Some(release));
                }
            }
        }

        self.state.last_checked = Some(SystemTime::now());
        self.save_state()?;

        Ok(None)
    }
}
```

### Binary Installer

```rust
// src/update/installer.rs
use std::fs;
use std::path::PathBuf;

pub struct Installer {
    backup_dir: PathBuf,
}

impl Installer {
    pub async fn install(&self, release: &Release) -> Result<(), Error> {
        let current_exe = std::env::current_exe()?;
        let platform = get_platform();

        // Find matching asset
        let asset = release.assets.iter()
            .find(|a| a.name.contains(&platform))
            .ok_or(Error::NoPlatformBinary)?;

        // Download to temp location
        let temp_path = self.download_asset(asset).await?;

        // Verify checksum
        self.verify_checksum(&temp_path, &asset.checksum)?;

        // Backup current binary
        let backup_path = self.backup_current(&current_exe)?;

        // Atomic replace
        self.atomic_replace(&temp_path, &current_exe)?;

        // Save rollback info
        self.save_rollback_info(&backup_path)?;

        Ok(())
    }

    pub fn rollback(&self) -> Result<(), Error> {
        let current_exe = std::env::current_exe()?;
        let backup_path = self.get_latest_backup()?;

        self.atomic_replace(&backup_path, &current_exe)?;

        Ok(())
    }

    fn atomic_replace(&self, src: &Path, dst: &Path) -> Result<(), Error> {
        #[cfg(unix)]
        {
            // On Unix, rename is atomic
            fs::rename(src, dst)?;
        }

        #[cfg(windows)]
        {
            // On Windows, use self-update crate or similar
            self_update::Move::from_source(src)
                .replace_using_temp(dst)?;
        }

        Ok(())
    }
}
```

### CLI Commands

```rust
// src/update/commands.rs

#[derive(Parser)]
pub enum UpdateCommand {
    /// Check for available updates
    Check {
        /// Check specific channel
        #[arg(long)]
        channel: Option<Channel>,
    },

    /// Install available update
    #[command(name = "")]
    Install {
        /// Install specific version
        #[arg(long)]
        version: Option<String>,

        /// Use specific channel
        #[arg(long)]
        channel: Option<Channel>,

        /// Rollback to previous version
        #[arg(long)]
        rollback: bool,
    },

    /// Show update history
    History,

    /// Show update configuration
    Config,

    /// Enable auto-updates
    Enable,

    /// Disable auto-updates
    Disable,
}
```

### Integration with Main

```rust
// src/main.rs additions

async fn main() -> Result<()> {
    // ... existing setup ...

    // Background update check (non-blocking)
    let update_handle = if update_config.is_enabled() {
        Some(tokio::spawn(async move {
            check_for_updates_background().await
        }))
    } else {
        None
    };

    // Execute main command
    let result = run_command(args).await;

    // Show update notification if available (after command)
    if let Some(handle) = update_handle {
        if let Ok(Some(version)) = handle.await {
            if !args.quiet {
                eprintln!("\n[update] Jarvy {} is available (current: {})",
                    version, env!("CARGO_PKG_VERSION"));
                eprintln!("[update] Run 'jarvy update' to install");
            }
        }
    }

    result
}
```

## Implementation Steps

1. Create update module structure
2. Implement UpdateConfig with defaults (enabled=true)
3. Add `[update]` section to global config parsing
4. Implement InstallMethod enum and detection logic
5. Implement package manager detection (brew, cargo, apt, dnf, winget, choco, scoop)
6. Implement GitHub releases API client
7. Implement version comparison logic
8. Implement background update checker
9. Implement package manager update execution
10. Implement binary download and verification (fallback method)
11. Implement atomic binary replacement
12. Implement rollback mechanism
13. Add `jarvy update` command with subcommands
14. Add `--method` flag for override
15. Add update notification to main command flow
16. Add CI environment detection
17. Write unit tests for method detection and version comparison
18. Write integration tests for update flow
19. Update documentation

## Dependencies

### New Dependencies

```toml
[dependencies]
self_update = "0.39"  # Cross-platform binary replacement
reqwest = { version = "0.11", features = ["json"] }  # HTTP client (may exist)
sha2 = "0.10"  # Checksum verification
```

### Existing Dependencies Used
- `semver` - Version parsing and comparison
- `serde` - Config serialization
- `tokio` - Async runtime

## Effort Estimate

| Task | Effort |
|------|--------|
| Module structure | 0.5 days |
| Update configuration | 1 day |
| Installation method detection | 1.5 days |
| Package manager update execution | 1 day |
| GitHub releases client | 1 day |
| Version checking | 0.5 days |
| Binary installation (fallback) | 1 day |
| Rollback mechanism | 1 day |
| CLI commands | 1 day |
| Main integration | 0.5 days |
| CI detection | 0.5 days |
| Testing | 2 days |
| Documentation | 0.5 days |
| **Total** | **12 days** |

## Files to Create/Modify

### New Files
- `src/update/mod.rs`
- `src/update/config.rs`
- `src/update/checker.rs`
- `src/update/installer.rs`
- `src/update/rollback.rs`
- `src/update/commands.rs`
- `src/update/release.rs`
- `src/update/method.rs`
- `tests/update_integration.rs`

### Modified Files
- `src/main.rs` - Add update command, background check
- `src/lib.rs` - Export update module
- `Cargo.toml` - Add dependencies
- `CLAUDE.md` - Document update features
- `docs/ConfigurationFile.md` - Document [update] section

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Users on latest version | Unknown | >80% within 1 week |
| Manual update questions | Frequent | Rare |
| Security patch deployment | Slow | <48 hours |
| Update-related issues | None tracked | <5% of support |

## Risks

1. **Binary replacement failures**: System permissions, antivirus
   - Mitigation: Clear error messages, manual fallback instructions

2. **Network dependency**: Updates fail offline
   - Mitigation: Graceful failure, cached state, work offline

3. **Breaking updates**: Bad release breaks user workflows
   - Mitigation: Rollback capability, staged rollouts, beta channel

4. **User trust**: Unexpected binary changes concern users
   - Mitigation: Transparent logging, opt-out mechanism, signature verification

5. **Platform differences**: Windows binary replacement is complex
   - Mitigation: Use `self_update` crate, extensive testing

6. **Package manager version lag**: brew/apt may have older versions than GitHub
   - Mitigation: Show warning when package manager version lags, offer `--method binary` for latest

7. **Installation method misdetection**: Edge cases in detection logic
   - Mitigation: Allow manual override via config or `--method` flag, cache detected method

## Security Considerations

1. **Signature verification**: Verify releases are signed by maintainers
2. **HTTPS only**: All downloads over TLS
3. **Checksum validation**: Verify SHA256 before installation
4. **GitHub API**: Use official releases API, not third-party mirrors
5. **Atomic updates**: Prevent partial/corrupted installations
6. **Rollback**: Allow recovery from bad updates

---

*PRD-035 v1.0 | Self-Updating | Priority: High*
