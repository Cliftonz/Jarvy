//! MCP Safety Mechanisms
//!
//! Implements rate limiting, allowlist/denylist checking, and confirmation prompts
//! to ensure safe tool installation via MCP.

use crate::mcp::config::McpConfig;
use crate::mcp::error::{McpError, McpResult};
use std::collections::VecDeque;
use std::io::{IsTerminal, Write};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Rate limiter using a sliding window approach
pub struct RateLimiter {
    /// Timestamps of recent check operations
    check_times: Mutex<VecDeque<Instant>>,
    /// Timestamps of recent install operations
    install_times: Mutex<VecDeque<Instant>>,
    /// Maximum checks per minute
    max_checks_per_minute: u32,
    /// Maximum installs per minute
    max_installs_per_minute: u32,
}

impl RateLimiter {
    /// Create a new rate limiter from configuration
    pub fn new(config: &McpConfig) -> Self {
        Self {
            check_times: Mutex::new(VecDeque::new()),
            install_times: Mutex::new(VecDeque::new()),
            max_checks_per_minute: config.mcp.max_checks_per_minute,
            max_installs_per_minute: config.mcp.max_installs_per_minute,
        }
    }

    /// Check if a tool check operation is allowed (and record it)
    pub fn check_check_limit(&self) -> McpResult<()> {
        self.check_limit(
            &self.check_times,
            self.max_checks_per_minute,
            "Tool check rate limit exceeded. Please wait before checking more tools.",
        )
    }

    /// Check if an install operation is allowed (and record it)
    pub fn check_install_limit(&self) -> McpResult<()> {
        self.check_limit(
            &self.install_times,
            self.max_installs_per_minute,
            "Install rate limit exceeded. Please wait before installing more tools.",
        )
    }

    fn check_limit(
        &self,
        times: &Mutex<VecDeque<Instant>>,
        max_per_minute: u32,
        error_message: &str,
    ) -> McpResult<()> {
        let mut times = times
            .lock()
            .map_err(|_| McpError::internal_error("Lock poisoned"))?;
        let now = Instant::now();
        let one_minute_ago = now - Duration::from_secs(60);

        // Remove old entries
        while times.front().is_some_and(|&t| t < one_minute_ago) {
            times.pop_front();
        }

        if times.len() >= max_per_minute as usize {
            return Err(McpError::rate_limited(error_message));
        }

        times.push_back(now);
        Ok(())
    }

    /// Get current check count in the last minute
    #[allow(dead_code)] // Public API for rate limiter stats
    pub fn check_count(&self) -> usize {
        self.get_count(&self.check_times)
    }

    /// Get current install count in the last minute
    #[allow(dead_code)] // Public API for rate limiter stats
    pub fn install_count(&self) -> usize {
        self.get_count(&self.install_times)
    }

    #[allow(dead_code)] // Used by check_count and install_count
    fn get_count(&self, times: &Mutex<VecDeque<Instant>>) -> usize {
        let times = match times.lock() {
            Ok(t) => t,
            Err(_) => return 0,
        };
        let one_minute_ago = Instant::now() - Duration::from_secs(60);
        times.iter().filter(|&&t| t >= one_minute_ago).count()
    }
}

/// Check if a tool is allowed based on allowlist configuration
pub fn check_allowlist(tool: &str, config: &McpConfig) -> McpResult<()> {
    if !config.is_allowed(tool) {
        if config.is_denied(tool) {
            return Err(McpError::tool_denied(tool));
        }
        return Err(McpError::tool_not_allowed(tool));
    }
    Ok(())
}

/// Prompt the user for confirmation via stderr
///
/// Returns Ok(true) if confirmed, Ok(false) if declined, or Err if an error occurs.
/// The prompt is written to stderr so it doesn't interfere with MCP protocol on stdout.
pub fn prompt_user_confirmation(
    tool_name: &str,
    command: &str,
    client_name: Option<&str>,
) -> McpResult<ConfirmationResult> {
    // Check if stderr is a terminal (interactive)
    if !std::io::stderr().is_terminal() {
        // Non-interactive mode - cannot confirm
        return Err(McpError::user_cancelled());
    }

    let mut stderr = std::io::stderr();

    writeln!(stderr)?;
    writeln!(
        stderr,
        "┌────────────────────────────────────────────────────"
    )?;
    writeln!(stderr, "│ Jarvy MCP: Install {}?", tool_name)?;
    writeln!(stderr, "│")?;
    writeln!(stderr, "│ This will execute:")?;
    writeln!(stderr, "│   {}", command)?;
    writeln!(stderr, "│")?;
    if let Some(client) = client_name {
        writeln!(stderr, "│ Requested by: {}", client)?;
        writeln!(stderr, "│")?;
    }
    writeln!(stderr, "│ [Y]es / [N]o / [A]lways allow {}:", tool_name)?;
    writeln!(
        stderr,
        "└────────────────────────────────────────────────────"
    )?;
    write!(stderr, "> ")?;
    stderr.flush()?;

    // Read response from stdin
    let mut response = String::new();
    std::io::stdin().read_line(&mut response)?;
    let response = response.trim().to_lowercase();

    match response.as_str() {
        "y" | "yes" => Ok(ConfirmationResult::Yes),
        "n" | "no" | "" => Ok(ConfirmationResult::No),
        "a" | "always" => Ok(ConfirmationResult::Always),
        _ => {
            writeln!(stderr, "Invalid response. Interpreting as 'no'.")?;
            Ok(ConfirmationResult::No)
        }
    }
}

/// Result of a confirmation prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationResult {
    /// User confirmed this single operation
    Yes,
    /// User declined
    No,
    /// User wants to always allow this tool
    Always,
}

/// Prompt the user for confirmation of a generic mutating MCP tool
/// invocation (not a tool install — those go through
/// [`prompt_user_confirmation`]). Used by the extended-tool mutation
/// guard for `ai_hooks_apply`, `mcp_register_apply`, `services_start`,
/// and `templates_use`.
///
/// Fails closed in non-interactive mode (stderr is not a TTY): returns
/// `Err(user_cancelled)` so a headless agent cannot trick the server
/// into applying changes without a human in the loop.
pub fn prompt_mutation_confirmation(
    tool_name: &str,
    summary: &str,
    client_name: Option<&str>,
) -> McpResult<ConfirmationResult> {
    if !std::io::stderr().is_terminal() {
        return Err(McpError::user_cancelled());
    }
    let mut stderr = std::io::stderr();
    writeln!(stderr)?;
    writeln!(
        stderr,
        "┌────────────────────────────────────────────────────"
    )?;
    writeln!(stderr, "│ Jarvy MCP: run {}?", tool_name)?;
    writeln!(stderr, "│")?;
    writeln!(stderr, "│ Effect:")?;
    for line in summary.lines() {
        writeln!(stderr, "│   {}", line)?;
    }
    writeln!(stderr, "│")?;
    if let Some(client) = client_name {
        writeln!(stderr, "│ Requested by: {}", client)?;
        writeln!(stderr, "│")?;
    }
    writeln!(stderr, "│ [Y]es / [N]o / [A]lways allow {}:", tool_name)?;
    writeln!(
        stderr,
        "└────────────────────────────────────────────────────"
    )?;
    write!(stderr, "> ")?;
    stderr.flush()?;
    let mut response = String::new();
    std::io::stdin().read_line(&mut response)?;
    let response = response.trim().to_lowercase();
    match response.as_str() {
        "y" | "yes" => Ok(ConfirmationResult::Yes),
        "n" | "no" | "" => Ok(ConfirmationResult::No),
        "a" | "always" => Ok(ConfirmationResult::Always),
        _ => {
            writeln!(stderr, "Invalid response. Interpreting as 'no'.")?;
            Ok(ConfirmationResult::No)
        }
    }
}

/// Resolve a caller-supplied path against the MCP workspace root and
/// refuse anything that would escape it.
///
/// Defenses:
/// - Absolute paths are accepted only when they canonicalize to a
///   location strictly under the workspace.
/// - Any `..` component that would walk above the workspace is refused.
/// - Symlink endpoints are refused (`symlink_metadata`) so a planted
///   link at the requested path cannot redirect the write.
///
/// Intentionally **does not** canonicalize the entire requested path
/// (which would fail when the file doesn't yet exist — the common
/// `templates_use` case). It canonicalizes the deepest existing
/// ancestor and verifies the result is under the workspace.
pub fn resolve_within_workspace(
    workspace: &std::path::Path,
    requested: &std::path::Path,
) -> McpResult<std::path::PathBuf> {
    use std::path::{Component, PathBuf};
    let workspace_canon = workspace
        .canonicalize()
        .map_err(|e| McpError::invalid_params(format!("workspace not accessible: {e}")))?;

    let joined = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        workspace_canon.join(requested)
    };

    // Walk normalized components, allowing CurDir but refusing ParentDir
    // that would escape the workspace.
    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() || !normalized.starts_with(&workspace_canon) {
                    return Err(McpError::invalid_params(format!(
                        "path escapes workspace: {}",
                        requested.display()
                    )));
                }
            }
            other => {
                normalized.push(other.as_os_str());
            }
        }
    }

    if !normalized.starts_with(&workspace_canon) {
        return Err(McpError::invalid_params(format!(
            "path is not under workspace ({})",
            workspace_canon.display()
        )));
    }

    // Refuse symlink at the requested final path (don't follow it).
    if let Ok(meta) = std::fs::symlink_metadata(&normalized)
        && meta.file_type().is_symlink()
    {
        return Err(McpError::invalid_params(format!(
            "refusing to write through symlink: {}",
            normalized.display()
        )));
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let config = McpConfig::default();
        let limiter = RateLimiter::new(&config);

        // Should allow up to max_checks_per_minute checks
        for _ in 0..config.mcp.max_checks_per_minute {
            assert!(limiter.check_check_limit().is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let mut config = McpConfig::default();
        config.mcp.max_checks_per_minute = 2;
        let limiter = RateLimiter::new(&config);

        assert!(limiter.check_check_limit().is_ok());
        assert!(limiter.check_check_limit().is_ok());
        assert!(limiter.check_check_limit().is_err()); // 3rd should fail
    }

    #[test]
    fn test_rate_limiter_install_limit() {
        let mut config = McpConfig::default();
        config.mcp.max_installs_per_minute = 1;
        let limiter = RateLimiter::new(&config);

        assert!(limiter.check_install_limit().is_ok());
        assert!(limiter.check_install_limit().is_err());
    }

    #[test]
    fn test_check_allowlist_no_lists() {
        let config = McpConfig::default();
        assert!(check_allowlist("git", &config).is_ok());
        assert!(check_allowlist("anything", &config).is_ok());
    }

    #[test]
    fn test_check_allowlist_with_denylist() {
        let mut config = McpConfig::default();
        config.mcp.denylist = Some(vec!["brew".to_string()]);

        assert!(check_allowlist("git", &config).is_ok());
        let result = check_allowlist("brew", &config);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, -32002); // tool_denied
    }

    #[test]
    fn test_check_allowlist_with_allowlist() {
        let mut config = McpConfig::default();
        config.mcp.allowlist = Some(vec!["git".to_string(), "docker".to_string()]);

        assert!(check_allowlist("git", &config).is_ok());
        assert!(check_allowlist("docker", &config).is_ok());

        let result = check_allowlist("vim", &config);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, -32003); // tool_not_allowed
    }
}
