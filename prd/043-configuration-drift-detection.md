# PRD-043: Configuration Drift Detection

## Overview

Enable Jarvy to detect when a developer's environment has drifted from the expected configuration defined in `jarvy.toml`, identifying missing tools, version mismatches, and configuration changes to help maintain consistent environments.

## Problem Statement

Over time, development environments drift from their intended state:

- Tools get manually upgraded or downgraded
- Dependencies are removed or become unavailable
- Configuration files are manually modified
- System updates break tool installations
- New team members have different base configurations

This drift causes "works on my machine" issues and makes troubleshooting difficult because the actual environment state is unknown.

## Evidence

- "It worked yesterday" debugging sessions
- Different team members have different tool versions
- Manual tool upgrades break project compatibility
- No visibility into environment state vs. expected state
- Setup issues discovered only when something breaks

## Requirements

### Functional Requirements

1. **State capture**: Record expected environment state after setup
2. **Drift detection**: Compare current state to expected state
3. **Version checking**: Detect tool version mismatches
4. **Presence checking**: Detect missing or extra tools
5. **Config checking**: Detect configuration file changes
6. **Reporting**: Generate human-readable drift reports
7. **Remediation**: Offer to fix detected drift

### Non-Functional Requirements

1. **Fast**: Drift check should complete in <5 seconds
2. **Non-destructive**: Detection only, no changes without consent
3. **Offline capable**: Work without network for installed tools
4. **Incremental**: Check only what changed when possible
5. **Machine-readable**: Support JSON output for automation

## Non-Goals

- Automatic drift remediation without user consent
- Detecting changes in user-specific settings (themes, keybindings)
- Monitoring/alerting system (this is on-demand checking)
- Full system inventory (only Jarvy-managed tools)
- Tracking changes over time (history/audit log)

## Feature Specifications

### 1. State File Format

```json
// .jarvy/state.json (generated after successful setup)
{
  "version": "1",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-20T14:22:00Z",
  "config_hash": "sha256:abc123...",
  "tools": {
    "node": {
      "version": "20.10.0",
      "path": "/usr/local/bin/node",
      "install_method": "brew"
    },
    "rust": {
      "version": "1.75.0",
      "path": "/Users/dev/.cargo/bin/rustc",
      "install_method": "rustup"
    },
    "docker": {
      "version": "24.0.7",
      "path": "/usr/local/bin/docker",
      "install_method": "cask"
    }
  },
  "files": {
    ".vscode/settings.json": "sha256:def456...",
    ".env": "sha256:ghi789..."
  }
}
```

### 2. Configuration Syntax

```toml
# jarvy.toml

[drift]
# Enable/disable drift detection
enabled = true

# Check on every jarvy command (default: false)
check_on_run = false

# Files to track for changes
track_files = [
    ".vscode/settings.json",
    ".vscode/extensions.json",
    ".editorconfig",
    "package.json",
    "Cargo.toml",
]

# Ignore specific version changes (major.minor only)
version_policy = "minor"  # major, minor, patch, exact

# Tools to exclude from drift detection
ignore_tools = ["vim", "neovim"]

# Allow version upgrades (only flag downgrades)
allow_upgrades = true
```

### 3. CLI Commands

```bash
# Check for drift
jarvy drift check

# Output:
# Environment Drift Report
# ========================
#
# Tool Version Changes:
#   node    20.10.0 → 21.5.0  (upgraded)
#   rust    1.75.0  → 1.74.0  (DOWNGRADED)
#
# Missing Tools:
#   docker  (was: 24.0.7)
#
# Extra Tools (not in config):
#   deno    1.39.0
#
# Changed Files:
#   .vscode/settings.json  (modified)
#
# Summary: 4 issues detected
# Run 'jarvy drift fix' to remediate

# Check with JSON output
jarvy drift check --json

# Fix detected drift
jarvy drift fix

# Output:
# Fixing environment drift...
#   ✓ Installing docker 24.0.7
#   ✓ Downgrading node 21.5.0 → 20.10.0
#   ⚠ Skipping rust (version manager: use rustup)
#   ⚠ Skipping .vscode/settings.json (manual change)
#
# Fixed 2 of 4 issues
# 2 issues require manual intervention

# Update state file to match current environment
jarvy drift accept

# Output:
# Accepting current environment state...
#   Updated node: 20.10.0 → 21.5.0
#   Removed docker from state
#   Added deno to state
#   Updated .vscode/settings.json hash
#
# State file updated. This is now the baseline.

# Show current state
jarvy drift status

# Compare two environments
jarvy drift compare ./other-project/jarvy.toml
```

### 4. Drift Report Format (JSON)

```json
{
  "timestamp": "2024-01-20T15:00:00Z",
  "status": "drift_detected",
  "summary": {
    "total_issues": 4,
    "version_changes": 2,
    "missing_tools": 1,
    "extra_tools": 1,
    "changed_files": 1
  },
  "version_changes": [
    {
      "tool": "node",
      "expected": "20.10.0",
      "actual": "21.5.0",
      "direction": "upgrade",
      "auto_fixable": true
    },
    {
      "tool": "rust",
      "expected": "1.75.0",
      "actual": "1.74.0",
      "direction": "downgrade",
      "auto_fixable": false,
      "reason": "version_manager"
    }
  ],
  "missing_tools": [
    {
      "tool": "docker",
      "expected_version": "24.0.7",
      "auto_fixable": true
    }
  ],
  "extra_tools": [
    {
      "tool": "deno",
      "version": "1.39.0"
    }
  ],
  "changed_files": [
    {
      "path": ".vscode/settings.json",
      "expected_hash": "sha256:abc...",
      "actual_hash": "sha256:def...",
      "auto_fixable": false
    }
  ]
}
```

## Technical Approach

### Module Structure

```
src/
  drift/
    mod.rs           # Public API
    config.rs        # Drift configuration
    state.rs         # State file management
    detector.rs      # Drift detection logic
    reporter.rs      # Report generation
    fixer.rs         # Remediation logic
    commands.rs      # CLI command handlers
```

### Configuration Types

```rust
// src/drift/config.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DriftConfig {
    /// Enable drift detection
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Check for drift on every jarvy command
    #[serde(default)]
    pub check_on_run: bool,

    /// Files to track for changes
    #[serde(default)]
    pub track_files: Vec<String>,

    /// Version matching policy
    #[serde(default)]
    pub version_policy: VersionPolicy,

    /// Tools to ignore
    #[serde(default)]
    pub ignore_tools: Vec<String>,

    /// Allow upgrades (only flag downgrades)
    #[serde(default)]
    pub allow_upgrades: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VersionPolicy {
    /// Only major version must match (1.x.x)
    Major,
    /// Major and minor must match (1.2.x)
    #[default]
    Minor,
    /// Major, minor, and patch must match (1.2.3)
    Patch,
    /// Exact version required
    Exact,
}
```

### State Management

```rust
// src/drift/state.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvironmentState {
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub config_hash: String,
    pub tools: HashMap<String, ToolState>,
    pub files: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolState {
    pub version: String,
    pub path: PathBuf,
    pub install_method: String,
}

impl EnvironmentState {
    pub fn capture(tools: &[InstalledTool], files: &[TrackedFile]) -> Self {
        let mut state = Self {
            version: "1".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            config_hash: String::new(),
            tools: HashMap::new(),
            files: HashMap::new(),
        };

        for tool in tools {
            state.tools.insert(tool.name.clone(), ToolState {
                version: tool.version.clone(),
                path: tool.path.clone(),
                install_method: tool.install_method.clone(),
            });
        }

        for file in files {
            state.files.insert(file.path.clone(), file.hash.clone());
        }

        state
    }

    pub fn load(project_dir: &Path) -> Result<Option<Self>, DriftError> {
        let state_path = project_dir.join(".jarvy/state.json");
        if !state_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&state_path)?;
        let state: Self = serde_json::from_str(&content)?;
        Ok(Some(state))
    }

    pub fn save(&self, project_dir: &Path) -> Result<(), DriftError> {
        let jarvy_dir = project_dir.join(".jarvy");
        std::fs::create_dir_all(&jarvy_dir)?;

        let state_path = jarvy_dir.join("state.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&state_path, content)?;
        Ok(())
    }
}
```

### Drift Detector

```rust
// src/drift/detector.rs
use crate::tools::common::get_tool_version;

pub struct DriftDetector {
    config: DriftConfig,
    expected_state: EnvironmentState,
}

#[derive(Debug)]
pub struct DriftReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: DriftStatus,
    pub version_changes: Vec<VersionChange>,
    pub missing_tools: Vec<MissingTool>,
    pub extra_tools: Vec<ExtraTool>,
    pub changed_files: Vec<ChangedFile>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DriftStatus {
    NoDrift,
    DriftDetected,
    NoBaseline,
}

#[derive(Debug)]
pub struct VersionChange {
    pub tool: String,
    pub expected: String,
    pub actual: String,
    pub direction: VersionDirection,
    pub auto_fixable: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VersionDirection {
    Upgrade,
    Downgrade,
}

impl DriftDetector {
    pub fn detect(&self) -> Result<DriftReport, DriftError> {
        let mut report = DriftReport {
            timestamp: chrono::Utc::now(),
            status: DriftStatus::NoDrift,
            version_changes: Vec::new(),
            missing_tools: Vec::new(),
            extra_tools: Vec::new(),
            changed_files: Vec::new(),
        };

        // Check each expected tool
        for (name, expected) in &self.expected_state.tools {
            if self.config.ignore_tools.contains(name) {
                continue;
            }

            match get_tool_version(name) {
                Some(actual_version) => {
                    if !self.versions_match(&expected.version, &actual_version) {
                        let direction = if self.is_upgrade(&expected.version, &actual_version) {
                            VersionDirection::Upgrade
                        } else {
                            VersionDirection::Downgrade
                        };

                        // Skip if allow_upgrades and this is an upgrade
                        if self.config.allow_upgrades && direction == VersionDirection::Upgrade {
                            continue;
                        }

                        report.version_changes.push(VersionChange {
                            tool: name.clone(),
                            expected: expected.version.clone(),
                            actual: actual_version,
                            direction,
                            auto_fixable: self.is_auto_fixable(name),
                            reason: None,
                        });
                    }
                }
                None => {
                    report.missing_tools.push(MissingTool {
                        tool: name.clone(),
                        expected_version: expected.version.clone(),
                        auto_fixable: true,
                    });
                }
            }
        }

        // Check tracked files
        for (path, expected_hash) in &self.expected_state.files {
            if let Ok(actual_hash) = self.hash_file(path) {
                if actual_hash != *expected_hash {
                    report.changed_files.push(ChangedFile {
                        path: path.clone(),
                        expected_hash: expected_hash.clone(),
                        actual_hash,
                        auto_fixable: false,
                    });
                }
            }
        }

        // Update status
        if !report.version_changes.is_empty()
            || !report.missing_tools.is_empty()
            || !report.changed_files.is_empty()
        {
            report.status = DriftStatus::DriftDetected;
        }

        Ok(report)
    }

    fn versions_match(&self, expected: &str, actual: &str) -> bool {
        match self.config.version_policy {
            VersionPolicy::Exact => expected == actual,
            VersionPolicy::Patch => {
                let exp = semver::Version::parse(expected).ok();
                let act = semver::Version::parse(actual).ok();
                match (exp, act) {
                    (Some(e), Some(a)) => e.major == a.major && e.minor == a.minor && e.patch == a.patch,
                    _ => expected == actual,
                }
            }
            VersionPolicy::Minor => {
                let exp = semver::Version::parse(expected).ok();
                let act = semver::Version::parse(actual).ok();
                match (exp, act) {
                    (Some(e), Some(a)) => e.major == a.major && e.minor == a.minor,
                    _ => expected == actual,
                }
            }
            VersionPolicy::Major => {
                let exp = semver::Version::parse(expected).ok();
                let act = semver::Version::parse(actual).ok();
                match (exp, act) {
                    (Some(e), Some(a)) => e.major == a.major,
                    _ => expected == actual,
                }
            }
        }
    }
}
```

### Report Generation

```rust
// src/drift/reporter.rs
use colored::Colorize;

pub struct DriftReporter;

impl DriftReporter {
    pub fn print_report(report: &DriftReport) {
        println!("Environment Drift Report");
        println!("========================\n");

        if report.status == DriftStatus::NoDrift {
            println!("{}", "✓ No drift detected. Environment matches expected state.".green());
            return;
        }

        if !report.version_changes.is_empty() {
            println!("Tool Version Changes:");
            for change in &report.version_changes {
                let arrow = match change.direction {
                    VersionDirection::Upgrade => "→".yellow(),
                    VersionDirection::Downgrade => "→".red(),
                };
                let label = match change.direction {
                    VersionDirection::Upgrade => "(upgraded)".yellow(),
                    VersionDirection::Downgrade => "(DOWNGRADED)".red(),
                };
                println!("  {:<12} {} {} {}  {}",
                    change.tool,
                    change.expected,
                    arrow,
                    change.actual,
                    label
                );
            }
            println!();
        }

        if !report.missing_tools.is_empty() {
            println!("Missing Tools:");
            for tool in &report.missing_tools {
                println!("  {}  (was: {})", tool.tool.red(), tool.expected_version);
            }
            println!();
        }

        if !report.extra_tools.is_empty() {
            println!("Extra Tools (not in config):");
            for tool in &report.extra_tools {
                println!("  {}  {}", tool.tool.cyan(), tool.version);
            }
            println!();
        }

        if !report.changed_files.is_empty() {
            println!("Changed Files:");
            for file in &report.changed_files {
                println!("  {}  (modified)", file.path.yellow());
            }
            println!();
        }

        let total = report.version_changes.len()
            + report.missing_tools.len()
            + report.changed_files.len();
        println!("Summary: {} issues detected", total.to_string().red());
        println!("Run '{}' to remediate", "jarvy drift fix".cyan());
    }

    pub fn to_json(report: &DriftReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_default()
    }
}
```

## Implementation Steps

1. Create drift module structure
2. Implement DriftConfig parsing
3. Implement EnvironmentState capture and persistence
4. Implement tool version detection
5. Implement file hash tracking
6. Implement drift detection logic
7. Implement version comparison with policies
8. Implement report generation (text and JSON)
9. Implement `drift check` command
10. Implement `drift fix` command
11. Implement `drift accept` command
12. Implement `drift status` command
13. Integrate state capture with setup command
14. Add check_on_run option
15. Write tests for drift detection
16. Update documentation

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Environment state visibility | None | Full |
| Time to detect environment issues | Hours/days | <5 seconds |
| "Works on my machine" incidents | Common | Rare |
| Manual environment audits | Manual | Automated |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| State file out of sync | Medium | Medium | Update on each setup run |
| Version parsing failures | Low | Low | Fallback to string comparison |
| File hash collisions | Very Low | Low | Use SHA-256 |
| Performance on large projects | Low | Medium | Track only configured files |
| False positives | Medium | Low | Configurable policies, ignore lists |

## Dependencies

### New Dependencies
- `sha2` - For file hashing (already in project for update checksums)
- `semver` - For version comparison

### Existing Dependencies
- `chrono` - Timestamps
- `serde_json` - State file format
- `colored` - Report formatting

## Effort Estimate

| Task | Effort |
|------|--------|
| Module structure and config | 0.5 days |
| State capture/persistence | 1 day |
| Tool version detection | 1 day |
| File hash tracking | 0.5 days |
| Drift detection logic | 1 day |
| Version policies | 0.5 days |
| Report generation | 0.5 days |
| CLI commands | 1 day |
| Drift fix logic | 1 day |
| Setup integration | 0.5 days |
| Testing | 1 day |
| Documentation | 0.5 days |
| **Total** | **9 days** |

## Files to Create/Modify

### New Files
- `src/drift/mod.rs`
- `src/drift/config.rs`
- `src/drift/state.rs`
- `src/drift/detector.rs`
- `src/drift/reporter.rs`
- `src/drift/fixer.rs`
- `src/drift/commands.rs`
- `tests/drift_integration.rs`

### Modified Files
- `src/config.rs` - Add drift config parsing
- `src/lib.rs` - Export drift module
- `src/main.rs` - Add drift subcommand
- `src/commands/setup_cmd.rs` - Capture state after setup
- `Cargo.toml` - Add semver dependency (if not present)
- `CLAUDE.md` - Document [drift] section

---

*PRD-043 v1.0 | Configuration Drift Detection | Priority: Medium*
