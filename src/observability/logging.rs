//! Structured Logging Configuration
//!
//! Provides debug logging with multiple verbosity levels and output formats.
//!
//! ## Log Levels
//!
//! - `Quiet`: Errors only
//! - `Normal`: Info and above (default)
//! - `Verbose`: Includes warnings
//! - `Debug`: Full debug logs
//! - `Trace`: Trace-level detail
//!
//! ## Usage
//!
//! ```bash
//! jarvy setup --debug              # Debug logging
//! jarvy setup --trace              # Trace logging
//! jarvy setup --debug --log-format json   # JSON output
//! jarvy setup --debug --log-file debug.log
//! ```

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;

/// Log verbosity level
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LogLevel {
    /// Errors only (--quiet)
    Quiet,
    /// Info and above (default)
    #[default]
    Normal,
    /// Warnings included (--verbose / -v)
    Verbose,
    /// Full debug logs (--debug / -vv)
    Debug,
    /// Trace-level detail (--trace / -vvv)
    Trace,
}

impl LogLevel {
    /// Convert to tracing EnvFilter string
    pub fn to_filter_string(&self) -> &'static str {
        match self {
            LogLevel::Quiet => "error",
            LogLevel::Normal => "info",
            LogLevel::Verbose => "warn,jarvy=info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

/// Log output format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable text (default)
    #[default]
    Text,
    /// Machine-parseable JSON
    Json,
}

/// Logging configuration
#[derive(Debug, Clone, Default)]
pub struct LogConfig {
    /// Verbosity level
    pub level: LogLevel,
    /// Output format
    pub format: LogFormat,
    /// Module filter (e.g., "jarvy::tools::docker")
    pub filter: Option<String>,
    /// File to write logs to
    pub file: Option<String>,
}

/// File writer that implements Write
struct FileWriter {
    file: Mutex<std::fs::File>,
}

impl FileWriter {
    fn new(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }
}

impl Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut file = self.file.lock().unwrap();
        file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut file = self.file.lock().unwrap();
        file.flush()
    }
}

impl Write for &FileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut file = self.file.lock().unwrap();
        file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut file = self.file.lock().unwrap();
        file.flush()
    }
}

/// Initialize debug logging with the given configuration
///
/// This should be called early in main() when debug flags are present.
/// Returns Ok(true) if debug logging was enabled, Ok(false) if using default logging.
pub fn init_debug_logging(config: &LogConfig) -> Result<bool, Box<dyn std::error::Error>> {
    // Only initialize if we have non-default settings
    if config.level == LogLevel::Normal
        && config.format == LogFormat::Text
        && config.filter.is_none()
        && config.file.is_none()
    {
        return Ok(false);
    }

    // Build filter string
    let filter_str = if let Some(ref module_filter) = config.filter {
        // Apply module filter on top of level
        format!("{},{}", config.level.to_filter_string(), module_filter)
    } else {
        config.level.to_filter_string().to_string()
    };

    let env_filter = EnvFilter::try_new(&filter_str)
        .unwrap_or_else(|_| EnvFilter::new(config.level.to_filter_string()));

    // Build subscriber based on format and output
    match (config.format, &config.file) {
        (LogFormat::Json, None) => {
            let subscriber = tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .json()
                .with_span_events(FmtSpan::CLOSE)
                .with_current_span(true)
                .with_target(true)
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        (LogFormat::Json, Some(path)) => {
            let file = OpenOptions::new().create(true).append(true).open(path)?;
            let subscriber = tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .json()
                .with_span_events(FmtSpan::CLOSE)
                .with_current_span(true)
                .with_target(true)
                .with_writer(Mutex::new(file))
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        (LogFormat::Text, None) => {
            let subscriber = tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_target(true)
                .with_thread_ids(false)
                .with_file(true)
                .with_line_number(true)
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        (LogFormat::Text, Some(path)) => {
            let file = OpenOptions::new().create(true).append(true).open(path)?;
            let subscriber = tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_target(true)
                .with_thread_ids(false)
                .with_file(true)
                .with_line_number(true)
                .with_writer(Mutex::new(file))
                .with_ansi(false) // No colors in file output
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_to_filter() {
        assert_eq!(LogLevel::Quiet.to_filter_string(), "error");
        assert_eq!(LogLevel::Normal.to_filter_string(), "info");
        assert_eq!(LogLevel::Debug.to_filter_string(), "debug");
        assert_eq!(LogLevel::Trace.to_filter_string(), "trace");
    }

    #[test]
    fn test_log_config_default() {
        let config = LogConfig::default();
        assert_eq!(config.level, LogLevel::Normal);
        assert_eq!(config.format, LogFormat::Text);
        assert!(config.filter.is_none());
        assert!(config.file.is_none());
    }
}
