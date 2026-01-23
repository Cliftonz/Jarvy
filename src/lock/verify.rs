//! Lock file verification
//!
//! Verifies that installed tool versions match the lock file.

use super::{LockError, LockFile, LockedTool};
use crate::tools::common::has;

/// Verification result for a single tool
#[derive(Debug, Clone)]
pub struct ToolVerification {
    /// Tool name
    pub name: String,
    /// Verification status
    pub status: VerificationStatus,
    /// Locked version
    pub locked_version: String,
    /// Currently installed version (if available)
    pub installed_version: Option<String>,
}

/// Status of a tool verification
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationStatus {
    /// Tool matches lock file
    Match,
    /// Version mismatch
    VersionMismatch,
    /// Tool not installed
    NotInstalled,
    /// Tool installed but not in lock file
    #[allow(dead_code)] // Reserved for unlocked tool detection
    NotLocked,
    /// Unable to determine version
    Unknown,
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationStatus::Match => write!(f, "match"),
            VerificationStatus::VersionMismatch => write!(f, "version mismatch"),
            VerificationStatus::NotInstalled => write!(f, "not installed"),
            VerificationStatus::NotLocked => write!(f, "not locked"),
            VerificationStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Overall verification result
#[derive(Debug)]
pub struct VerificationResult {
    /// Individual tool results
    pub tools: Vec<ToolVerification>,
    /// Whether all tools match
    pub all_match: bool,
    /// Number of matched tools
    pub matched: usize,
    /// Number of mismatched tools
    pub mismatched: usize,
    /// Number of missing tools
    pub missing: usize,
}

impl VerificationResult {
    /// Create a new verification result
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            all_match: true,
            matched: 0,
            mismatched: 0,
            missing: 0,
        }
    }

    /// Add a tool verification
    pub fn add(&mut self, verification: ToolVerification) {
        match verification.status {
            VerificationStatus::Match => self.matched += 1,
            VerificationStatus::VersionMismatch => {
                self.mismatched += 1;
                self.all_match = false;
            }
            VerificationStatus::NotInstalled => {
                self.missing += 1;
                self.all_match = false;
            }
            VerificationStatus::NotLocked | VerificationStatus::Unknown => {
                self.all_match = false;
            }
        }
        self.tools.push(verification);
    }

    /// Get all mismatched tools
    #[allow(dead_code)] // Public API for lock verification results
    pub fn mismatches(&self) -> Vec<&ToolVerification> {
        self.tools
            .iter()
            .filter(|t| t.status == VerificationStatus::VersionMismatch)
            .collect()
    }

    /// Get all missing tools
    #[allow(dead_code)] // Public API for lock verification results
    pub fn missing_tools(&self) -> Vec<&ToolVerification> {
        self.tools
            .iter()
            .filter(|t| t.status == VerificationStatus::NotInstalled)
            .collect()
    }
}

impl Default for VerificationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Verify lock file against current environment
pub fn verify_lock(lock: &LockFile, platform: &str) -> VerificationResult {
    let mut result = VerificationResult::new();

    for (name, locked_tool) in &lock.tools {
        // Check for platform-specific override
        let tool = lock.get_tool(name, platform).unwrap_or(locked_tool);
        let verification = verify_tool(name, tool);
        result.add(verification);
    }

    // Also check platform-specific tools not in common
    if let Some(platform_tools) = lock.platforms.get(platform) {
        for (name, tool) in platform_tools {
            if !lock.tools.contains_key(name) {
                let verification = verify_tool(name, tool);
                result.add(verification);
            }
        }
    }

    result
}

/// Verify a single tool against its lock entry
fn verify_tool(name: &str, locked: &LockedTool) -> ToolVerification {
    // Check if tool is installed
    if !has(name) {
        return ToolVerification {
            name: name.to_string(),
            status: VerificationStatus::NotInstalled,
            locked_version: locked.version.clone(),
            installed_version: None,
        };
    }

    // Get installed version
    let installed_version = super::generate::get_installed_version(name);

    match &installed_version {
        Some(installed) => {
            if versions_match(&locked.version, installed) {
                ToolVerification {
                    name: name.to_string(),
                    status: VerificationStatus::Match,
                    locked_version: locked.version.clone(),
                    installed_version: Some(installed.clone()),
                }
            } else {
                ToolVerification {
                    name: name.to_string(),
                    status: VerificationStatus::VersionMismatch,
                    locked_version: locked.version.clone(),
                    installed_version: Some(installed.clone()),
                }
            }
        }
        None => ToolVerification {
            name: name.to_string(),
            status: VerificationStatus::Unknown,
            locked_version: locked.version.clone(),
            installed_version: None,
        },
    }
}

/// Check if two version strings match
fn versions_match(locked: &str, installed: &str) -> bool {
    // Normalize versions for comparison
    let locked_normalized = normalize_version(locked);
    let installed_normalized = normalize_version(installed);

    // Exact match
    if locked_normalized == installed_normalized {
        return true;
    }

    // Check if installed version starts with locked version (prefix match)
    // e.g., locked "2.45" matches installed "2.45.0"
    if installed_normalized.starts_with(&locked_normalized) {
        let remainder = &installed_normalized[locked_normalized.len()..];
        if remainder.is_empty() || remainder.starts_with('.') {
            return true;
        }
    }

    false
}

/// Normalize a version string
fn normalize_version(version: &str) -> String {
    // Remove 'v' prefix
    let version = version.strip_prefix('v').unwrap_or(version);

    // Trim whitespace
    version.trim().to_string()
}

/// Verify and optionally update tools to match lock file
#[allow(dead_code)] // Public API for lock verification with output
pub fn verify_and_report(
    lock: &LockFile,
    platform: &str,
    verbose: bool,
) -> Result<VerificationResult, LockError> {
    let result = verify_lock(lock, platform);

    if verbose {
        for tool in &result.tools {
            match tool.status {
                VerificationStatus::Match => {
                    println!("  {} {} [match]", tool.name, tool.locked_version);
                }
                VerificationStatus::VersionMismatch => {
                    println!(
                        "  {} {} != {} [mismatch]",
                        tool.name,
                        tool.installed_version.as_deref().unwrap_or("?"),
                        tool.locked_version
                    );
                }
                VerificationStatus::NotInstalled => {
                    println!("  {} {} [not installed]", tool.name, tool.locked_version);
                }
                VerificationStatus::NotLocked => {
                    println!("  {} [not in lock file]", tool.name);
                }
                VerificationStatus::Unknown => {
                    println!("  {} {} [unknown]", tool.name, tool.locked_version);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_versions_match_exact() {
        assert!(versions_match("2.45.0", "2.45.0"));
        assert!(versions_match("1.0", "1.0"));
    }

    #[test]
    fn test_versions_match_v_prefix() {
        assert!(versions_match("v2.45.0", "2.45.0"));
        assert!(versions_match("2.45.0", "v2.45.0"));
    }

    #[test]
    fn test_versions_match_prefix() {
        assert!(versions_match("2.45", "2.45.0"));
        assert!(versions_match("2.45", "2.45.1"));
    }

    #[test]
    fn test_versions_mismatch() {
        assert!(!versions_match("2.45.0", "2.46.0"));
        assert!(!versions_match("2.45", "2.46.0"));
        assert!(!versions_match("1.0", "2.0"));
    }

    #[test]
    fn test_normalize_version() {
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
        assert_eq!(normalize_version("  1.2.3  "), "1.2.3");
        assert_eq!(normalize_version("v1.2.3-beta"), "1.2.3-beta");
    }

    #[test]
    fn test_verification_result() {
        let mut result = VerificationResult::new();

        result.add(ToolVerification {
            name: "git".to_string(),
            status: VerificationStatus::Match,
            locked_version: "2.45.0".to_string(),
            installed_version: Some("2.45.0".to_string()),
        });

        result.add(ToolVerification {
            name: "node".to_string(),
            status: VerificationStatus::VersionMismatch,
            locked_version: "20.10.0".to_string(),
            installed_version: Some("18.0.0".to_string()),
        });

        assert_eq!(result.matched, 1);
        assert_eq!(result.mismatched, 1);
        assert!(!result.all_match);
    }

    #[test]
    fn test_verification_status_display() {
        assert_eq!(VerificationStatus::Match.to_string(), "match");
        assert_eq!(
            VerificationStatus::VersionMismatch.to_string(),
            "version mismatch"
        );
        assert_eq!(
            VerificationStatus::NotInstalled.to_string(),
            "not installed"
        );
    }
}
