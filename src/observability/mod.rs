//! Observability & Debugging Module
//!
//! Provides comprehensive observability features for Jarvy including:
//! - Structured logging with multiple verbosity levels
//! - Performance profiling with phase tracking
//! - Network request tracing
//! - Sensitive data sanitization
//! - Diagnostic bundle export

pub mod bundle;
pub mod error;
pub mod logging;
pub mod network_trace;
pub mod profiler;
pub mod sanitizer;

pub use bundle::{BundleScope, DiagnosticBundle, SystemInfo as BundleSystemInfo};
pub use error::ObservabilityError;
pub use logging::{LogConfig, LogFormat, LogLevel, init_debug_logging};
pub use network_trace::{DomainStats, NetworkSummary, NetworkTiming, NetworkTracer};
pub use profiler::{PhaseTiming, ProfileReport, Profiler};
pub use sanitizer::Sanitizer;

/// Global observability configuration
#[derive(Debug, Clone, Default)]
pub struct ObservabilityConfig {
    /// Logging configuration
    pub log: LogConfig,
    /// Whether profiling is enabled
    pub profile: bool,
    /// Path to write profile output
    pub profile_output: Option<String>,
    /// Whether network tracing is enabled
    pub trace_network: bool,
    /// Path to write network trace
    pub network_log: Option<String>,
}

impl ObservabilityConfig {
    /// Create from CLI flags
    pub fn from_flags(
        quiet: bool,
        verbose: u8,
        log_format: Option<&str>,
        debug_filter: Option<&str>,
        log_file: Option<&str>,
        profile: bool,
        profile_output: Option<&str>,
        trace_network: bool,
        network_log: Option<&str>,
    ) -> Self {
        let level = if quiet {
            LogLevel::Quiet
        } else {
            match verbose {
                0 => LogLevel::Normal,
                1 => LogLevel::Verbose,
                2 => LogLevel::Debug,
                _ => LogLevel::Trace,
            }
        };

        let format = match log_format {
            Some("json") => LogFormat::Json,
            _ => LogFormat::Text,
        };

        Self {
            log: LogConfig {
                level,
                format,
                filter: debug_filter.map(|s| s.to_string()),
                file: log_file.map(|s| s.to_string()),
            },
            profile,
            profile_output: profile_output.map(|s| s.to_string()),
            trace_network,
            network_log: network_log.map(|s| s.to_string()),
        }
    }
}
