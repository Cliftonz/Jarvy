# PRD-003: Post-Install Hooks

## Overview

Add support for custom shell scripts that run after tool installation, enabling shell configuration, environment setup, and project-specific initialization.

## Problem Statement

Teams cannot fully automate their development environment setup with Jarvy because:
1. Tools often need shell configuration (PATH updates, aliases, completions)
2. Projects need post-install steps (clone repos, run build scripts, create directories)
3. Some tools require manual initialization (aws configure, docker login)

This blocks ~40% of potential adopters who need more than just tool installation.

## Evidence

- Competitors (Nix/Devenv, Vagrant, DevPod) all support provisioning scripts
- Common requests: "How do I run `nvm use` after installing nvm?"
- Setup.rs has hardcoded shell configuration that should be user-configurable

## Requirements

### Functional Requirements

1. **Per-tool hooks**: Run script after specific tool installs
2. **Global hooks**: Run scripts after all tools are installed
3. **Shell selection**: Support bash, zsh, sh, powershell
4. **Environment access**: Scripts can access installed tool paths
5. **Failure handling**: Option to continue or abort on hook failure
6. **Dry-run mode**: Show what hooks would run without executing

### Non-Functional Requirements

1. Scripts run in user's default shell
2. Timeout after 5 minutes by default (configurable)
3. Output captured and displayed in real-time
4. Cross-platform path handling

## Proposed Config Syntax

```toml
# jarvy.toml

[tools]
node = "20"
python = "3.12"
docker = "latest"

# Per-tool hooks
[hooks.node]
post_install = """
nvm install 20
nvm alias default 20
"""

[hooks.python]
post_install = "pip install poetry"

# Global hooks
[hooks]
pre_setup = "echo 'Starting Jarvy setup...'"
post_setup = """
git clone https://github.com/company/dotfiles ~/.dotfiles
~/.dotfiles/install.sh
"""

# Hook configuration
[hooks.config]
shell = "zsh"           # or "bash", "sh", "powershell"
timeout = 300           # seconds
continue_on_error = false
```

## Technical Approach

### Hook Execution Flow

```
1. Parse jarvy.toml
2. Run hooks.pre_setup (if defined)
3. For each tool:
   a. Install tool
   b. Run hooks.{tool}.post_install (if defined)
4. Run hooks.post_setup (if defined)
5. Report results
```

### Implementation

```rust
// src/hooks.rs
pub struct HookConfig {
    pub shell: String,
    pub timeout: Duration,
    pub continue_on_error: bool,
}

pub struct Hook {
    pub script: String,
    pub config: HookConfig,
}

impl Hook {
    pub fn execute(&self, env: &HashMap<String, String>) -> Result<(), HookError> {
        let shell = match self.config.shell.as_str() {
            "bash" => "/bin/bash",
            "zsh" => "/bin/zsh",
            "sh" => "/bin/sh",
            "powershell" => "powershell.exe",
            _ => return Err(HookError::UnsupportedShell),
        };

        let output = Command::new(shell)
            .arg("-c")
            .arg(&self.script)
            .envs(env)
            .timeout(self.config.timeout)
            .output()?;

        if !output.status.success() {
            return Err(HookError::ScriptFailed {
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        Ok(())
    }
}
```

### Config Parsing

Extend `src/config.rs`:

```rust
#[derive(Deserialize)]
pub struct Config {
    pub tools: HashMap<String, ToolConfig>,
    #[serde(default)]
    pub hooks: HooksConfig,
}

#[derive(Deserialize, Default)]
pub struct HooksConfig {
    pub pre_setup: Option<String>,
    pub post_setup: Option<String>,
    #[serde(flatten)]
    pub tool_hooks: HashMap<String, ToolHooks>,
    #[serde(default)]
    pub config: HookSettings,
}

#[derive(Deserialize)]
pub struct ToolHooks {
    pub post_install: Option<String>,
}
```

## Environment Variables Passed to Hooks

| Variable | Description | Example |
|----------|-------------|---------|
| `JARVY_TOOL` | Current tool name | `node` |
| `JARVY_VERSION` | Installed version | `20.10.0` |
| `JARVY_OS` | Operating system | `macos`, `linux`, `windows` |
| `JARVY_ARCH` | Architecture | `x86_64`, `aarch64` |
| `JARVY_HOME` | Jarvy config dir | `~/.jarvy` |
| `PATH` | Updated with new tool | `/usr/local/bin:...` |

## CLI Integration

```bash
# Normal setup runs hooks
jarvy setup

# Skip hooks
jarvy setup --no-hooks

# Run only hooks (tools already installed)
jarvy hooks

# Dry-run to see what would execute
jarvy setup --dry-run
```

## Implementation Steps

1. Add `HooksConfig` to `src/config.rs`
2. Create `src/hooks.rs` with `Hook` struct and execution logic
3. Add timeout support using `wait_timeout` crate
4. Integrate hook execution into main setup flow
5. Add `--no-hooks` and `--dry-run` CLI flags
6. Add environment variable injection
7. Write integration tests
8. Update documentation with examples

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Post-install automation | 0% | 100% |
| Setup completion rate | ~60% (manual steps needed) | 95% |
| Config complexity | Low | Medium (acceptable) |

## Risks

1. **Security**: Arbitrary script execution
   - Mitigation: Warn users about untrusted jarvy.toml files
   - Mitigation: Add `--confirm-hooks` flag for CI
2. **Cross-platform scripts**: bash scripts won't work on Windows
   - Mitigation: Detect OS and warn if script uses wrong shell
   - Mitigation: Support separate `post_install_windows` key
3. **Long-running scripts**: Could hang setup
   - Mitigation: Default 5-minute timeout with override

## Platform-Specific Hooks (Future)

```toml
[hooks.node]
post_install = "nvm use 20"
post_install_windows = "nvm use 20.0.0"
post_install_macos = """
nvm use 20
# macOS-specific setup
"""
```

## Dependencies

- `wait_timeout` crate for script timeouts
- No other new dependencies

## Effort Estimate

- Config parsing: 1 day
- Hook execution: 1.5 days
- CLI integration: 0.5 days
- Testing: 1 day
- Documentation: 0.5 days

## Files to Modify

- `src/config.rs` - Add hooks config parsing
- `src/hooks.rs` - New file for hook execution
- `src/main.rs` - Integrate hooks into setup flow
- `src/cli.rs` - Add --no-hooks, --dry-run flags
- `Cargo.toml` - Add wait_timeout dependency
