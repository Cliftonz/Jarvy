# PRD-040: IDE/Editor Extension Management

## Overview

Enable Jarvy to install and configure IDE extensions (VS Code, JetBrains IDEs) as part of environment provisioning, ensuring consistent editor configurations across team members.

## Problem Statement

Development environments include more than CLI tools - editor/IDE configuration is equally important:

- Teams use different VS Code extensions, causing inconsistent linting/formatting
- New developers spend hours installing recommended extensions
- JetBrains plugin versions vary, causing project file conflicts
- Editor settings drift over time between team members
- "Works on my machine" often means "with my editor configuration"

Currently, editor setup is manual and poorly documented, leading to fragmented developer experiences.

## Evidence

- `.vscode/extensions.json` is often incomplete or outdated
- New hire onboarding includes "install these extensions" lists
- Code reviews catch issues that linters should have flagged
- Teams maintain separate setup documentation for IDE configuration
- IDE plugin conflicts cause project file merge issues

## Requirements

### Functional Requirements

1. **VS Code extensions**: Install extensions from marketplace by ID
2. **VS Code settings**: Optionally sync/configure workspace settings
3. **JetBrains plugins**: Install plugins for IntelliJ, WebStorm, RustRover, etc.
4. **Extension version pinning**: Support specific versions
5. **Workspace vs user scope**: Distinguish workspace-level from user-level config
6. **Extension recommendations**: Generate `.vscode/extensions.json`

### Non-Functional Requirements

1. **Non-destructive**: Don't override existing user preferences
2. **Optional**: Extension installation is opt-in
3. **Cross-platform**: Work on macOS, Linux, Windows
4. **Fail gracefully**: Continue if IDE not installed
5. **Idempotent**: Skip already-installed extensions

## Non-Goals

- Full IDE settings synchronization (defer to built-in sync)
- Theme/color scheme management
- Keybinding configuration
- Neovim/Emacs plugin management (future enhancement)
- IDE installation (handled by [provisioner] section)

## Feature Specifications

### 1. Configuration Syntax

```toml
# jarvy.toml

[provisioner]
vscode = "latest"  # Install VS Code first

# VS Code Extensions
[vscode]
extensions = [
    "rust-analyzer.rust-analyzer",
    "ms-python.python",
    "dbaeumer.vscode-eslint",
    "esbenp.prettier-vscode",
    "github.copilot",
    "eamodio.gitlens",
]

# With version pinning
[vscode.extensions]
"rust-analyzer.rust-analyzer" = "latest"
"ms-python.python" = "2024.0.0"
"dbaeumer.vscode-eslint" = "^2.4.0"

# Generate recommendations file
[vscode]
generate_recommendations = true  # Creates .vscode/extensions.json

# Workspace settings (optional)
[vscode.settings]
"editor.formatOnSave" = true
"editor.defaultFormatter" = "esbenp.prettier-vscode"
"[rust]" = { "editor.defaultFormatter" = "rust-analyzer.rust-analyzer" }
```

### 2. JetBrains IDE Configuration

```toml
# JetBrains plugins (applies to all installed JetBrains IDEs)
[jetbrains]
plugins = [
    "org.rust.lang",           # Rust plugin
    "com.intellij.kubernetes", # Kubernetes
    "Docker",                  # Docker
]

# IDE-specific plugins
[jetbrains.rustrover]
plugins = ["org.rust.lang"]

[jetbrains.intellij]
plugins = [
    "org.jetbrains.plugins.go",
    "com.intellij.kubernetes",
]

[jetbrains.webstorm]
plugins = [
    "com.intellij.plugins.tailwindcss",
]
```

### 3. Combined Example

```toml
[provisioner]
vscode = "latest"

[vscode]
extensions = [
    # Language support
    "rust-analyzer.rust-analyzer",
    "ms-python.python",
    "golang.go",

    # Formatting/linting
    "dbaeumer.vscode-eslint",
    "esbenp.prettier-vscode",

    # Git
    "eamodio.gitlens",
    "github.vscode-pull-request-github",

    # AI assistants
    "github.copilot",
    "github.copilot-chat",

    # Utilities
    "ms-azuretools.vscode-docker",
    "redhat.vscode-yaml",
]
generate_recommendations = true

# Optional: workspace settings
[vscode.settings]
"editor.formatOnSave" = true
"editor.tabSize" = 4
"files.trimTrailingWhitespace" = true
```

## Technical Approach

### Module Structure

```
src/
  ide/
    mod.rs           # Public API
    config.rs        # IDE configuration parsing
    vscode.rs        # VS Code extension handler
    jetbrains.rs     # JetBrains plugin handler
    common.rs        # Shared utilities
```

### Configuration Types

```rust
// src/ide/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct IdeConfig {
    pub vscode: Option<VsCodeConfig>,
    pub jetbrains: Option<JetBrainsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct VsCodeConfig {
    /// List of extension IDs to install
    #[serde(default)]
    pub extensions: Vec<String>,

    /// Extensions with version requirements
    #[serde(default, flatten)]
    pub versioned_extensions: HashMap<String, String>,

    /// Generate .vscode/extensions.json
    #[serde(default)]
    pub generate_recommendations: bool,

    /// Workspace settings to apply
    #[serde(default)]
    pub settings: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct JetBrainsConfig {
    /// Plugins for all JetBrains IDEs
    #[serde(default)]
    pub plugins: Vec<String>,

    /// IDE-specific plugins
    pub intellij: Option<IdePlugins>,
    pub webstorm: Option<IdePlugins>,
    pub rustrover: Option<IdePlugins>,
    pub goland: Option<IdePlugins>,
    pub pycharm: Option<IdePlugins>,
    pub rider: Option<IdePlugins>,
    pub clion: Option<IdePlugins>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct IdePlugins {
    pub plugins: Vec<String>,
}
```

### VS Code Handler

```rust
// src/ide/vscode.rs
use std::process::Command;
use std::path::PathBuf;

pub struct VsCodeHandler {
    config: VsCodeConfig,
    project_dir: PathBuf,
}

impl VsCodeHandler {
    pub fn install(&self) -> Result<(), IdeError> {
        // Check if VS Code is installed
        let code_cmd = self.find_vscode_command()?;

        // Get currently installed extensions
        let installed = self.get_installed_extensions(&code_cmd)?;

        // Install missing extensions
        for ext in self.all_extensions() {
            if !installed.contains(&ext.to_lowercase()) {
                self.install_extension(&code_cmd, ext)?;
            } else {
                println!("  Extension already installed: {}", ext);
            }
        }

        // Generate recommendations file if configured
        if self.config.generate_recommendations {
            self.generate_recommendations_file()?;
        }

        // Apply workspace settings if configured
        if !self.config.settings.is_empty() {
            self.apply_workspace_settings()?;
        }

        Ok(())
    }

    fn find_vscode_command(&self) -> Result<String, IdeError> {
        // Try various VS Code command names
        for cmd in &["code", "code-insiders", "codium"] {
            if Command::new(cmd).arg("--version").output().is_ok() {
                return Ok(cmd.to_string());
            }
        }
        Err(IdeError::VsCodeNotFound)
    }

    fn get_installed_extensions(&self, code_cmd: &str) -> Result<Vec<String>, IdeError> {
        let output = Command::new(code_cmd)
            .args(["--list-extensions"])
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_lowercase())
            .collect())
    }

    fn install_extension(&self, code_cmd: &str, extension: &str) -> Result<(), IdeError> {
        println!("  Installing: {}", extension);

        let status = Command::new(code_cmd)
            .args(["--install-extension", extension, "--force"])
            .status()?;

        if !status.success() {
            eprintln!("  Warning: Failed to install {}", extension);
        }

        Ok(())
    }

    fn all_extensions(&self) -> impl Iterator<Item = &str> {
        self.config.extensions.iter()
            .map(|s| s.as_str())
            .chain(self.config.versioned_extensions.keys().map(|s| s.as_str()))
    }

    fn generate_recommendations_file(&self) -> Result<(), IdeError> {
        let extensions: Vec<&str> = self.all_extensions().collect();

        let content = serde_json::json!({
            "recommendations": extensions
        });

        let vscode_dir = self.project_dir.join(".vscode");
        std::fs::create_dir_all(&vscode_dir)?;

        let path = vscode_dir.join("extensions.json");
        std::fs::write(&path, serde_json::to_string_pretty(&content)?)?;

        println!("  Generated: .vscode/extensions.json");
        Ok(())
    }

    fn apply_workspace_settings(&self) -> Result<(), IdeError> {
        let vscode_dir = self.project_dir.join(".vscode");
        std::fs::create_dir_all(&vscode_dir)?;

        let settings_path = vscode_dir.join("settings.json");

        // Merge with existing settings if present
        let mut settings: serde_json::Value = if settings_path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&settings_path)?)?
        } else {
            serde_json::json!({})
        };

        // Apply configured settings
        if let serde_json::Value::Object(ref mut map) = settings {
            for (key, value) in &self.config.settings {
                let json_value = toml_to_json(value);
                map.insert(key.clone(), json_value);
            }
        }

        std::fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
        println!("  Updated: .vscode/settings.json");

        Ok(())
    }
}
```

### JetBrains Handler

```rust
// src/ide/jetbrains.rs
use std::process::Command;
use std::path::PathBuf;

pub struct JetBrainsHandler {
    config: JetBrainsConfig,
}

impl JetBrainsHandler {
    pub fn install(&self) -> Result<(), IdeError> {
        // Find installed JetBrains IDEs
        let ides = self.find_installed_ides();

        if ides.is_empty() {
            println!("  No JetBrains IDEs detected, skipping plugin installation");
            return Ok(());
        }

        // Install plugins for each detected IDE
        for ide in &ides {
            let plugins = self.plugins_for_ide(ide);
            if !plugins.is_empty() {
                self.install_plugins_for_ide(ide, &plugins)?;
            }
        }

        Ok(())
    }

    fn find_installed_ides(&self) -> Vec<JetBrainsIde> {
        let mut ides = Vec::new();

        // Check common installation locations
        #[cfg(target_os = "macos")]
        {
            let apps_dir = PathBuf::from("/Applications");
            for (app_name, ide) in [
                ("IntelliJ IDEA", JetBrainsIde::IntelliJ),
                ("WebStorm", JetBrainsIde::WebStorm),
                ("RustRover", JetBrainsIde::RustRover),
                ("GoLand", JetBrainsIde::GoLand),
                ("PyCharm", JetBrainsIde::PyCharm),
                ("Rider", JetBrainsIde::Rider),
                ("CLion", JetBrainsIde::CLion),
            ] {
                if apps_dir.join(format!("{}.app", app_name)).exists() {
                    ides.push(ide);
                }
            }
        }

        // Check for JetBrains Toolbox installations
        if let Some(toolbox_apps) = self.find_toolbox_apps() {
            ides.extend(toolbox_apps);
        }

        ides
    }

    fn plugins_for_ide(&self, ide: &JetBrainsIde) -> Vec<String> {
        let mut plugins = self.config.plugins.clone();

        // Add IDE-specific plugins
        let specific = match ide {
            JetBrainsIde::IntelliJ => &self.config.intellij,
            JetBrainsIde::WebStorm => &self.config.webstorm,
            JetBrainsIde::RustRover => &self.config.rustrover,
            JetBrainsIde::GoLand => &self.config.goland,
            JetBrainsIde::PyCharm => &self.config.pycharm,
            JetBrainsIde::Rider => &self.config.rider,
            JetBrainsIde::CLion => &self.config.clion,
        };

        if let Some(ref cfg) = specific {
            plugins.extend(cfg.plugins.clone());
        }

        plugins
    }

    fn install_plugins_for_ide(
        &self,
        ide: &JetBrainsIde,
        plugins: &[String],
    ) -> Result<(), IdeError> {
        println!("  Installing plugins for {:?}...", ide);

        // JetBrains IDEs can install plugins via command line
        // This uses the IDE's built-in plugin manager
        for plugin in plugins {
            println!("    Plugin: {}", plugin);
            // Note: JetBrains plugin installation is more complex
            // and may require the IDE to be running or use the Toolbox
        }

        println!("  Note: JetBrains plugins may require IDE restart");
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum JetBrainsIde {
    IntelliJ,
    WebStorm,
    RustRover,
    GoLand,
    PyCharm,
    Rider,
    CLion,
}
```

## Implementation Steps

1. Create ide module structure
2. Implement IdeConfig parsing
3. Implement VS Code extension installation
4. Implement recommendations file generation
5. Implement workspace settings application
6. Implement JetBrains IDE detection
7. Implement JetBrains plugin installation
8. Integrate with setup command
9. Add validation for extension IDs
10. Write tests for extension installation
11. Update documentation

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Manual extension installation | 10-20 per developer | 0 |
| Onboarding time for IDE setup | 30-60 minutes | 5 minutes |
| Extension version consistency | ~50% | >95% |
| ".vscode checked in" projects | ~30% | >80% |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| VS Code not installed | Medium | Low | Skip gracefully, don't fail setup |
| Extension ID typos | Medium | Low | Validate against marketplace |
| Rate limiting | Low | Medium | Batch installations |
| JetBrains CLI complexity | High | Medium | Document limitations, recommend Toolbox |
| Settings conflicts | Medium | Medium | Merge carefully, don't override user prefs |

## Dependencies

### New Dependencies
- None required (uses CLI tools)

### Prerequisite Tools
- VS Code (code CLI) for [vscode] section
- JetBrains IDE or Toolbox for [jetbrains] section

## Effort Estimate

| Task | Effort |
|------|--------|
| Module structure and config | 0.5 days |
| VS Code extension installation | 1 day |
| VS Code settings/recommendations | 1 day |
| JetBrains IDE detection | 1 day |
| JetBrains plugin installation | 1.5 days |
| Setup integration | 0.5 days |
| Testing | 1.5 days |
| Documentation | 0.5 days |
| **Total** | **7.5 days** |

## Files to Create/Modify

### New Files
- `src/ide/mod.rs`
- `src/ide/config.rs`
- `src/ide/vscode.rs`
- `src/ide/jetbrains.rs`
- `src/ide/common.rs`
- `tests/ide_integration.rs`

### Modified Files
- `src/config.rs` - Add IDE config parsing
- `src/lib.rs` - Export ide module
- `src/commands/setup_cmd.rs` - Integrate IDE setup
- `CLAUDE.md` - Document IDE sections

---

*PRD-040 v1.0 | IDE/Editor Extension Management | Priority: High*
