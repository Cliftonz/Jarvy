//! CI-specific output formatting
//!
//! Provides provider-specific output formatting for log groups, warnings, and errors.

use super::CiProvider;
use std::io::{self, Write};

/// CI output helper for provider-specific formatting
#[derive(Debug, Clone)]
pub struct CiOutput {
    provider: CiProvider,
}

impl CiOutput {
    /// Creates a new CI output helper for the given provider
    pub fn new(provider: CiProvider) -> Self {
        Self { provider }
    }

    /// Returns the CI provider
    pub fn provider(&self) -> CiProvider {
        self.provider
    }

    /// Starts a collapsible log group
    pub fn group_start(&self, name: &str) {
        match self.provider {
            CiProvider::GitHubActions => {
                println!("::group::{}", name);
            }
            CiProvider::GitLabCi => {
                // GitLab CI uses section markers
                let section_id = name.to_lowercase().replace(' ', "_");
                println!(
                    "\x1b[0Ksection_start:{}:{}[collapsed=true]\r\x1b[0K{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    section_id,
                    name
                );
            }
            CiProvider::AzureDevOps => {
                println!("##[group]{}", name);
            }
            CiProvider::Buildkite => {
                println!("--- {}", name);
            }
            _ => {
                // Fallback for providers without group support
                println!("=== {} ===", name);
            }
        }
    }

    /// Ends a collapsible log group
    pub fn group_end(&self, name: &str) {
        match self.provider {
            CiProvider::GitHubActions => {
                println!("::endgroup::");
            }
            CiProvider::GitLabCi => {
                let section_id = name.to_lowercase().replace(' ', "_");
                println!(
                    "\x1b[0Ksection_end:{}:{}\r\x1b[0K",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    section_id
                );
            }
            CiProvider::AzureDevOps => {
                println!("##[endgroup]");
            }
            _ => {
                // No-op for providers without group support
            }
        }
    }

    /// Creates a group guard that automatically closes when dropped
    pub fn group(&self, name: &str) -> GroupGuard {
        self.group_start(name);
        GroupGuard {
            output: self.clone(),
            name: name.to_string(),
        }
    }

    /// Outputs a warning message with provider-specific formatting
    pub fn warning(&self, message: &str) {
        match self.provider {
            CiProvider::GitHubActions => {
                println!("::warning::{}", message);
            }
            CiProvider::AzureDevOps => {
                println!("##[warning]{}", message);
            }
            CiProvider::GitLabCi => {
                // GitLab doesn't have a special warning format, use ANSI colors
                println!("\x1b[33mWarning: {}\x1b[0m", message);
            }
            _ => {
                eprintln!("Warning: {}", message);
            }
        }
    }

    /// Outputs a warning message with file location
    pub fn warning_with_location(&self, message: &str, file: &str, line: Option<u32>) {
        match self.provider {
            CiProvider::GitHubActions => {
                if let Some(line) = line {
                    println!("::warning file={},line={}::{}", file, line, message);
                } else {
                    println!("::warning file={}::{}", file, message);
                }
            }
            CiProvider::AzureDevOps => {
                if let Some(line) = line {
                    println!(
                        "##vso[task.logissue type=warning;sourcepath={};linenumber={}]{}",
                        file, line, message
                    );
                } else {
                    println!(
                        "##vso[task.logissue type=warning;sourcepath={}]{}",
                        file, message
                    );
                }
            }
            _ => {
                if let Some(line) = line {
                    eprintln!("Warning: {}:{}: {}", file, line, message);
                } else {
                    eprintln!("Warning: {}: {}", file, message);
                }
            }
        }
    }

    /// Outputs an error message with provider-specific formatting
    pub fn error(&self, message: &str) {
        match self.provider {
            CiProvider::GitHubActions => {
                println!("::error::{}", message);
            }
            CiProvider::AzureDevOps => {
                println!("##[error]{}", message);
            }
            CiProvider::GitLabCi => {
                // GitLab doesn't have a special error format, use ANSI colors
                println!("\x1b[31mError: {}\x1b[0m", message);
            }
            _ => {
                eprintln!("Error: {}", message);
            }
        }
    }

    /// Outputs an error message with file location
    pub fn error_with_location(&self, message: &str, file: &str, line: Option<u32>) {
        match self.provider {
            CiProvider::GitHubActions => {
                if let Some(line) = line {
                    println!("::error file={},line={}::{}", file, line, message);
                } else {
                    println!("::error file={}::{}", file, message);
                }
            }
            CiProvider::AzureDevOps => {
                if let Some(line) = line {
                    println!(
                        "##vso[task.logissue type=error;sourcepath={};linenumber={}]{}",
                        file, line, message
                    );
                } else {
                    println!(
                        "##vso[task.logissue type=error;sourcepath={}]{}",
                        file, message
                    );
                }
            }
            _ => {
                if let Some(line) = line {
                    eprintln!("Error: {}:{}: {}", file, line, message);
                } else {
                    eprintln!("Error: {}: {}", file, message);
                }
            }
        }
    }

    /// Sets an output variable (only supported by some providers)
    pub fn set_output(&self, name: &str, value: &str) {
        match self.provider {
            CiProvider::GitHubActions => {
                // GitHub Actions uses GITHUB_OUTPUT file
                if let Ok(output_file) = std::env::var("GITHUB_OUTPUT") {
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&output_file)
                    {
                        let _ = writeln!(file, "{}={}", name, value);
                    }
                } else {
                    // Fallback to deprecated set-output command
                    println!("::set-output name={}::{}", name, value);
                }
            }
            CiProvider::AzureDevOps => {
                println!("##vso[task.setvariable variable={}]{}", name, value);
            }
            _ => {
                // No-op for providers without output variable support
            }
        }
    }

    /// Masks a value in logs (only supported by some providers)
    pub fn mask_value(&self, value: &str) {
        match self.provider {
            CiProvider::GitHubActions => {
                println!("::add-mask::{}", value);
            }
            CiProvider::AzureDevOps => {
                println!("##vso[task.setsecret]{}", value);
            }
            _ => {
                // No-op for providers without masking support
            }
        }
    }

    /// Outputs a debug message (only visible when debug logging is enabled)
    pub fn debug(&self, message: &str) {
        match self.provider {
            CiProvider::GitHubActions => {
                println!("::debug::{}", message);
            }
            CiProvider::AzureDevOps => {
                println!("##[debug]{}", message);
            }
            _ => {
                // No-op for providers without debug support
            }
        }
    }

    /// Outputs a notice/info message
    pub fn notice(&self, message: &str) {
        match self.provider {
            CiProvider::GitHubActions => {
                println!("::notice::{}", message);
            }
            _ => {
                println!("Note: {}", message);
            }
        }
    }

    /// Outputs plain text, flushing immediately
    pub fn print(&self, message: &str) {
        print!("{}", message);
        let _ = io::stdout().flush();
    }

    /// Outputs plain text with newline
    pub fn println(&self, message: &str) {
        println!("{}", message);
    }
}

/// RAII guard that closes a log group when dropped
pub struct GroupGuard {
    output: CiOutput,
    name: String,
}

impl Drop for GroupGuard {
    fn drop(&mut self) {
        self.output.group_end(&self.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_output_creation() {
        let output = CiOutput::new(CiProvider::GitHubActions);
        assert_eq!(output.provider(), CiProvider::GitHubActions);
    }

    #[test]
    fn test_group_guard_creation() {
        let output = CiOutput::new(CiProvider::Generic);
        let _guard = output.group("test group");
        // Guard will close when dropped
    }
}
