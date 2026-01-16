# PRD-022: Remote Telemetry and Monitoring

## Status: Draft
## Priority: P1
## Effort: Medium (3-5 days)

---

## Problem Statement

Jarvy currently has fragmented telemetry: PostHog for product analytics and limited OTEL for ERROR-level logs only. This creates:

1. **Two data silos** - operational and product data in different systems
2. **Vendor lock-in** - users can't route telemetry to their own backends
3. **Missing metrics** - no counters, histograms, or gauges for monitoring
4. **No MCP feedback loop** - when an LLM requests an unsupported tool, there's no structured way to report this

This PRD proposes **consolidating to OTEL-only telemetry**, removing the PostHog dependency entirely.

---

## Current State

### PostHog (`src/posthog.rs`) - TO BE REMOVED
- Product analytics events sent to PostHog cloud
- Requires API key configuration
- Vendor-locked data

### OTEL (`src/analytics.rs`) - TO BE EXPANDED
- ERROR-level logs only exported to OTLP endpoint
- Configurable via `JARVY_OTLP_ENDPOINT`
- No metrics or traces

### Problems
| Issue | Impact |
|-------|--------|
| Two telemetry systems | Maintenance burden, code complexity |
| PostHog vendor lock-in | Users can't use their own observability stack |
| No metrics | Can't monitor installation rates, performance |
| No traces | Can't debug complex failures |
| Errors only | Missing INFO/WARN for operational visibility |

---

## Proposed Solution: OTEL-Only

Remove PostHog entirely. Use OpenTelemetry for all telemetry:

```
┌─────────┐     ┌─────────────────┐     ┌────────────────────────┐
│ Jarvy   │────▶│ OTEL Collector  │────▶│ User's chosen backend  │
│   CLI   │     │ (or direct)     │     │ - Grafana Cloud        │
└─────────┘     └─────────────────┘     │ - Datadog              │
                                        │ - Honeycomb            │
     Logs ──────────────────────────────│ - Self-hosted Grafana  │
     Metrics ───────────────────────────│ - Jaeger               │
     Traces ────────────────────────────└────────────────────────┘
```

### Benefits
1. **Single telemetry path** - simpler code, one dependency
2. **User choice** - enterprises route to their own stack
3. **No vendor lock-in** - OTEL is an open standard
4. **Cost control** - users pay their backend, not our PostHog bill
5. **Better primitives** - native metrics, traces, structured logs

---

## Telemetry Schema

### 1. Structured Log Events

All events use semantic conventions with consistent attributes:

```rust
// Tool installation requested
tracing::info!(
    event = "tool.requested",
    tool = %name,
    version = %version,
    source = %source,  // "config" | "mcp" | "cli"
    platform = %std::env::consts::OS,
);

// Tool installation succeeded
tracing::info!(
    event = "tool.installed",
    tool = %name,
    version = %version,
    package_manager = %pm,
    duration_ms = %duration,
);

// Tool installation failed
tracing::warn!(
    event = "tool.failed",
    tool = %name,
    version = %version,
    error = %err,
    error_code = %code,
);

// Unsupported tool requested (critical for MCP feedback)
tracing::warn!(
    event = "tool.not_supported",
    tool = %name,
    version = %version,
    source = %source,
    platform = %std::env::consts::OS,
);

// Setup session completed
tracing::info!(
    event = "setup.completed",
    tools_requested = %total,
    tools_installed = %installed,
    tools_skipped = %skipped,
    tools_failed = %failed,
    duration_ms = %duration,
);
```

### 2. Metrics

```rust
// Counters (monotonic)
jarvy.tool.requests{tool, source, platform}           // Total tool requests
jarvy.tool.installs{tool, pm, platform, status}       // Installs by outcome
jarvy.tool.not_supported{tool, source, platform}      // Unsupported requests
jarvy.errors{error_type, command}                     // Error counts

// Histograms (distributions)
jarvy.install.duration{tool, pm}                      // Seconds per install
jarvy.setup.duration{tools_count}                     // Total setup time
jarvy.version_check.duration{}                        // Version check time

// Gauges (current values)
jarvy.tools.available{platform}                       // Supported tool count
```

### 3. Traces (Optional)

For debugging complex installations:

```
jarvy.setup [trace_id: abc123]
├── version_check [span_id: 001]
│   ├── check: git (2ms)
│   ├── check: node (3ms)
│   └── check: rust (5ms)
├── install: node [span_id: 002]
│   ├── brew install node (45s)
│   └── post_hook: nvm setup (2s)
└── summary [span_id: 003]
```

---

## Configuration

### Config File (`~/.jarvy/config.toml`)

```toml
[telemetry]
# Master switch (also respects JARVY_TELEMETRY=0)
enabled = true

# OTLP endpoint (HTTP by default)
endpoint = "http://localhost:4318"
# protocol = "http"  # or "grpc"

# What signals to export
logs = true
metrics = true
traces = false  # opt-in, more verbose

# Sampling for high-volume deployments
sample_rate = 1.0  # 0.0-1.0, applies to traces

# Resource attributes (added to all telemetry)
[telemetry.resource]
service.name = "jarvy"
# deployment.environment = "production"
```

### Environment Variables

```bash
# Master switches
JARVY_TELEMETRY=0              # Disable all telemetry
JARVY_TELEMETRY=1              # Enable telemetry

# Endpoint configuration
JARVY_OTLP_ENDPOINT=http://collector:4318
JARVY_OTLP_PROTOCOL=grpc       # "http" or "grpc"

# Signal toggles
JARVY_OTLP_LOGS=1
JARVY_OTLP_METRICS=1
JARVY_OTLP_TRACES=1

# Sampling
JARVY_OTLP_SAMPLE_RATE=0.1     # 10% of traces
```

### Precedence

1. Environment variables (highest)
2. Config file (`~/.jarvy/config.toml`)
3. Defaults (telemetry disabled until configured)

---

## CLI Commands

### `jarvy telemetry`

```bash
# Show current telemetry configuration
$ jarvy telemetry status
Telemetry: enabled
Endpoint:  http://localhost:4318 (HTTP)
Signals:   logs=on, metrics=on, traces=off
Sample:    100%

# Enable telemetry interactively
$ jarvy telemetry enable
Telemetry enabled. Configure endpoint:
  jarvy telemetry set-endpoint <url>

# Disable telemetry
$ jarvy telemetry disable
Telemetry disabled.

# Set OTLP endpoint
$ jarvy telemetry set-endpoint http://otel-collector:4318
Endpoint set to: http://otel-collector:4318

# Test connectivity (sends test event)
$ jarvy telemetry test
Sending test event to http://localhost:4318...
✓ Connection successful (response: 200 OK)

# Show what would be sent for a dry-run
$ jarvy telemetry preview
Would send on next setup:
  - tool.requested (per tool in config)
  - tool.installed | tool.failed (per tool)
  - setup.completed (summary)
  - Metrics: jarvy.tool.requests, jarvy.install.duration
```

---

## Implementation Plan

### Phase 1: OTEL Foundation (P0) - 2 days

**Goal**: Replace PostHog with OTEL logs + metrics

1. Create unified `src/telemetry.rs` module
2. Add OTEL metrics exporter (already have logs)
3. Define metric instruments (counters, histograms)
4. Deprecate `src/posthog.rs` (keep but mark deprecated)

**Files**:
- `src/telemetry.rs` (new - unified telemetry API)
- `src/analytics.rs` (refactor to use telemetry.rs)
- `Cargo.toml` (ensure opentelemetry features)

```rust
// src/telemetry.rs - Core API
pub fn init(config: &TelemetryConfig) -> Result<(), Error>;
pub fn tool_requested(tool: &str, version: &str, source: Source);
pub fn tool_installed(tool: &str, version: &str, pm: &str, duration: Duration);
pub fn tool_failed(tool: &str, version: &str, error: &str);
pub fn tool_not_supported(tool: &str, version: Option<&str>, source: Source);
pub fn setup_completed(summary: &SetupSummary);
pub fn shutdown();  // Flush and close exporters
```

### Phase 2: Structured Events (P0) - 1 day

**Goal**: Instrument all tool operations

1. Update `setup` command to emit telemetry events
2. Add `Source` enum tracking (Config, Mcp, Cli)
3. Emit `tool.not_supported` for unknown tools
4. Add timing instrumentation

**Files**:
- `src/main.rs` (setup command instrumentation)
- `src/tools/spec.rs` (add telemetry hooks to ensure())

### Phase 3: Configuration & CLI (P1) - 1 day

**Goal**: User-configurable telemetry

1. Extend config schema for telemetry settings
2. Add `jarvy telemetry` subcommand
3. Implement endpoint connectivity test
4. Add sampling support

**Files**:
- `src/config.rs` (TelemetryConfig struct)
- `src/main.rs` (telemetry subcommand)

### Phase 4: Remove PostHog (P1) - 0.5 days

**Goal**: Clean up old code

1. Remove `src/posthog.rs`
2. Remove `ureq` dependency (if only used by PostHog)
3. Update all call sites to use `telemetry.rs`
4. Remove PostHog env vars from docs

**Files**:
- Delete `src/posthog.rs`
- `src/main.rs` (remove posthog::init)
- `Cargo.toml` (remove unused deps)

### Phase 5: Traces (P2) - 1 day

**Goal**: Optional distributed tracing

1. Add tracing spans to setup flow
2. Instrument package manager calls
3. Add trace context propagation
4. Document trace visualization

**Files**:
- `src/telemetry.rs` (add span helpers)
- `src/tools/spec.rs` (instrument install methods)

---

## Migration Guide

### For Existing Users

If you were using PostHog telemetry:

```bash
# Old (PostHog)
export JARVY_POSTHOG_API_KEY=phc_xxx
export JARVY_ANALYTICS=1

# New (OTEL)
export JARVY_TELEMETRY=1
export JARVY_OTLP_ENDPOINT=http://your-collector:4318
```

### For Self-Hosted Observability

Recommended stack (all open source):

```yaml
# docker-compose.yml
services:
  otel-collector:
    image: otel/opentelemetry-collector-contrib:latest
    ports:
      - "4318:4318"  # OTLP HTTP
    volumes:
      - ./otel-config.yaml:/etc/otel/config.yaml

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"

  loki:
    image: grafana/loki:latest
    ports:
      - "3100:3100"

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
```

---

## Privacy & Data Collected

### What IS Collected

| Field | Purpose | PII Risk |
|-------|---------|----------|
| Machine ID | Deduplication | Low (SHA256 hash) |
| OS/Platform | Compatibility | None |
| Tool names | Coverage analysis | None |
| Error messages | Debugging | Low |
| Timing data | Performance | None |
| Jarvy version | Compatibility | None |

### What IS NOT Collected

- File paths (redacted from errors)
- Environment variable values
- Config file contents
- Usernames or home directories
- Network information
- Credentials

### Compliance

- **Opt-in by default**: Telemetry disabled until `jarvy telemetry enable`
- **CI-aware**: Auto-disabled when `CI=true` unless explicitly enabled
- **Override**: `JARVY_TELEMETRY=0` always respected
- **Data locality**: Users control where data goes (their OTLP endpoint)

---

## Success Metrics

1. **Unsupported Tool Visibility**: Query "top 10 requested unsupported tools" from OTEL backend
2. **Error Rate Dashboard**: Installation success rate by platform/tool visible in Grafana
3. **Performance Baseline**: p50/p95/p99 install times available as histograms
4. **MCP Feedback**: `tool.not_supported` events with `source=mcp` queryable within seconds

---

## Dependencies

Already in `Cargo.toml`:
- `opentelemetry = "0.22"`
- `opentelemetry-otlp = "0.15"`
- `opentelemetry_sdk = "0.22"`
- `tracing-opentelemetry` (for span integration)

To add:
- `opentelemetry-semantic-conventions = "0.14"` (standard attribute names)

To remove (after Phase 4):
- `ureq` (if only used by PostHog)

---

## Open Questions

1. **Default endpoint**: Should we default to localhost:4318 or require explicit configuration?
2. **Metric cardinality**: Cap tool name labels to known tools, hash unknowns?
3. **Trace sampling default**: 100% for debugging ease, or 10% for production?
4. **Batching config**: Expose batch size/interval or use OTEL defaults?

---

## References

- [OpenTelemetry Rust SDK](https://github.com/open-telemetry/opentelemetry-rust)
- [OTLP Specification](https://opentelemetry.io/docs/specs/otlp/)
- [Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/)
- [Grafana Alloy](https://grafana.com/docs/alloy/) (recommended collector)
- PRD-021: MCP Server (consumer of `tool.not_supported` events)
