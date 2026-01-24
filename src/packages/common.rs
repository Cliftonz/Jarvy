//! Common utilities for package handlers
//!
//! Provides shared functionality for running package manager commands
//! and handling errors across different package ecosystems.

use std::path::Path;
use std::process::Command;
use thiserror::Error;

/// Errors that can occur during package installation
#[derive(Debug, Error)]
pub enum PackageError {
    #[error("Package manager not installed: {0}")]
    PackageManagerNotInstalled(String),

    #[error("Lock file not found: {0}")]
    LockfileNotFound(String),

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Virtual environment creation failed: {0}")]
    VenvCreationFailed(String),

    #[error("Package installation failed: {0}")]
    #[allow(dead_code)] // Reserved for future use
    InstallFailed(String),
}

/// Run a package manager command with the given arguments
pub fn run_package_command(
    cmd: &str,
    args: &[&str],
    working_dir: &Path,
) -> Result<(), PackageError> {
    let display_cmd = format!("{} {}", cmd, args.join(" "));
    println!("    Running: {}", display_cmd);

    let status = Command::new(cmd)
        .args(args)
        .current_dir(working_dir)
        .status()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                PackageError::PackageManagerNotInstalled(cmd.to_string())
            } else {
                PackageError::Io(e)
            }
        })?;

    if !status.success() {
        return Err(PackageError::CommandFailed(format!(
            "'{}' exited with status {}",
            display_cmd,
            status.code().unwrap_or(-1)
        )));
    }

    Ok(())
}

/// Check if a command is available in PATH
pub fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_exists() {
        // These commands should exist on most systems
        assert!(command_exists("echo") || command_exists("cmd"));
    }

    #[test]
    fn test_command_not_exists() {
        assert!(!command_exists(
            "this_command_definitely_does_not_exist_12345"
        ));
    }
}
