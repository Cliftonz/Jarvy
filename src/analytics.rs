// Telemetry OTLP endpoints are hardcoded at compile time for this CLI.
// Build-time env (set when running `cargo build`) can override the defaults:
// - Logs:   JARVY_OTLP_LOGS_ENDPOINT (preferred) or JARVY_OTLP_ENDPOINT
// If neither is set at build time, we default to the local Alloy instance
// running on port 4318 (HTTP/protobuf):
//   logs   -> http://localhost:4318/v1/logs

use std::env;
use tracing::Level;
use tracing::field::Visit;
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::{FilterFn, LevelFilter};
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::Registry;

// Layer that forwards ERROR events to PostHog
struct PosthogErrorLayer;

struct EventVisitor {
    fields: Vec<(String, String)>,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields
            .push((field.name().to_string(), format!("{:?}", value)));
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }
}

impl<S> Layer<S> for PosthogErrorLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if event.metadata().level() == &Level::ERROR {
            let mut visitor = EventVisitor { fields: Vec::new() };
            event.record(&mut visitor);

            // Prefer the `message` field if present
            let mut message = None;
            for (k, v) in &visitor.fields {
                if k == "message" {
                    message = Some(v.clone());
                    break;
                }
            }
            let msg = message.unwrap_or_else(|| {
                // Fallback: join k=v pairs
                let parts: Vec<String> = visitor
                    .fields
                    .into_iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                if parts.is_empty() {
                    "unknown error".to_string()
                } else {
                    parts.join(", ")
                }
            });

            // Send to PostHog (no-op if client disabled)
            let props = serde_json::Map::new();
            crate::posthog::capture_error("cli_error", &msg, props);
        }
    }
}

pub fn init_logging(enable_analytics: bool) {
    // Always log errors to stderr and forward to PostHog.
    let stdout_non_error = tracing_subscriber::fmt::layer()
        .with_filter(FilterFn::new(|meta| meta.level() < &Level::ERROR));

    let stderr_errors = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(LevelFilter::ERROR);

    // Only if analytics enabled, export errors to OTLP logs
    let otel_layer_opt = if enable_analytics {
        let logger_provider = build_otlp_logger_provider();
        let layer = opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(
            &logger_provider,
        )
        .with_filter(LevelFilter::ERROR); // export only errors to OTEL
        Some(layer)
    } else {
        None
    };

    let subscriber = Registry::default()
        .with(stdout_non_error)
        .with(stderr_errors)
        .with(PosthogErrorLayer)
        .with(otel_layer_opt);

    tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");
}

fn compile_time_otlp_logs_endpoint() -> &'static str {
    option_env!("JARVY_OTLP_LOGS_ENDPOINT")
        .or(option_env!("JARVY_OTLP_ENDPOINT"))
        .unwrap_or("http://localhost:4318/v1/logs")
}

fn build_otlp_logger_provider() -> opentelemetry_sdk::logs::SdkLoggerProvider {
    use opentelemetry_otlp::{Protocol, WithExportConfig};

    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(compile_time_otlp_logs_endpoint())
        .build()
        .expect("failed to build OTLP log exporter");

    let mut logger_builder = opentelemetry_sdk::logs::SdkLoggerProvider::builder();
    if env::var("JARVY_TELEMETRY_SMOKE").as_deref() == Ok("1") {
        logger_builder = logger_builder.with_simple_exporter(exporter);
    } else {
        logger_builder = logger_builder.with_batch_exporter(exporter);
    }
    logger_builder.build()
}
