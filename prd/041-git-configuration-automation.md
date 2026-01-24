# PRD-041: Git Configuration Automation

## Overview

Enable Jarvy to configure Git settings including user identity, signing keys, default branch, hooks, and aliases as part of environment provisioning, eliminating the most common post-setup manual configuration step.

## Problem Statement

After Jarvy provisions a development environment, developers must still manually configure Git:

- Set user.name and user.email for every new machine
- Configure GPG or SSH key signing for commit verification
- Set up consistent defaults (default branch, pull strategy)
- Configure Git aliases that the team uses
- Install pre-commit hooks

This manual step is error-prone and often forgotten, leading to commits with wrong email addresses, unsigned commits, and inconsistent Git behavior across team members.

## Evidence

- "Configure your git email" is in every onboarding doc
- Commits from "unknown@users.noreply.github.com" appear regularly
- Teams have inconsistent default branch names (main vs master)
- Pre-commit hooks must be manually installed per-clone
- Git aliases are shared informally and inconsistently

## Requirements

### Functional Requirements

1. **User identity**: Configure user.name and user.email (global or repo-level)
2. **Commit signing**: Set up GPG or SSH key signing
3. **Default settings**: Configure default branch, pull strategy, editor
4. **Git aliases**: Install team-standard aliases
5. **Credential helpers**: Configure OS-specific credential storage
6. **Core settings**: Configure line endings, autocrlf, etc.
7. **Environment variables**: Optionally source from environment

### Non-Functional Requirements

1. **Non-destructive**: Warn before overwriting existing config
2. **Scope-aware**: Support global vs repository-level config
3. **Cross-platform**: Work on macOS, Linux, Windows
4. **Secure**: Never store secrets in config files
5. **Idempotent**: Safe to run multiple times

## Non-Goals

- SSH key generation (users should have keys already)
- GitHub/GitLab account setup
- Repository creation/cloning
- Git LFS configuration (future enhancement)
- Pre-commit hook content (separate PRD-048)

## Feature Specifications

### 1. Configuration Syntax

```toml
# jarvy.toml

[git]
# User identity (required for most workflows)
user_name = "John Doe"
user_email = "john@example.com"

# Or source from environment
user_name = { env = "GIT_USER_NAME" }
user_email = { env = "GIT_USER_EMAIL" }

# Commit signing
signing = true
signing_key = "~/.ssh/id_ed25519.pub"  # SSH key
# signing_key = "ABC123DEF456"          # GPG key ID
signing_format = "ssh"  # ssh or gpg (default: auto-detect)

# Default settings
default_branch = "main"
pull_rebase = true     # git config pull.rebase true
auto_stash = true      # git config rebase.autoStash true
push_autosetup = true  # git config push.autoSetupRemote true

# Editor
editor = "code --wait"  # Or vim, nano, etc.

# Line endings
autocrlf = "input"  # true, false, input (default: input on Unix, true on Windows)
eol = "lf"          # lf or crlf

# Credential helper
credential_helper = "osxkeychain"  # osxkeychain, manager-core, cache, store
# Auto-detected by default based on OS

# Scope: global (default) or local (repo-level)
scope = "global"  # global, local
```

### 2. Git Aliases

```toml
[git.aliases]
# Common shortcuts
co = "checkout"
br = "branch"
ci = "commit"
st = "status"
cp = "cherry-pick"

# Useful compound commands
lg = "log --oneline --graph --decorate"
unstage = "reset HEAD --"
last = "log -1 HEAD"
undo = "reset --soft HEAD~1"

# Team-specific workflows
sync = "!git fetch origin && git rebase origin/main"
cleanup = "!git branch --merged | grep -v main | xargs git branch -d"
```

### 3. Environment Variable Support

```toml
[git]
# Source identity from environment (useful for CI/secrets)
user_name = { env = "GIT_USER_NAME" }
user_email = { env = "GIT_USER_EMAIL" }
signing_key = { env = "GIT_SIGNING_KEY" }

# Fallback values if env not set
user_name = { env = "GIT_USER_NAME", default = "Developer" }
user_email = { env = "GIT_USER_EMAIL", default = "dev@example.com" }
```

### 4. Interactive Mode

```bash
# If identity not configured, prompt interactively
jarvy setup

# Output:
# Git Configuration
# =================
#
# Enter your name: John Doe
# Enter your email: john@example.com
# Enable commit signing? [Y/n]: y
# Path to signing key [~/.ssh/id_ed25519.pub]:
#
# ✓ Git configured successfully
```

## Technical Approach

### Module Structure

```
src/
  git/
    mod.rs           # Public API
    config.rs        # Git configuration parsing
    setup.rs         # Git configuration application
    signing.rs       # Signing key setup
    aliases.rs       # Alias configuration
    identity.rs      # User identity handling
```

### Configuration Types

```rust
// src/git/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GitConfig {
    // User identity
    pub user_name: Option<ConfigValue>,
    pub user_email: Option<ConfigValue>,

    // Signing
    #[serde(default)]
    pub signing: bool,
    pub signing_key: Option<String>,
    pub signing_format: Option<SigningFormat>,

    // Defaults
    pub default_branch: Option<String>,
    #[serde(default)]
    pub pull_rebase: bool,
    #[serde(default)]
    pub auto_stash: bool,
    #[serde(default)]
    pub push_autosetup: bool,

    // Editor
    pub editor: Option<String>,

    // Line endings
    pub autocrlf: Option<AutoCrlf>,
    pub eol: Option<String>,

    // Credential helper
    pub credential_helper: Option<String>,

    // Scope
    #[serde(default)]
    pub scope: ConfigScope,

    // Aliases
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ConfigValue {
    Plain(String),
    FromEnv {
        env: String,
        default: Option<String>,
    },
}

impl ConfigValue {
    pub fn resolve(&self) -> Option<String> {
        match self {
            ConfigValue::Plain(s) => Some(s.clone()),
            ConfigValue::FromEnv { env, default } => {
                std::env::var(env).ok().or_else(|| default.clone())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConfigScope {
    #[default]
    Global,
    Local,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SigningFormat {
    Ssh,
    Gpg,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoCrlf {
    True,
    False,
    Input,
}
```

### Git Setup Handler

```rust
// src/git/setup.rs
use std::process::Command;

pub struct GitSetup {
    config: GitConfig,
    project_dir: Option<PathBuf>,
}

impl GitSetup {
    pub fn configure(&self) -> Result<(), GitError> {
        // Configure identity
        if let Some(ref name) = self.config.user_name {
            if let Some(value) = name.resolve() {
                self.set_config("user.name", &value)?;
            }
        }

        if let Some(ref email) = self.config.user_email {
            if let Some(value) = email.resolve() {
                self.set_config("user.email", &value)?;
            }
        }

        // Configure signing
        if self.config.signing {
            self.configure_signing()?;
        }

        // Configure defaults
        if let Some(ref branch) = self.config.default_branch {
            self.set_config("init.defaultBranch", branch)?;
        }

        if self.config.pull_rebase {
            self.set_config("pull.rebase", "true")?;
        }

        if self.config.auto_stash {
            self.set_config("rebase.autoStash", "true")?;
        }

        if self.config.push_autosetup {
            self.set_config("push.autoSetupRemote", "true")?;
        }

        // Configure editor
        if let Some(ref editor) = self.config.editor {
            self.set_config("core.editor", editor)?;
        }

        // Configure line endings
        if let Some(ref autocrlf) = self.config.autocrlf {
            let value = match autocrlf {
                AutoCrlf::True => "true",
                AutoCrlf::False => "false",
                AutoCrlf::Input => "input",
            };
            self.set_config("core.autocrlf", value)?;
        }

        // Configure credential helper
        self.configure_credential_helper()?;

        // Configure aliases
        for (alias, command) in &self.config.aliases {
            self.set_config(&format!("alias.{}", alias), command)?;
        }

        Ok(())
    }

    fn set_config(&self, key: &str, value: &str) -> Result<(), GitError> {
        let scope_flag = match self.config.scope {
            ConfigScope::Global => "--global",
            ConfigScope::Local => "--local",
        };

        let status = Command::new("git")
            .args(["config", scope_flag, key, value])
            .current_dir(self.project_dir.as_deref().unwrap_or(Path::new(".")))
            .status()?;

        if !status.success() {
            return Err(GitError::ConfigFailed(key.to_string()));
        }

        println!("  Set git config {}: {}", key, value);
        Ok(())
    }

    fn configure_signing(&self) -> Result<(), GitError> {
        self.set_config("commit.gpgsign", "true")?;

        if let Some(ref key) = self.config.signing_key {
            // Expand tilde
            let key_path = shellexpand::tilde(key);

            let format = self.config.signing_format.unwrap_or_else(|| {
                // Auto-detect based on key path
                if key_path.ends_with(".pub") {
                    SigningFormat::Ssh
                } else {
                    SigningFormat::Gpg
                }
            });

            match format {
                SigningFormat::Ssh => {
                    self.set_config("gpg.format", "ssh")?;
                    self.set_config("user.signingkey", &key_path)?;
                }
                SigningFormat::Gpg => {
                    self.set_config("user.signingkey", &key_path)?;
                }
            }
        }

        Ok(())
    }

    fn configure_credential_helper(&self) -> Result<(), GitError> {
        let helper = self.config.credential_helper.as_deref()
            .unwrap_or_else(|| self.default_credential_helper());

        self.set_config("credential.helper", helper)?;
        Ok(())
    }

    fn default_credential_helper(&self) -> &'static str {
        #[cfg(target_os = "macos")]
        { "osxkeychain" }

        #[cfg(target_os = "linux")]
        { "cache" }

        #[cfg(target_os = "windows")]
        { "manager-core" }
    }
}
```

### Interactive Identity Setup

```rust
// src/git/identity.rs
use inquire::{Confirm, Text};

pub fn prompt_identity(existing: &GitConfig) -> Result<GitConfig, GitError> {
    let mut config = existing.clone();

    // Check if identity already configured
    let current_name = get_current_config("user.name");
    let current_email = get_current_config("user.email");

    if current_name.is_some() && current_email.is_some() {
        println!("Git identity already configured:");
        println!("  Name:  {}", current_name.as_ref().unwrap());
        println!("  Email: {}", current_email.as_ref().unwrap());

        if !Confirm::new("Update Git identity?").with_default(false).prompt()? {
            return Ok(config);
        }
    }

    // Prompt for identity
    let name = Text::new("Enter your name:")
        .with_default(current_name.as_deref().unwrap_or(""))
        .prompt()?;
    config.user_name = Some(ConfigValue::Plain(name));

    let email = Text::new("Enter your email:")
        .with_default(current_email.as_deref().unwrap_or(""))
        .prompt()?;
    config.user_email = Some(ConfigValue::Plain(email));

    // Prompt for signing
    if Confirm::new("Enable commit signing?").with_default(true).prompt()? {
        config.signing = true;

        let default_key = find_default_signing_key();
        let key = Text::new("Path to signing key:")
            .with_default(&default_key)
            .prompt()?;
        config.signing_key = Some(key);
    }

    Ok(config)
}

fn find_default_signing_key() -> String {
    let home = dirs::home_dir().unwrap_or_default();

    // Check for common SSH keys
    for key in &["id_ed25519.pub", "id_rsa.pub"] {
        let path = home.join(".ssh").join(key);
        if path.exists() {
            return path.to_string_lossy().to_string();
        }
    }

    "~/.ssh/id_ed25519.pub".to_string()
}
```

## Implementation Steps

1. Create git module structure
2. Implement GitConfig parsing with ConfigValue enum
3. Implement environment variable resolution
4. Implement git config setting with scope support
5. Implement signing configuration (SSH and GPG)
6. Implement credential helper configuration
7. Implement alias configuration
8. Implement interactive identity prompting
9. Integrate with setup command
10. Add validation for config values
11. Write tests for git configuration
12. Update documentation

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Manual git config commands | 5-10 per machine | 0 |
| Commits with wrong email | ~10% | <1% |
| Unsigned commits (for signing teams) | ~30% | <5% |
| Onboarding git setup time | 10-15 minutes | 2 minutes |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Overwriting user preferences | Medium | High | Warn before overwriting, use --force flag |
| Invalid signing key path | Medium | Medium | Validate key exists |
| Git not installed | Low | High | Check git availability first |
| Scope confusion | Medium | Low | Clear documentation, sensible defaults |
| Credential helper not available | Low | Medium | Fall back to 'cache' or 'store' |

## Dependencies

### New Dependencies
- `shellexpand` - For tilde expansion in paths

### Prerequisite Tools
- Git (should be in [provisioner])

## Effort Estimate

| Task | Effort |
|------|--------|
| Module structure and config | 0.5 days |
| Config value resolution | 0.5 days |
| Git config setting | 1 day |
| Signing configuration | 1 day |
| Credential helpers | 0.5 days |
| Aliases | 0.5 days |
| Interactive prompts | 1 day |
| Setup integration | 0.5 days |
| Testing | 1 day |
| Documentation | 0.5 days |
| **Total** | **7 days** |

## Files to Create/Modify

### New Files
- `src/git/mod.rs`
- `src/git/config.rs`
- `src/git/setup.rs`
- `src/git/signing.rs`
- `src/git/aliases.rs`
- `src/git/identity.rs`
- `tests/git_integration.rs`

### Modified Files
- `src/config.rs` - Add git config parsing
- `src/lib.rs` - Export git module
- `src/commands/setup_cmd.rs` - Integrate git setup
- `Cargo.toml` - Add shellexpand dependency
- `CLAUDE.md` - Document [git] section

---

*PRD-041 v1.0 | Git Configuration Automation | Priority: High*
