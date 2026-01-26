//! Logging configuration types
//!
//! Configuration for the file-based logging system.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Log level configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Only errors
    Error,
    /// Errors and warnings
    Warn,
    /// Errors, warnings, and info (default)
    #[default]
    Info,
    /// All above plus debug
    Debug,
    /// Everything including trace
    Trace,
}

impl LogLevel {
    /// Convert to tracing filter string
    pub fn as_filter(&self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => write!(f, "error"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Trace => write!(f, "trace"),
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(LogLevel::Error),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

/// Log output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Human-readable text format
    Text,
    /// JSON format for machine parsing
    #[default]
    Json,
}

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogFormat::Text => write!(f, "text"),
            LogFormat::Json => write!(f, "json"),
        }
    }
}

impl std::str::FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(LogFormat::Text),
            "json" => Ok(LogFormat::Json),
            _ => Err(format!("Invalid log format: {}", s)),
        }
    }
}

/// Logging configuration
///
/// Stored in ~/.jarvy/config.toml under [logging]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    /// Enable file-based logging
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Log level (error, warn, info, debug, trace)
    #[serde(default)]
    pub level: LogLevel,

    /// Log directory path
    #[serde(default = "default_directory")]
    pub directory: PathBuf,

    /// Log format (text, json)
    #[serde(default)]
    pub format: LogFormat,

    /// Maximum size per log file before rotation (in bytes)
    /// Default: 10MB (10 * 1024 * 1024)
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,

    /// Maximum number of rotated files to keep
    #[serde(default = "default_max_files")]
    pub max_files: usize,

    /// Maximum total log storage (in bytes)
    /// Default: 50MB (50 * 1024 * 1024)
    #[serde(default = "default_max_total_size")]
    pub max_total_size: u64,

    /// Maximum age of log files (in days)
    #[serde(default = "default_max_age_days")]
    pub max_age_days: u32,
}

fn default_enabled() -> bool {
    true
}

fn default_directory() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".jarvy")
        .join("logs")
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024 // 10MB
}

fn default_max_files() -> usize {
    5
}

fn default_max_total_size() -> u64 {
    50 * 1024 * 1024 // 50MB
}

fn default_max_age_days() -> u32 {
    30
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            level: LogLevel::default(),
            directory: default_directory(),
            format: LogFormat::default(),
            max_file_size: default_max_file_size(),
            max_files: default_max_files(),
            max_total_size: default_max_total_size(),
            max_age_days: default_max_age_days(),
        }
    }
}

impl LoggingConfig {
    /// Parse a human-readable size string (e.g., "10MB", "1GB")
    pub fn parse_size(s: &str) -> Result<u64, String> {
        let s = s.trim().to_uppercase();

        if let Some(num) = s.strip_suffix("GB") {
            num.trim()
                .parse::<u64>()
                .map(|n| n * 1024 * 1024 * 1024)
                .map_err(|_| format!("Invalid size: {}", s))
        } else if let Some(num) = s.strip_suffix("MB") {
            num.trim()
                .parse::<u64>()
                .map(|n| n * 1024 * 1024)
                .map_err(|_| format!("Invalid size: {}", s))
        } else if let Some(num) = s.strip_suffix("KB") {
            num.trim()
                .parse::<u64>()
                .map(|n| n * 1024)
                .map_err(|_| format!("Invalid size: {}", s))
        } else if let Some(num) = s.strip_suffix('B') {
            num.trim()
                .parse::<u64>()
                .map_err(|_| format!("Invalid size: {}", s))
        } else {
            s.parse::<u64>().map_err(|_| format!("Invalid size: {}", s))
        }
    }

    /// Format a size in bytes to human-readable string
    pub fn format_size(bytes: u64) -> String {
        if bytes >= 1024 * 1024 * 1024 {
            format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if bytes >= 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes >= 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} B", bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_default() {
        assert_eq!(LogLevel::default(), LogLevel::Info);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
        assert!("invalid".parse::<LogLevel>().is_err());
    }

    #[test]
    fn test_log_format_default() {
        assert_eq!(LogFormat::default(), LogFormat::Json);
    }

    #[test]
    fn test_log_format_from_str() {
        assert_eq!("text".parse::<LogFormat>().unwrap(), LogFormat::Text);
        assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert!("xml".parse::<LogFormat>().is_err());
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.level, LogLevel::Info);
        assert_eq!(config.format, LogFormat::Json);
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert_eq!(config.max_files, 5);
        assert_eq!(config.max_total_size, 50 * 1024 * 1024);
        assert_eq!(config.max_age_days, 30);
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(LoggingConfig::parse_size("10MB").unwrap(), 10 * 1024 * 1024);
        assert_eq!(
            LoggingConfig::parse_size("1GB").unwrap(),
            1024 * 1024 * 1024
        );
        assert_eq!(LoggingConfig::parse_size("512KB").unwrap(), 512 * 1024);
        assert_eq!(LoggingConfig::parse_size("1024B").unwrap(), 1024);
        assert_eq!(LoggingConfig::parse_size("1024").unwrap(), 1024);
        assert!(LoggingConfig::parse_size("invalid").is_err());
    }

    #[test]
    fn test_format_size() {
        assert_eq!(LoggingConfig::format_size(1024), "1.0 KB");
        assert_eq!(LoggingConfig::format_size(1024 * 1024), "1.0 MB");
        assert_eq!(LoggingConfig::format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(LoggingConfig::format_size(500), "500 B");
    }

    #[test]
    fn test_toml_parsing() {
        let toml_str = r#"
enabled = true
level = "debug"
format = "text"
max_file_size = 5242880
max_files = 3
max_age_days = 14
"#;
        let config: LoggingConfig = toml::from_str(toml_str).unwrap();
        assert!(config.enabled);
        assert_eq!(config.level, LogLevel::Debug);
        assert_eq!(config.format, LogFormat::Text);
        assert_eq!(config.max_file_size, 5 * 1024 * 1024);
        assert_eq!(config.max_files, 3);
        assert_eq!(config.max_age_days, 14);
    }
}
