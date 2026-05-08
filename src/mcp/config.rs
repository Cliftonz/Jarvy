//! MCP Configuration
//!
//! Handles loading and managing MCP-specific configuration from ~/.jarvy/mcp-config.toml
//!
//! ## Configuration File Format
//!
//! ```toml
//! [mcp]
//! # If set, only these tools can be installed via MCP
//! allowlist = ["git", "docker", "node", "python"]
//!
//! # These tools are never installable via MCP (takes precedence over allowlist)
//! denylist = ["brew"]  # Don't let MCP install package managers
//!
//! # Require confirmation for all installs (default: true)
//! require_confirmation = true
//!
//! # Rate limits
//! max_checks_per_minute = 10
//! max_installs_per_minute = 3
//!
//! # Audit logging
//! audit_log = "~/.jarvy/mcp-audit.log"
//! ```

use crate::mcp::error::{McpError, McpResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    /// MCP-specific settings
    #[serde(default)]
    pub mcp: McpSettings,
}

/// MCP-specific settings within the config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSettings {
    /// If set, only these tools can be installed via MCP
    #[serde(default)]
    pub allowlist: Option<Vec<String>>,

    /// These tools are never installable via MCP (takes precedence over allowlist)
    #[serde(default)]
    pub denylist: Option<Vec<String>>,

    /// Require confirmation for all installs (default: true)
    #[serde(default = "default_require_confirmation")]
    pub require_confirmation: bool,

    /// Maximum tool checks per minute (default: 10)
    #[serde(default = "default_max_checks_per_minute")]
    pub max_checks_per_minute: u32,

    /// Maximum installs per minute (default: 3)
    #[serde(default = "default_max_installs_per_minute")]
    pub max_installs_per_minute: u32,

    /// Path to audit log file (default: ~/.jarvy/mcp-audit.log)
    #[serde(default = "default_audit_log")]
    pub audit_log: String,

    /// Tools that are always allowed without confirmation (user can add via "Always allow" option)
    #[serde(default)]
    pub always_allow: Vec<String>,
}

fn default_require_confirmation() -> bool {
    true
}

fn default_max_checks_per_minute() -> u32 {
    10
}

fn default_max_installs_per_minute() -> u32 {
    3
}

fn default_audit_log() -> String {
    "~/.jarvy/mcp-audit.log".to_string()
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            allowlist: None,
            denylist: None,
            require_confirmation: default_require_confirmation(),
            max_checks_per_minute: default_max_checks_per_minute(),
            max_installs_per_minute: default_max_installs_per_minute(),
            audit_log: default_audit_log(),
            always_allow: Vec::new(),
        }
    }
}

impl McpConfig {
    /// Load configuration from the default path (~/.jarvy/mcp-config.toml)
    pub fn load_default() -> McpResult<Self> {
        let config_path = Self::default_config_path()?;
        if config_path.exists() {
            Self::load_from(&config_path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration from a specific path
    pub fn load_from(path: &Path) -> McpResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: McpConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get the default configuration file path
    pub fn default_config_path() -> McpResult<PathBuf> {
        crate::paths::mcp_config_toml().map_err(|e| McpError::config_error(format!("{e}")))
    }

    /// Get the expanded audit log path
    pub fn audit_log_path(&self) -> McpResult<PathBuf> {
        let path = &self.mcp.audit_log;
        if let Some(stripped) = path.strip_prefix("~/") {
            let home = dirs::home_dir()
                .ok_or_else(|| McpError::config_error("Could not determine home directory"))?;
            Ok(home.join(stripped))
        } else {
            Ok(PathBuf::from(path))
        }
    }

    /// Check if a tool is in the denylist
    pub fn is_denied(&self, tool: &str) -> bool {
        if let Some(ref denylist) = self.mcp.denylist {
            denylist.iter().any(|t| t.eq_ignore_ascii_case(tool))
        } else {
            false
        }
    }

    /// Check if a tool is allowed (considering allowlist if configured)
    pub fn is_allowed(&self, tool: &str) -> bool {
        // Denylist takes precedence
        if self.is_denied(tool) {
            return false;
        }

        // If allowlist is configured, tool must be in it
        if let Some(ref allowlist) = self.mcp.allowlist {
            allowlist.iter().any(|t| t.eq_ignore_ascii_case(tool))
        } else {
            // No allowlist = all tools allowed (except denied)
            true
        }
    }

    /// Check if a tool should skip confirmation (in always_allow list)
    pub fn skip_confirmation(&self, tool: &str) -> bool {
        self.mcp
            .always_allow
            .iter()
            .any(|t| t.eq_ignore_ascii_case(tool))
    }

    /// Add a tool to the always_allow list and save the config
    #[allow(dead_code)] // Public API for MCP configuration
    pub fn add_always_allow(&mut self, tool: &str) -> McpResult<()> {
        if !self.skip_confirmation(tool) {
            self.mcp.always_allow.push(tool.to_string());
            self.save()?;
        }
        Ok(())
    }

    /// Save the configuration to the default path
    #[allow(dead_code)] // Public API for MCP configuration
    pub fn save(&self) -> McpResult<()> {
        let config_path = Self::default_config_path()?;

        // Ensure the directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| McpError::config_error(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(&config_path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = McpConfig::default();
        assert!(config.mcp.require_confirmation);
        assert_eq!(config.mcp.max_checks_per_minute, 10);
        assert_eq!(config.mcp.max_installs_per_minute, 3);
        assert!(config.mcp.allowlist.is_none());
        assert!(config.mcp.denylist.is_none());
    }

    #[test]
    fn test_is_denied() {
        let mut config = McpConfig::default();
        config.mcp.denylist = Some(vec!["brew".to_string(), "apt".to_string()]);

        assert!(config.is_denied("brew"));
        assert!(config.is_denied("BREW")); // case insensitive
        assert!(config.is_denied("apt"));
        assert!(!config.is_denied("git"));
    }

    #[test]
    fn test_is_allowed_no_lists() {
        let config = McpConfig::default();
        assert!(config.is_allowed("git"));
        assert!(config.is_allowed("anything"));
    }

    #[test]
    fn test_is_allowed_with_allowlist() {
        let mut config = McpConfig::default();
        config.mcp.allowlist = Some(vec!["git".to_string(), "docker".to_string()]);

        assert!(config.is_allowed("git"));
        assert!(config.is_allowed("docker"));
        assert!(!config.is_allowed("vim"));
    }

    #[test]
    fn test_denylist_takes_precedence() {
        let mut config = McpConfig::default();
        config.mcp.allowlist = Some(vec!["git".to_string(), "brew".to_string()]);
        config.mcp.denylist = Some(vec!["brew".to_string()]);

        assert!(config.is_allowed("git"));
        assert!(!config.is_allowed("brew")); // denied even though in allowlist
    }

    #[test]
    fn test_skip_confirmation() {
        let mut config = McpConfig::default();
        config.mcp.always_allow = vec!["git".to_string()];

        assert!(config.skip_confirmation("git"));
        assert!(!config.skip_confirmation("docker"));
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_str = r#"
[mcp]
allowlist = ["git", "docker"]
denylist = ["brew"]
require_confirmation = false
max_checks_per_minute = 20
max_installs_per_minute = 5
audit_log = "/var/log/jarvy-mcp.log"
always_allow = ["node"]
"#;

        let config: McpConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.mcp.allowlist,
            Some(vec!["git".to_string(), "docker".to_string()])
        );
        assert_eq!(config.mcp.denylist, Some(vec!["brew".to_string()]));
        assert!(!config.mcp.require_confirmation);
        assert_eq!(config.mcp.max_checks_per_minute, 20);
        assert_eq!(config.mcp.max_installs_per_minute, 5);
        assert_eq!(config.mcp.audit_log, "/var/log/jarvy-mcp.log");
        assert_eq!(config.mcp.always_allow, vec!["node".to_string()]);
    }
}
