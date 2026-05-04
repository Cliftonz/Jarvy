//! Output formatting for CLI commands
//!
//! Provides traits and implementations for outputting command results in
//! different formats: human-readable, JSON, and quiet mode.

#![allow(dead_code)] // Public API for output formatting

use serde::Serialize;

/// Output format options for CLI commands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    /// Human-readable colored output (default)
    #[default]
    Human,
    /// Machine-readable JSON output
    Json,
    /// Minimal output, rely on exit codes
    Quiet,
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Human => write!(f, "human"),
            Format::Json => write!(f, "json"),
            Format::Quiet => write!(f, "quiet"),
        }
    }
}

impl std::str::FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "human" | "pretty" | "text" | "table" => Ok(Format::Human),
            "json" => Ok(Format::Json),
            "quiet" => Ok(Format::Quiet),
            _ => Err(format!(
                "Unknown format '{}'. Valid options: human, json, quiet",
                s
            )),
        }
    }
}

/// Render a command's `Outputable` result and return its exit code.
///
/// Single source of truth for the
/// `if json { to_json } else { to_human }; println!; exit_code` boilerplate
/// that was previously duplicated across every CLI handler. Centralizing
/// here also normalizes the `output_format` strings — `"pretty"`, `"json"`,
/// `"human"`, `"text"` etc. all flow through the same `Format::FromStr`.
pub fn print_and_exit<T: Outputable>(result: T, format_str: &str) -> i32 {
    let format = format_str.parse::<Format>().unwrap_or(Format::Human);
    let rendered = result.render(format);
    if !rendered.is_empty() {
        println!("{}", rendered);
    }
    result.exit_code().code()
}

/// Exit codes for commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Success
    Ok = 0,
    /// Warnings present
    Warning = 1,
    /// Errors present
    Error = 2,
}

impl ExitCode {
    pub fn code(self) -> i32 {
        self as i32
    }
}

/// Trait for command output that can be rendered in multiple formats
pub trait Outputable: Serialize {
    /// Render as human-readable output with optional colors
    fn to_human(&self) -> String;

    /// Render as JSON
    fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    /// Render for quiet mode (minimal or empty)
    fn to_quiet(&self) -> String {
        String::new()
    }

    /// Render based on format
    fn render(&self, format: Format) -> String {
        match format {
            Format::Human => self.to_human(),
            Format::Json => self.to_json(),
            Format::Quiet => self.to_quiet(),
        }
    }

    /// Get the exit code for this result
    fn exit_code(&self) -> ExitCode {
        ExitCode::Ok
    }
}

/// ANSI color codes for terminal output
pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const CYAN: &str = "\x1b[36m";
    pub const DIM: &str = "\x1b[2m";
}

/// Status icons for different states
pub mod icons {
    pub const OK: &str = "[OK]";
    pub const WARN: &str = "[WARN]";
    pub const ERROR: &str = "[ERROR]";
    pub const INFO: &str = "[INFO]";
    pub const INSTALL: &str = "+";
    pub const UPDATE: &str = "~";
    pub const SATISFIED: &str = "=";
    pub const HOOK: &str = "->";
}

/// Helper for printing colored status
pub fn status_line(icon: &str, color: &str, message: &str) -> String {
    format!("{}{}{} {}", color, icon, colors::RESET, message)
}

/// Print a section header
pub fn header(title: &str) -> String {
    format!(
        "{}{}{}\n{}",
        colors::BOLD,
        title,
        colors::RESET,
        "=".repeat(title.len())
    )
}

/// Print a sub-section header
pub fn subheader(title: &str) -> String {
    format!("\n{}{}{}:", colors::BOLD, title, colors::RESET)
}
