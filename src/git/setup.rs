//! Git configuration setup - applies git config settings

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

use super::config::{ConfigScope, GitConfig, SigningFormat};

/// Errors that can occur during git configuration
#[derive(Debug, Error)]
pub enum GitError {
    #[error("Failed to set git config '{0}': {1}")]
    ConfigFailed(String, String),

    #[error("Git command failed: {0}")]
    CommandFailed(#[from] std::io::Error),

    #[error("Git not installed")]
    GitNotInstalled,

    #[error("Signing key not found: {0}")]
    #[allow(dead_code)] // Reserved for future validation
    SigningKeyNotFound(String),
}

/// Git setup handler
pub struct GitSetup {
    config: GitConfig,
    project_dir: Option<PathBuf>,
    quiet: bool,
}

impl GitSetup {
    /// Create a new GitSetup with the given configuration
    pub fn new(config: GitConfig) -> Self {
        Self {
            config,
            project_dir: None,
            quiet: false,
        }
    }

    /// Set the project directory for local scope configurations
    #[allow(dead_code)] // Builder pattern for advanced usage
    pub fn with_project_dir(mut self, dir: PathBuf) -> Self {
        self.project_dir = Some(dir);
        self
    }

    /// Set quiet mode (suppress output)
    #[allow(dead_code)] // Builder pattern for quiet mode
    pub fn quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// Check if git is installed
    pub fn check_git_installed() -> Result<(), GitError> {
        let output = Command::new("git").arg("--version").output()?;

        if !output.status.success() {
            return Err(GitError::GitNotInstalled);
        }
        Ok(())
    }

    /// Apply all git configuration settings
    pub fn configure(&self) -> Result<(), GitError> {
        Self::check_git_installed()?;

        // Configure identity
        self.configure_identity()?;

        // Configure signing if enabled
        if self.config.signing {
            self.configure_signing()?;
        }

        // Configure defaults
        self.configure_defaults()?;

        // Configure editor
        if let Some(ref editor) = self.config.editor {
            self.set_config("core.editor", editor)?;
        }

        // Configure line endings
        self.configure_line_endings()?;

        // Configure credential helper
        self.configure_credential_helper()?;

        // Configure aliases
        self.configure_aliases()?;

        Ok(())
    }

    /// Configure user identity (name and email)
    fn configure_identity(&self) -> Result<(), GitError> {
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

        Ok(())
    }

    /// Configure commit signing
    fn configure_signing(&self) -> Result<(), GitError> {
        self.set_config("commit.gpgsign", "true")?;

        if let Some(ref key) = self.config.signing_key {
            // Expand tilde in path
            let key_path = shellexpand::tilde(key);

            // Auto-detect format if not specified
            let format = self.config.signing_format.unwrap_or_else(|| {
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

    /// Configure default settings (branch, pull strategy, etc.)
    fn configure_defaults(&self) -> Result<(), GitError> {
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

        Ok(())
    }

    /// Configure line ending settings
    fn configure_line_endings(&self) -> Result<(), GitError> {
        if let Some(ref autocrlf) = self.config.autocrlf {
            self.set_config("core.autocrlf", autocrlf.as_str())?;
        }

        if let Some(ref eol) = self.config.eol {
            self.set_config("core.eol", eol)?;
        }

        Ok(())
    }

    /// Configure credential helper (auto-detect based on OS if not specified)
    fn configure_credential_helper(&self) -> Result<(), GitError> {
        let helper = self
            .config
            .credential_helper
            .as_deref()
            .unwrap_or_else(|| Self::default_credential_helper());

        self.set_config("credential.helper", helper)?;
        Ok(())
    }

    /// Configure git aliases
    fn configure_aliases(&self) -> Result<(), GitError> {
        for (alias, command) in &self.config.aliases {
            self.set_config(&format!("alias.{alias}"), command)?;
        }
        Ok(())
    }

    /// Set a single git config value
    fn set_config(&self, key: &str, value: &str) -> Result<(), GitError> {
        let scope_flag = match self.config.scope {
            ConfigScope::Global => "--global",
            ConfigScope::Local => "--local",
        };

        let mut cmd = Command::new("git");
        cmd.args(["config", scope_flag, key, value]);

        if let Some(ref dir) = self.project_dir {
            cmd.current_dir(dir);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::ConfigFailed(key.to_string(), stderr.to_string()));
        }

        if !self.quiet {
            println!("  Set git config {key}: {value}");
        }

        Ok(())
    }

    /// Get the default credential helper for the current OS
    fn default_credential_helper() -> &'static str {
        #[cfg(target_os = "macos")]
        {
            "osxkeychain"
        }

        #[cfg(target_os = "linux")]
        {
            "cache"
        }

        #[cfg(target_os = "windows")]
        {
            "manager-core"
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            "cache"
        }
    }
}

/// Get a current git config value
#[allow(dead_code)] // Public API for config inspection
pub fn get_git_config(key: &str, scope: ConfigScope) -> Option<String> {
    let scope_flag = match scope {
        ConfigScope::Global => "--global",
        ConfigScope::Local => "--local",
    };

    let output = Command::new("git")
        .args(["config", scope_flag, "--get", key])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Check if a signing key file exists
#[allow(dead_code)] // Public API for key validation
pub fn signing_key_exists(key_path: &str) -> bool {
    let expanded = shellexpand::tilde(key_path);
    Path::new(expanded.as_ref()).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::config::AutoCrlf;

    #[test]
    fn test_default_credential_helper() {
        let helper = GitSetup::default_credential_helper();
        // Should return a valid helper name
        assert!(!helper.is_empty());
    }

    #[test]
    fn test_git_setup_builder() {
        let config = GitConfig::default();
        let setup = GitSetup::new(config.clone())
            .with_project_dir(PathBuf::from("/tmp/test"))
            .quiet(true);

        assert!(setup.quiet);
        assert_eq!(setup.project_dir, Some(PathBuf::from("/tmp/test")));
    }

    #[test]
    fn test_autocrlf_as_str() {
        assert_eq!(AutoCrlf::True.as_str(), "true");
        assert_eq!(AutoCrlf::False.as_str(), "false");
        assert_eq!(AutoCrlf::Input.as_str(), "input");
    }
}
