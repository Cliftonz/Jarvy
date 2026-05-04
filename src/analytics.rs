// Telemetry OTLP endpoints are hardcoded at compile time for this CLI.
// Build-time env (set when running `cargo build`) can override the defaults:
// - Logs:   JARVY_OTLP_LOGS_ENDPOINT (preferred) or JARVY_OTLP_ENDPOINT
// If neither is set at build time, we default to the local Alloy instance
// running on port 4318 (HTTP/protobuf). Note: opentelemetry_otlp expects a base URL
// and will append the signal path (e.g., /v1/logs) automatically.
//   base   -> http://localhost:4318

use std::env;
use std::io::Write;
use tracing::Level;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::{FilterFn, LevelFilter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::Registry;

/// Records the runtime state of OTEL telemetry initialization. Read by
/// `jarvy telemetry status` so users can see whether their OTEL endpoint
/// actually came up — previously a misconfigured endpoint produced one
/// stderr line at startup with no further signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryBootstrapState {
    /// OTLP exporter active.
    Healthy,
    /// User explicitly disabled telemetry.
    Disabled,
    /// User enabled telemetry but exporter setup failed.
    Degraded,
}

static BOOTSTRAP_STATE: std::sync::OnceLock<std::sync::RwLock<TelemetryBootstrapState>> =
    std::sync::OnceLock::new();

fn bootstrap_state_cell() -> &'static std::sync::RwLock<TelemetryBootstrapState> {
    BOOTSTRAP_STATE.get_or_init(|| std::sync::RwLock::new(TelemetryBootstrapState::Disabled))
}

pub fn telemetry_bootstrap_state() -> TelemetryBootstrapState {
    bootstrap_state_cell()
        .read()
        .map(|g| *g)
        .unwrap_or(TelemetryBootstrapState::Degraded)
}

fn set_bootstrap_state(state: TelemetryBootstrapState) {
    if let Ok(mut g) = bootstrap_state_cell().write() {
        *g = state;
    }
}

pub fn init_logging(enable_analytics: bool) {
    // Always log to console: stdout for non-errors, stderr for errors
    let stdout_non_error = tracing_subscriber::fmt::layer()
        .with_filter(FilterFn::new(|meta| meta.level() < &Level::ERROR));

    let stderr_errors = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(LevelFilter::ERROR);

    // Only if analytics enabled, export errors to OTLP logs
    let mut bootstrap_error: Option<String> = None;
    let otel_layer_opt = if enable_analytics {
        match build_otlp_logger_provider() {
            Ok(logger_provider) => {
                let layer = opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(
                    &logger_provider,
                )
                .with_filter(LevelFilter::ERROR); // export only errors to OTEL
                Some(layer)
            }
            Err(e) => {
                // No subscriber yet — eprintln! is the only channel until the
                // fallback subscriber is up. After that, we emit a structured
                // event so the degradation is visible in any downstream sink
                // and in `jarvy telemetry status`.
                eprintln!("Warning: failed to initialize OTLP telemetry: {e}");
                bootstrap_error = Some(e.to_string());
                None
            }
        }
    } else {
        None
    };

    let subscriber = Registry::default()
        .with(stdout_non_error)
        .with(stderr_errors)
        .with(otel_layer_opt);

    let install_result = tracing::subscriber::set_global_default(subscriber);
    if let Err(e) = install_result {
        eprintln!("Failed to set tracing default: {e}");
    }

    if !enable_analytics {
        set_bootstrap_state(TelemetryBootstrapState::Disabled);
    } else if let Some(reason) = bootstrap_error {
        set_bootstrap_state(TelemetryBootstrapState::Degraded);
        // Subscriber is now installed (without OTEL layer); this event reaches
        // the fallback console layer and any downstream consumer.
        tracing::error!(
            event = "telemetry.bootstrap.degraded",
            reason = %reason,
            "OTLP exporter failed to initialize; running with console logs only"
        );
    } else {
        set_bootstrap_state(TelemetryBootstrapState::Healthy);
    }
}

fn otlp_logs_endpoint() -> String {
    if let Ok(v) = env::var("JARVY_OTLP_LOGS_ENDPOINT") {
        if !v.trim().is_empty() {
            return v;
        }
    }
    if let Ok(v) = env::var("JARVY_OTLP_ENDPOINT") {
        if !v.trim().is_empty() {
            return v;
        }
    }
    // Fallback to compile-time overrides or default (base URL; path is appended by exporter)
    option_env!("JARVY_OTLP_LOGS_ENDPOINT")
        .or(option_env!("JARVY_OTLP_ENDPOINT"))
        .unwrap_or("http://localhost:4318")
        .to_string()
}

pub fn send_otlp_smoke_probe() {
    if env::var("JARVY_TELEMETRY_SMOKE").as_deref() != Ok("1") {
        return;
    }
    // Best-effort: try IPv4 then IPv6. Ignore errors; this is just a smoke trigger.
    let req = b"POST /v1/logs HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    // IPv4
    if let Ok(mut s) = std::net::TcpStream::connect(("127.0.0.1", 4318)) {
        let _ = s.write_all(req);
        let _ = s.flush();
        return;
    }
    // IPv6
    if let Ok(mut s) = std::net::TcpStream::connect(("::1", 4318)) {
        let _ = s.write_all(req);
        let _ = s.flush();
    }
}

fn build_otlp_logger_provider()
-> Result<opentelemetry_sdk::logs::SdkLoggerProvider, Box<dyn std::error::Error>> {
    use opentelemetry_otlp::{Protocol, WithExportConfig};

    let endpoint = otlp_logs_endpoint();
    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(endpoint.as_str())
        .build()?;

    let mut logger_builder = opentelemetry_sdk::logs::SdkLoggerProvider::builder();
    if env::var("JARVY_TELEMETRY_SMOKE").as_deref() == Ok("1") {
        logger_builder = logger_builder.with_simple_exporter(exporter);
    } else {
        logger_builder = logger_builder.with_batch_exporter(exporter);
    }
    Ok(logger_builder.build())
}
