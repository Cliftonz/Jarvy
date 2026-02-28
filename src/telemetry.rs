//! Unified telemetry module for Jarvy
//!
//! This module provides a single API for all telemetry: logs, metrics, and traces.
//! It replaces both PostHog analytics and the limited OTEL setup in analytics.rs.
//!
//! ## Configuration
//!
//! Telemetry is opt-in and disabled by default. Configure via:
//! - `~/.jarvy/config.toml` [telemetry] section
//! - Environment variables (JARVY_TELEMETRY, JARVY_OTLP_ENDPOINT, etc.)
//!
//! ## Events
//!
//! Use the event functions to emit structured telemetry:
//! - `tool_requested()`, `tool_installed()`, `tool_failed()`, `tool_not_supported()`

#![allow(dead_code)] // Public API for telemetry - many functions reserved for future use
//! - `hook_started()`, `hook_completed()`, `hook_failed()`
//! - `command_executed()`, `setup_completed()`

use opentelemetry::KeyValue;
use opentelemetry::metrics::{Counter, Gauge, Histogram, MeterProvider};
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

// ============================================================================
// Configuration
// ============================================================================

/// Telemetry configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct TelemetryConfig {
    /// Master switch for telemetry (default: false)
    pub enabled: bool,
    /// OTLP endpoint URL (default: http://localhost:4318)
    pub endpoint: String,
    /// Protocol: "http" or "grpc" (default: "http")
    pub protocol: String,
    /// Enable log export (default: true when telemetry enabled)
    pub logs: bool,
    /// Enable metrics export (default: true when telemetry enabled)
    pub metrics: bool,
    /// Enable trace export (default: false)
    pub traces: bool,
    /// Trace sampling rate 0.0-1.0 (default: 1.0)
    pub sample_rate: f64,
    /// Custom resource attributes
    #[serde(default)]
    pub resource: HashMap<String, String>,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://localhost:4318".to_string(),
            protocol: "http".to_string(),
            logs: true,
            metrics: true,
            traces: false,
            sample_rate: 1.0,
            resource: HashMap::new(),
        }
    }
}

impl TelemetryConfig {
    /// Load config from environment variables, overriding defaults
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Master switch
        if let Ok(v) = env::var("JARVY_TELEMETRY") {
            config.enabled = matches!(v.as_str(), "1" | "true" | "yes");
        }

        // Endpoint
        if let Ok(v) = env::var("JARVY_OTLP_ENDPOINT") {
            if !v.trim().is_empty() {
                config.endpoint = v;
            }
        }

        // Protocol
        if let Ok(v) = env::var("JARVY_OTLP_PROTOCOL") {
            config.protocol = v;
        }

        // Signal toggles
        if let Ok(v) = env::var("JARVY_OTLP_LOGS") {
            config.logs = matches!(v.as_str(), "1" | "true" | "yes");
        }
        if let Ok(v) = env::var("JARVY_OTLP_METRICS") {
            config.metrics = matches!(v.as_str(), "1" | "true" | "yes");
        }
        if let Ok(v) = env::var("JARVY_OTLP_TRACES") {
            config.traces = matches!(v.as_str(), "1" | "true" | "yes");
        }

        // Sample rate
        if let Ok(v) = env::var("JARVY_OTLP_SAMPLE_RATE") {
            if let Ok(rate) = v.parse::<f64>() {
                config.sample_rate = rate.clamp(0.0, 1.0);
            }
        }

        // CI-aware: auto-disable in CI unless explicitly enabled
        if env::var("CI").is_ok() || env::var("GITHUB_ACTIONS").is_ok() {
            // If JARVY_TELEMETRY is not explicitly set, disable in CI
            if env::var("JARVY_TELEMETRY").is_err() {
                config.enabled = false;
            }
        }

        config
    }

    /// Check if telemetry is effectively enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled && (self.logs || self.metrics || self.traces)
    }
}

// ============================================================================
// Source enum for event tracking
// ============================================================================

/// Source of a tool request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// From jarvy.toml config file
    Config,
    /// From MCP server request
    Mcp,
    /// From CLI argument
    Cli,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Config => write!(f, "config"),
            Source::Mcp => write!(f, "mcp"),
            Source::Cli => write!(f, "cli"),
        }
    }
}

// ============================================================================
// Global State
// ============================================================================

static TELEMETRY: OnceLock<TelemetryState> = OnceLock::new();

struct TelemetryState {
    config: TelemetryConfig,
    meter_provider: Option<SdkMeterProvider>,
    metrics: Option<Metrics>,
}

struct Metrics {
    tool_requests: Counter<u64>,
    tool_installs: Counter<u64>,
    tool_not_supported: Counter<u64>,
    errors: Counter<u64>,
    hooks_executions: Counter<u64>,
    commands: Counter<u64>,
    install_duration: Histogram<f64>,
    setup_duration: Histogram<f64>,
    hooks_duration: Histogram<f64>,
    commands_duration: Histogram<f64>,
    setup_inventory_size: Gauge<u64>,
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize telemetry with the given configuration
pub fn init(config: TelemetryConfig) {
    let _ = TELEMETRY.set(build_telemetry_state(config));
}

/// Initialize telemetry from environment variables
pub fn init_from_env() {
    init(TelemetryConfig::from_env());
}

fn build_telemetry_state(config: TelemetryConfig) -> TelemetryState {
    if !config.is_enabled() {
        return TelemetryState {
            config,
            meter_provider: None,
            metrics: None,
        };
    }

    // Build meter provider for metrics
    let (meter_provider, metrics) = if config.metrics {
        match build_meter_provider(&config) {
            Ok(provider) => {
                let meter = provider.meter("jarvy");
                let metrics = Metrics {
                    tool_requests: meter
                        .u64_counter("jarvy.tool.requests")
                        .with_description("Number of tool installation requests")
                        .build(),
                    tool_installs: meter
                        .u64_counter("jarvy.tool.installs")
                        .with_description("Number of tool installations by status")
                        .build(),
                    tool_not_supported: meter
                        .u64_counter("jarvy.tool.not_supported")
                        .with_description("Number of unsupported tool requests")
                        .build(),
                    errors: meter
                        .u64_counter("jarvy.errors")
                        .with_description("Number of errors by type")
                        .build(),
                    hooks_executions: meter
                        .u64_counter("jarvy.hooks.executions")
                        .with_description("Number of hook executions by type and status")
                        .build(),
                    commands: meter
                        .u64_counter("jarvy.commands")
                        .with_description("Number of command executions")
                        .build(),
                    install_duration: meter
                        .f64_histogram("jarvy.install.duration")
                        .with_description("Tool installation duration in seconds")
                        .with_unit("s")
                        .build(),
                    setup_duration: meter
                        .f64_histogram("jarvy.setup.duration")
                        .with_description("Total setup duration in seconds")
                        .with_unit("s")
                        .build(),
                    hooks_duration: meter
                        .f64_histogram("jarvy.hooks.duration")
                        .with_description("Hook execution duration in seconds")
                        .with_unit("s")
                        .build(),
                    commands_duration: meter
                        .f64_histogram("jarvy.commands.duration")
                        .with_description("Command execution duration in seconds")
                        .with_unit("s")
                        .build(),
                    setup_inventory_size: meter
                        .u64_gauge("jarvy.setup.inventory_size")
                        .with_description(
                            "Number of tools in the provisioning inventory (security audit)",
                        )
                        .build(),
                };
                (Some(provider), Some(metrics))
            }
            Err(e) => {
                tracing::warn!("Failed to initialize metrics: {}", e);
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    TelemetryState {
        config,
        meter_provider,
        metrics,
    }
}

fn build_meter_provider(config: &TelemetryConfig) -> Result<SdkMeterProvider, String> {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(&config.endpoint)
        .build()
        .map_err(|e| format!("Failed to build metric exporter: {}", e))?;

    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(60))
        .build();

    Ok(SdkMeterProvider::builder().with_reader(reader).build())
}

/// Shutdown telemetry, flushing any pending data
pub fn shutdown() {
    if let Some(state) = TELEMETRY.get() {
        if let Some(ref provider) = state.meter_provider {
            let _ = provider.shutdown();
        }
    }
}

/// Check if telemetry is enabled
pub fn is_enabled() -> bool {
    TELEMETRY
        .get()
        .map(|s| s.config.is_enabled())
        .unwrap_or(false)
}

/// Get the current telemetry configuration
pub fn config() -> Option<&'static TelemetryConfig> {
    TELEMETRY.get().map(|s| &s.config)
}

// ============================================================================
// Event Functions - Tool Operations
// ============================================================================

/// Record a tool installation request
pub fn tool_requested(tool: &str, version: &str, source: Source) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "tool.requested",
        tool = %tool,
        version = %version,
        source = %source,
        platform = %env::consts::OS,
    );

    if let Some(state) = TELEMETRY.get() {
        if let Some(ref metrics) = state.metrics {
            metrics.tool_requests.add(
                1,
                &[
                    KeyValue::new("tool", tool.to_string()),
                    KeyValue::new("source", source.to_string()),
                    KeyValue::new("platform", env::consts::OS.to_string()),
                ],
            );
        }
    }
}

/// Record a successful tool installation
pub fn tool_installed(tool: &str, version: &str, package_manager: &str, duration: Duration) {
    if !is_enabled() {
        return;
    }

    let duration_ms = duration.as_millis() as u64;
    tracing::info!(
        event = "tool.installed",
        tool = %tool,
        version = %version,
        package_manager = %package_manager,
        duration_ms = %duration_ms,
        platform = %env::consts::OS,
    );

    if let Some(state) = TELEMETRY.get() {
        if let Some(ref metrics) = state.metrics {
            let attrs = [
                KeyValue::new("tool", tool.to_string()),
                KeyValue::new("pm", package_manager.to_string()),
                KeyValue::new("platform", env::consts::OS.to_string()),
                KeyValue::new("status", "success"),
            ];
            metrics.tool_installs.add(1, &attrs);
            metrics
                .install_duration
                .record(duration.as_secs_f64(), &attrs[..3]);
        }
    }
}

/// Record a failed tool installation
pub fn tool_failed(tool: &str, version: &str, error: &str) {
    if !is_enabled() {
        return;
    }

    // Redact potentially sensitive info from error
    let redacted_error = redact_sensitive(error);

    tracing::warn!(
        event = "tool.failed",
        tool = %tool,
        version = %version,
        error = %redacted_error,
        platform = %env::consts::OS,
    );

    if let Some(state) = TELEMETRY.get() {
        if let Some(ref metrics) = state.metrics {
            metrics.tool_installs.add(
                1,
                &[
                    KeyValue::new("tool", tool.to_string()),
                    KeyValue::new("platform", env::consts::OS.to_string()),
                    KeyValue::new("status", "failed"),
                ],
            );
            metrics
                .errors
                .add(1, &[KeyValue::new("error_type", "tool_install")]);
        }
    }
}

/// Record an unsupported tool request (critical for MCP feedback)
pub fn tool_not_supported(tool: &str, version: Option<&str>, source: Source) {
    if !is_enabled() {
        return;
    }

    tracing::warn!(
        event = "tool.not_supported",
        tool = %tool,
        version = %version.unwrap_or("*"),
        source = %source,
        platform = %env::consts::OS,
    );

    if let Some(state) = TELEMETRY.get() {
        if let Some(ref metrics) = state.metrics {
            metrics.tool_not_supported.add(
                1,
                &[
                    KeyValue::new("tool", tool.to_string()),
                    KeyValue::new("source", source.to_string()),
                    KeyValue::new("platform", env::consts::OS.to_string()),
                ],
            );
        }
    }
}

// ============================================================================
// Event Functions - Setup Flow
// ============================================================================

/// Summary of a setup operation
#[derive(Debug, Clone, Default)]
pub struct SetupSummary {
    pub tools_requested: usize,
    pub tools_installed: usize,
    pub tools_skipped: usize,
    pub tools_failed: usize,
    pub hooks_run: usize,
    pub duration: Duration,
}

/// Record setup started
pub fn setup_started(tools_count: usize) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "setup.started",
        tools_count = %tools_count,
        platform = %env::consts::OS,
    );
}

/// Record setup completed
pub fn setup_completed(summary: &SetupSummary) {
    if !is_enabled() {
        return;
    }

    let duration_ms = summary.duration.as_millis() as u64;
    tracing::info!(
        event = "setup.completed",
        tools_requested = %summary.tools_requested,
        tools_installed = %summary.tools_installed,
        tools_skipped = %summary.tools_skipped,
        tools_failed = %summary.tools_failed,
        hooks_run = %summary.hooks_run,
        duration_ms = %duration_ms,
    );

    if let Some(state) = TELEMETRY.get() {
        if let Some(ref metrics) = state.metrics {
            metrics.setup_duration.record(
                summary.duration.as_secs_f64(),
                &[KeyValue::new("tools_count", summary.tools_requested as i64)],
            );
        }
    }
}

/// Record the complete tool inventory being provisioned (for security audit).
///
/// Emits a single structured log event with the full manifest of tools, versions,
/// machine hardware ID, hostname, and platform so security teams can audit
/// provisioning across the fleet via their OTEL-connected observability platform.
pub fn setup_inventory(
    tools: &[(String, String)],
    role: Option<&str>,
    config_source: &str,
    machine_id: Option<&str>,
) {
    if !is_enabled() {
        return;
    }

    let tools_str = tools
        .iter()
        .map(|(name, version)| format!("{}={}", name, version))
        .collect::<Vec<_>>()
        .join(",");

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    tracing::info!(
        event = "setup.inventory",
        tools = %tools_str,
        tools_count = %tools.len(),
        role = %role.unwrap_or("none"),
        config_source = %redact_path(config_source),
        machine_id = %machine_id.unwrap_or("unknown"),
        hostname = %hostname,
        platform = %env::consts::OS,
    );

    if let Some(state) = TELEMETRY.get() {
        if let Some(ref metrics) = state.metrics {
            metrics.setup_inventory_size.record(
                tools.len() as u64,
                &[
                    KeyValue::new("machine_id", machine_id.unwrap_or("unknown").to_string()),
                    KeyValue::new("platform", env::consts::OS.to_string()),
                ],
            );
        }
    }
}

// ============================================================================
// Event Functions - Hooks
// ============================================================================

/// Record hook started
pub fn hook_started(hook_name: &str, hook_type: &str, tool: Option<&str>) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "hook.started",
        hook_name = %hook_name,
        hook_type = %hook_type,
        tool = %tool.unwrap_or("global"),
    );
}

/// Record hook completed successfully
pub fn hook_completed(hook_name: &str, hook_type: &str, duration: Duration, exit_code: i32) {
    if !is_enabled() {
        return;
    }

    let duration_ms = duration.as_millis() as u64;
    tracing::info!(
        event = "hook.completed",
        hook_name = %hook_name,
        hook_type = %hook_type,
        duration_ms = %duration_ms,
        exit_code = %exit_code,
    );

    if let Some(state) = TELEMETRY.get() {
        if let Some(ref metrics) = state.metrics {
            let attrs = [
                KeyValue::new("hook_type", hook_type.to_string()),
                KeyValue::new("status", "success"),
            ];
            metrics.hooks_executions.add(1, &attrs);
            metrics
                .hooks_duration
                .record(duration.as_secs_f64(), &[attrs[0].clone()]);
        }
    }
}

/// Record hook failed
pub fn hook_failed(hook_name: &str, hook_type: &str, error: &str, error_type: &str) {
    if !is_enabled() {
        return;
    }

    let redacted_error = redact_sensitive(error);
    tracing::warn!(
        event = "hook.failed",
        hook_name = %hook_name,
        hook_type = %hook_type,
        error = %redacted_error,
        error_type = %error_type,
    );

    if let Some(state) = TELEMETRY.get() {
        if let Some(ref metrics) = state.metrics {
            metrics.hooks_executions.add(
                1,
                &[
                    KeyValue::new("hook_type", hook_type.to_string()),
                    KeyValue::new("status", "failed"),
                ],
            );
            metrics
                .errors
                .add(1, &[KeyValue::new("error_type", "hook")]);
        }
    }
}

/// Record hook timeout
pub fn hook_timeout(hook_name: &str, hook_type: &str, timeout_secs: u64) {
    if !is_enabled() {
        return;
    }

    tracing::warn!(
        event = "hook.timeout",
        hook_name = %hook_name,
        hook_type = %hook_type,
        timeout_seconds = %timeout_secs,
    );

    if let Some(state) = TELEMETRY.get() {
        if let Some(ref metrics) = state.metrics {
            metrics.hooks_executions.add(
                1,
                &[
                    KeyValue::new("hook_type", hook_type.to_string()),
                    KeyValue::new("status", "timeout"),
                ],
            );
        }
    }
}

// ============================================================================
// Event Functions - Commands
// ============================================================================

/// Record command execution
pub fn command_executed(command: &str, duration: Duration, success: bool) {
    if !is_enabled() {
        return;
    }

    let duration_ms = duration.as_millis() as u64;
    let status = if success { "success" } else { "failed" };

    tracing::info!(
        event = "command.executed",
        command = %command,
        duration_ms = %duration_ms,
        status = %status,
    );

    if let Some(state) = TELEMETRY.get() {
        if let Some(ref metrics) = state.metrics {
            let attrs = [
                KeyValue::new("command", command.to_string()),
                KeyValue::new("status", status),
            ];
            metrics.commands.add(1, &attrs);
            metrics
                .commands_duration
                .record(duration.as_secs_f64(), &[attrs[0].clone()]);
        }
    }
}

/// Record doctor issue found
pub fn doctor_issue_found(category: &str, severity: &str, message: &str) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "doctor.issue_found",
        category = %category,
        severity = %severity,
        message = %message,
    );
}

/// Record search execution
pub fn search_executed(query: &str, results_count: usize) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "search.executed",
        query = %query,
        results_count = %results_count,
    );
}

/// Record validate result
pub fn validate_result(errors_count: usize, warnings_count: usize) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "validate.result",
        errors_count = %errors_count,
        warnings_count = %warnings_count,
    );
}

/// Record export completed
pub fn export_completed(tools_count: usize, format: &str) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "export.completed",
        tools_count = %tools_count,
        format = %format,
    );
}

/// Record diff executed
pub fn diff_executed(to_install: usize, to_update: usize, satisfied: usize, unknown: usize) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "diff.executed",
        to_install = %to_install,
        to_update = %to_update,
        satisfied = %satisfied,
        unknown = %unknown,
    );
}

/// Record upgrade result
pub fn upgrade_result(upgraded: usize, failed: usize, skipped: usize) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "upgrade.result",
        upgraded = %upgraded,
        failed = %failed,
        skipped = %skipped,
    );
}

/// Record doctor result
pub fn doctor_completed(issues_count: usize, tools_count: usize, exit_code: i32) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "doctor.completed",
        issues_count = %issues_count,
        tools_count = %tools_count,
        exit_code = %exit_code,
    );
}

// ============================================================================
// Event Functions - Config
// ============================================================================

/// Record config loaded
pub fn config_loaded(
    source: &str,
    tools_count: usize,
    has_hooks: bool,
    has_env: bool,
    has_services: bool,
) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "config.loaded",
        source = %source,
        tools_count = %tools_count,
        has_hooks = %has_hooks,
        has_env = %has_env,
        has_services = %has_services,
    );
}

/// Record config parse error
pub fn config_parse_error(file: &str, error: &str) {
    if !is_enabled() {
        return;
    }

    let redacted_file = redact_path(file);
    let redacted_error = redact_sensitive(error);

    tracing::warn!(
        event = "config.parse_error",
        file = %redacted_file,
        error = %redacted_error,
    );

    if let Some(state) = TELEMETRY.get() {
        if let Some(ref metrics) = state.metrics {
            metrics
                .errors
                .add(1, &[KeyValue::new("error_type", "config_parse")]);
        }
    }
}

// ============================================================================
// Event Functions - Services
// ============================================================================

/// Record service operation
pub fn service_operation(backend: &str, action: &str, success: bool) {
    if !is_enabled() {
        return;
    }

    let status = if success { "success" } else { "failed" };
    tracing::info!(
        event = "service.operation",
        backend = %backend,
        action = %action,
        status = %status,
    );
}

// ============================================================================
// Event Functions - Package Manager
// ============================================================================

/// Record package manager batch install
pub fn package_manager_batch_install(
    pm: &str,
    packages_count: usize,
    succeeded: usize,
    failed: usize,
    duration: Duration,
) {
    if !is_enabled() {
        return;
    }

    let duration_ms = duration.as_millis() as u64;
    tracing::info!(
        event = "package_manager.batch_install",
        pm = %pm,
        packages_count = %packages_count,
        succeeded = %succeeded,
        failed = %failed,
        duration_ms = %duration_ms,
    );
}

// ============================================================================
// Event Functions - CI
// ============================================================================

/// Record CI environment detected
pub fn ci_detected(provider: &str, build_id: Option<&str>, branch: Option<&str>) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "ci.detected",
        provider = %provider,
        build_id = %build_id.unwrap_or("unknown"),
        branch = %branch.unwrap_or("unknown"),
    );
}

// ============================================================================
// Event Functions - Environment
// ============================================================================

/// Record .env file generated
pub fn env_dotenv_generated(vars_count: usize, secrets_count: usize) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "env.dotenv_generated",
        vars_count = %vars_count,
        secrets_count = %secrets_count,
    );
}

/// Record shell rc updated
pub fn env_shell_rc_updated(shell: &str, vars_count: usize) {
    if !is_enabled() {
        return;
    }

    tracing::info!(
        event = "env.shell_rc_updated",
        shell = %shell,
        vars_count = %vars_count,
    );
}

// ============================================================================
// Tracing Spans (T8)
// ============================================================================

/// Create a span for setup operations
#[macro_export]
macro_rules! telemetry_span {
    ($name:expr) => {
        tracing::info_span!($name)
    };
    ($name:expr, $($field:tt)*) => {
        tracing::info_span!($name, $($field)*)
    };
}

/// Create a setup span
pub fn span_setup(tools_count: usize) -> tracing::Span {
    tracing::info_span!("jarvy.setup", tools_count = tools_count, platform = %env::consts::OS)
}

/// Create a version check span
pub fn span_version_check(tool: &str) -> tracing::Span {
    tracing::info_span!("jarvy.version_check", tool = %tool)
}

/// Create an install span
pub fn span_install(tool: &str, version: &str) -> tracing::Span {
    tracing::info_span!("jarvy.install", tool = %tool, version = %version)
}

/// Create a hook span
pub fn span_hook(hook_name: &str, hook_type: &str) -> tracing::Span {
    tracing::info_span!("jarvy.hook", hook_name = %hook_name, hook_type = %hook_type)
}

/// Create a command span
pub fn span_command(command: &str) -> tracing::Span {
    tracing::info_span!("jarvy.command", command = %command)
}

/// Create a service span
pub fn span_service(backend: &str, action: &str) -> tracing::Span {
    tracing::info_span!("jarvy.service", backend = %backend, action = %action)
}

// ============================================================================
// Privacy Helpers
// ============================================================================

/// Redact potentially sensitive information from strings
fn redact_sensitive(s: &str) -> String {
    // Redact file paths that look like home directories
    let result = regex::Regex::new(r"(/home/[^/\s]+|/Users/[^/\s]+|C:\\Users\\[^/\\\s]+)")
        .map(|re| re.replace_all(s, "[HOME]").to_string())
        .unwrap_or_else(|_| s.to_string());

    // Redact potential environment variable values
    regex::Regex::new(r"(API_KEY|TOKEN|SECRET|PASSWORD|CREDENTIAL)=\S+")
        .map(|re| re.replace_all(&result, "$1=[REDACTED]").to_string())
        .unwrap_or(result)
}

/// Redact file paths to remove user-identifying information
fn redact_path(path: &str) -> String {
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    if !home.is_empty() && path.starts_with(&home) {
        path.replace(&home, "~")
    } else {
        path.to_string()
    }
}

// ============================================================================
// Timing Helpers
// ============================================================================

/// Create a timestamp for timing operations
pub fn now() -> Instant {
    Instant::now()
}

/// Convert duration to milliseconds
pub fn ms(d: Duration) -> u128 {
    d.as_millis()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.endpoint, "http://localhost:4318");
        assert_eq!(config.protocol, "http");
        assert!(config.logs);
        assert!(config.metrics);
        assert!(!config.traces);
        assert_eq!(config.sample_rate, 1.0);
    }

    #[test]
    fn test_telemetry_config_is_enabled() {
        let mut config = TelemetryConfig::default();
        assert!(!config.is_enabled());

        config.enabled = true;
        assert!(config.is_enabled());

        config.logs = false;
        config.metrics = false;
        config.traces = false;
        assert!(!config.is_enabled()); // No signals enabled
    }

    #[test]
    fn test_source_display() {
        assert_eq!(Source::Config.to_string(), "config");
        assert_eq!(Source::Mcp.to_string(), "mcp");
        assert_eq!(Source::Cli.to_string(), "cli");
    }

    #[test]
    fn test_redact_sensitive() {
        let input = "Error at /home/user/project: API_KEY=secret123";
        let result = redact_sensitive(input);
        assert!(result.contains("[HOME]"));
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("user"));
        assert!(!result.contains("secret123"));
    }

    #[test]
    fn test_redact_path() {
        // This test is platform-dependent
        let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string());
        if let Some(home) = home {
            let path = format!("{}/some/path", home);
            let result = redact_path(&path);
            assert!(result.starts_with("~"));
            assert!(!result.contains(&home));
        }
    }

    #[test]
    fn test_setup_summary_default() {
        let summary = SetupSummary::default();
        assert_eq!(summary.tools_requested, 0);
        assert_eq!(summary.tools_installed, 0);
        assert_eq!(summary.tools_skipped, 0);
        assert_eq!(summary.tools_failed, 0);
        assert_eq!(summary.hooks_run, 0);
    }
}
