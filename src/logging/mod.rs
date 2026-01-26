//! Persistent file-based logging with rotation
//!
//! This module provides:
//! - File-based logging to ~/.jarvy/logs/
//! - Automatic log rotation by size and age
//! - Gzip compression of rotated logs
//! - Sensitive data sanitization
//! - Integration with tracing-subscriber

mod config;
mod formatter;
mod rotator;
mod sanitizer;
mod writer;

pub use config::{LogFormat, LogLevel, LoggingConfig};
pub use formatter::{LogEntry, LogFormatter};
pub use rotator::LogRotator;
pub use sanitizer::Sanitizer;
pub use writer::RotatingFileWriter;

use std::path::PathBuf;
use thiserror::Error;

/// Logging errors
#[derive(Debug, Error)]
pub enum LogError {
    #[error("Failed to create log directory: {0}")]
    DirectoryCreationFailed(#[from] std::io::Error),

    #[error("Failed to open log file: {0}")]
    FileOpenFailed(String),

    #[error("Failed to write to log file: {0}")]
    WriteFailed(String),

    #[error("Failed to rotate log file: {0}")]
    RotationFailed(String),

    #[error("Failed to compress log file: {0}")]
    CompressionFailed(String),

    #[error("Invalid log configuration: {0}")]
    InvalidConfig(String),
}

/// Get the default log directory path (~/.jarvy/logs/)
pub fn default_log_directory() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".jarvy")
        .join("logs")
}

/// Get the current log file path
pub fn current_log_file() -> PathBuf {
    default_log_directory().join("jarvy.log")
}

/// Initialize the logging system with the given configuration
///
/// This sets up file-based logging alongside console output.
pub fn init(config: &LoggingConfig) -> Result<(), LogError> {
    if !config.enabled {
        return Ok(());
    }

    // Ensure log directory exists
    let log_dir = config.directory.clone();
    std::fs::create_dir_all(&log_dir)?;

    // Run initial cleanup of old logs
    let rotator = LogRotator::new(config.clone());
    if let Err(e) = rotator.cleanup_old_logs() {
        tracing::warn!("Failed to cleanup old logs: {}", e);
    }

    Ok(())
}

/// Read recent log entries from the log file
///
/// Returns the last `lines` entries from the current log file.
pub fn read_recent_logs(lines: usize) -> Result<Vec<String>, LogError> {
    let log_file = current_log_file();

    if !log_file.exists() {
        return Ok(Vec::new());
    }

    let content =
        std::fs::read_to_string(&log_file).map_err(|e| LogError::FileOpenFailed(e.to_string()))?;

    let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let start = all_lines.len().saturating_sub(lines);

    Ok(all_lines[start..].to_vec())
}

/// Get statistics about log files
#[derive(Debug, serde::Serialize)]
pub struct LogStats {
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub current_file_size_bytes: u64,
    pub oldest_entry: Option<String>,
    pub newest_entry: Option<String>,
    pub entries_by_level: std::collections::HashMap<String, usize>,
}

/// Calculate log statistics
pub fn get_log_stats() -> Result<LogStats, LogError> {
    let log_dir = default_log_directory();
    let mut total_files = 0;
    let mut total_size: u64 = 0;
    let mut current_file_size: u64 = 0;

    if log_dir.exists() {
        for entry in (std::fs::read_dir(&log_dir)
            .map_err(|e| LogError::FileOpenFailed(e.to_string()))?)
        .flatten()
        {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = path.metadata() {
                    total_files += 1;
                    total_size += metadata.len();
                    if path.file_name().map(|n| n == "jarvy.log").unwrap_or(false) {
                        current_file_size = metadata.len();
                    }
                }
            }
        }
    }

    // Count entries by level from current log
    let mut entries_by_level = std::collections::HashMap::new();
    let mut oldest_entry = None;
    let mut newest_entry = None;

    let log_file = current_log_file();
    if log_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&log_file) {
            let lines: Vec<&str> = content.lines().collect();
            if !lines.is_empty() {
                oldest_entry = lines.first().map(|s| s.to_string());
                newest_entry = lines.last().map(|s| s.to_string());
            }

            for line in lines {
                // Try to parse log level from line
                if line.contains("\"level\":\"ERROR\"") || line.contains(" ERROR ") {
                    *entries_by_level.entry("ERROR".to_string()).or_insert(0) += 1;
                } else if line.contains("\"level\":\"WARN\"") || line.contains(" WARN ") {
                    *entries_by_level.entry("WARN".to_string()).or_insert(0) += 1;
                } else if line.contains("\"level\":\"INFO\"") || line.contains(" INFO ") {
                    *entries_by_level.entry("INFO".to_string()).or_insert(0) += 1;
                } else if line.contains("\"level\":\"DEBUG\"") || line.contains(" DEBUG ") {
                    *entries_by_level.entry("DEBUG".to_string()).or_insert(0) += 1;
                } else if line.contains("\"level\":\"TRACE\"") || line.contains(" TRACE ") {
                    *entries_by_level.entry("TRACE".to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    Ok(LogStats {
        total_files,
        total_size_bytes: total_size,
        current_file_size_bytes: current_file_size,
        oldest_entry,
        newest_entry,
        entries_by_level,
    })
}

/// Clean old log files based on configuration
pub fn clean_logs(config: &LoggingConfig, all: bool) -> Result<(usize, u64), LogError> {
    let log_dir = config.directory.clone();

    if !log_dir.exists() {
        return Ok((0, 0));
    }

    let mut removed_files = 0;
    let mut removed_bytes: u64 = 0;

    for entry in std::fs::read_dir(&log_dir)
        .map_err(|e| LogError::FileOpenFailed(e.to_string()))?
        .flatten()
    {
        let path = entry.path();
        if path.is_file() {
            let should_remove = if all {
                true
            } else {
                // Check age
                if let Ok(metadata) = path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let age = std::time::SystemTime::now()
                            .duration_since(modified)
                            .unwrap_or_default();
                        age.as_secs() > config.max_age_days as u64 * 24 * 60 * 60
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if should_remove {
                if let Ok(metadata) = path.metadata() {
                    removed_bytes += metadata.len();
                }
                if std::fs::remove_file(&path).is_ok() {
                    removed_files += 1;
                }
            }
        }
    }

    Ok((removed_files, removed_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_log_directory() {
        let dir = default_log_directory();
        assert!(dir.ends_with(".jarvy/logs"));
    }

    #[test]
    fn test_current_log_file() {
        let file = current_log_file();
        assert!(file.ends_with("jarvy.log"));
    }
}
