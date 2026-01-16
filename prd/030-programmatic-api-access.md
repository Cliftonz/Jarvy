# PRD-030: Programmatic & API Access

## Overview

Provide programmatic access to Jarvy's functionality through a Rust library crate, REST API, and webhook integrations, enabling automation, dashboards, and custom tooling.

## Problem Statement

Jarvy is CLI-only, limiting automation and integration:
- No way to embed Jarvy in other tools
- Can't integrate with internal dashboards
- No webhook notifications for events
- Custom tooling requires parsing CLI output
- MCP server (PRD-021) is AI-focused, not general purpose

Internal platform teams and DevOps engineers need programmatic access to Jarvy's capabilities.

## Evidence

- Enterprise teams want to build on Jarvy
- DevOps needs dashboard integration
- CI/CD systems prefer APIs over CLI parsing
- Slack/Teams notifications requested
- Internal tooling teams blocked without library

## Requirements

### Functional Requirements

1. **Library crate**: `jarvy-core` for embedding
2. **REST API**: Local daemon with HTTP endpoints
3. **Webhook integrations**: Event notifications
4. **Event system**: Subscribe to Jarvy events
5. **JSON output**: Machine-readable for all commands

### Non-Functional Requirements

1. Library has minimal dependencies
2. API responds in < 100ms for status queries
3. Webhooks have retry logic
4. API documentation auto-generated
5. Authentication for remote access

## Non-Goals

- GraphQL API (REST is sufficient initially)
- Public hosted API service
- Client SDKs for other languages
- Real-time streaming API
- API versioning beyond v1

## Feature Specifications

### 1. Library Crate (`jarvy-core`)

Embeddable Rust library for Jarvy functionality.

```rust
// Cargo.toml
[dependencies]
jarvy-core = "0.1"

// Usage example
use jarvy_core::{Config, Setup, Tool, ToolStatus};

fn main() -> Result<(), jarvy_core::Error> {
    // Load configuration
    let config = Config::from_file("jarvy.toml")?;

    // Check tool status
    for tool in config.tools() {
        let status = Tool::check_status(&tool)?;
        match status {
            ToolStatus::Installed(version) => {
                println!("{}: {} installed", tool.name, version);
            }
            ToolStatus::Missing => {
                println!("{}: not installed", tool.name);
            }
            ToolStatus::Outdated { installed, required } => {
                println!("{}: {} installed, {} required", tool.name, installed, required);
            }
        }
    }

    // Run setup programmatically
    let setup = Setup::new(config)
        .with_callback(|event| {
            match event {
                SetupEvent::ToolStarted(name) => println!("Installing {}...", name),
                SetupEvent::ToolCompleted(name, version) => {
                    println!("✓ {} {}", name, version);
                }
                SetupEvent::ToolFailed(name, error) => {
                    eprintln!("✗ {}: {}", name, error);
                }
                _ => {}
            }
        })
        .dry_run(false);

    let result = setup.run()?;

    println!("Installed: {}", result.installed.len());
    println!("Failed: {}", result.failed.len());

    Ok(())
}
```

**Library modules:**

```rust
// jarvy-core/src/lib.rs
pub mod config;      // Configuration parsing
pub mod tools;       // Tool definitions and status
pub mod setup;       // Setup execution
pub mod doctor;      // Health checks
pub mod registry;    // Tool registry
pub mod version;     // Version management
pub mod events;      // Event system

// Re-exports
pub use config::Config;
pub use setup::{Setup, SetupResult, SetupEvent};
pub use tools::{Tool, ToolStatus, ToolSpec};
pub use doctor::{Doctor, DoctorResult};
pub use events::{Event, EventHandler};
```

**Library API examples:**

```rust
// Doctor functionality
use jarvy_core::{Doctor, DoctorCheck};

let doctor = Doctor::new();
let result = doctor.run()?;

for check in result.checks {
    match check.status {
        CheckStatus::Ok => println!("✓ {}", check.name),
        CheckStatus::Warning(msg) => println!("⚠ {}: {}", check.name, msg),
        CheckStatus::Error(msg) => println!("✗ {}: {}", check.name, msg),
    }
}

// Registry access
use jarvy_core::registry::Registry;

let registry = Registry::global();
let tools = registry.search("docker")?;

for tool in tools {
    println!("{}: {}", tool.name, tool.description);
}

// Version management
use jarvy_core::version::{History, Rollback};

let history = History::for_tool("node")?;
for entry in history.entries() {
    println!("{}: {}", entry.timestamp, entry.version);
}

let rollback = Rollback::new("node")
    .to_version("18.19.0")?
    .execute()?;
```

### 2. REST API

Local daemon providing HTTP API access.

```bash
# Start the API server
jarvy api start

# Output:
# Starting Jarvy API server...
#
# Server: http://localhost:9876
# API docs: http://localhost:9876/docs
# Health: http://localhost:9876/health
#
# Press Ctrl+C to stop

# Start with custom port
jarvy api start --port 8080

# Start in background
jarvy api start --daemon

# Stop the daemon
jarvy api stop

# Check daemon status
jarvy api status
```

**API Endpoints:**

```
GET  /health                    # Health check
GET  /api/v1/tools              # List all tools in config
GET  /api/v1/tools/:name        # Get tool status
POST /api/v1/tools/:name/install # Install a tool
POST /api/v1/tools/:name/upgrade # Upgrade a tool
POST /api/v1/tools/:name/rollback # Rollback a tool

GET  /api/v1/config             # Get current configuration
PUT  /api/v1/config             # Update configuration

POST /api/v1/setup              # Run setup
POST /api/v1/setup/dry-run      # Preview setup

GET  /api/v1/doctor             # Run doctor checks

GET  /api/v1/registry           # List available tools
GET  /api/v1/registry/search    # Search tools

GET  /api/v1/events             # SSE event stream
POST /api/v1/webhooks           # Register webhook
GET  /api/v1/webhooks           # List webhooks
DELETE /api/v1/webhooks/:id     # Remove webhook
```

**API Examples:**

```bash
# Get tool status
curl http://localhost:9876/api/v1/tools/node

# Response:
{
  "name": "node",
  "status": "installed",
  "version": "20.11.0",
  "required": "20",
  "satisfies": true,
  "path": "/Users/user/.nvm/versions/node/v20.11.0/bin/node",
  "install_method": "nvm"
}

# Install a tool
curl -X POST http://localhost:9876/api/v1/tools/docker/install

# Response:
{
  "success": true,
  "tool": "docker",
  "version": "24.0.7",
  "duration_ms": 45230
}

# Run setup
curl -X POST http://localhost:9876/api/v1/setup \
  -H "Content-Type: application/json" \
  -d '{"tools": ["git", "node", "docker"]}'

# Response:
{
  "success": true,
  "installed": ["docker"],
  "updated": ["node"],
  "unchanged": ["git"],
  "failed": [],
  "duration_ms": 52340
}

# Run doctor
curl http://localhost:9876/api/v1/doctor

# Response:
{
  "status": "warning",
  "checks": [
    {
      "name": "git",
      "status": "ok",
      "version": "2.43.0",
      "message": "Installed and up to date"
    },
    {
      "name": "node",
      "status": "warning",
      "version": "20.10.0",
      "message": "Update available: 20.11.0"
    }
  ],
  "recommendations": [
    "Run 'jarvy upgrade node' to update"
  ]
}

# Subscribe to events (SSE)
curl -N http://localhost:9876/api/v1/events

# Event stream:
data: {"event":"setup_started","timestamp":"2024-01-15T10:30:00Z"}

data: {"event":"tool_installing","tool":"node","timestamp":"2024-01-15T10:30:01Z"}

data: {"event":"tool_installed","tool":"node","version":"20.11.0","timestamp":"2024-01-15T10:30:15Z"}

data: {"event":"setup_completed","duration_ms":15000,"timestamp":"2024-01-15T10:30:15Z"}
```

**Authentication:**

```bash
# Start with authentication enabled
jarvy api start --auth

# Output:
# Generating API token...
#
# API Token: jarvy_sk_abc123...
#
# Store this securely. It will not be shown again.
# Set in environment: export JARVY_API_TOKEN=jarvy_sk_abc123...

# Using the API with auth
curl -H "Authorization: Bearer jarvy_sk_abc123..." \
  http://localhost:9876/api/v1/tools
```

### 3. Webhook Integrations

Event notifications to external services.

```bash
# Register a webhook
jarvy webhook add slack https://hooks.slack.com/services/XXX/YYY/ZZZ

# Output:
# Webhook registered:
#   ID: wh_abc123
#   URL: https://hooks.slack.com/services/XXX/YYY/ZZZ
#   Events: all
#   Status: Active

# Register with specific events
jarvy webhook add slack https://hooks.slack.com/... \
  --events setup.failed,tool.failed

# List webhooks
jarvy webhook list

# Output:
# Registered Webhooks
# ===================
#
# ID          Name    URL                             Events
# ─────────────────────────────────────────────────────────────
# wh_abc123   slack   hooks.slack.com/services/...    all
# wh_def456   teams   outlook.office.com/webhook/...  failures
# wh_ghi789   custom  internal.company.com/jarvy      all

# Test webhook
jarvy webhook test wh_abc123

# Remove webhook
jarvy webhook remove wh_abc123
```

**Webhook payload format:**

```json
// Tool installed
{
  "event": "tool.installed",
  "timestamp": "2024-01-15T10:30:15Z",
  "data": {
    "tool": "node",
    "version": "20.11.0",
    "duration_ms": 14500,
    "machine_id": "abc123"
  }
}

// Setup completed
{
  "event": "setup.completed",
  "timestamp": "2024-01-15T10:30:45Z",
  "data": {
    "installed": ["node", "docker"],
    "updated": ["git"],
    "failed": [],
    "duration_ms": 45230,
    "machine_id": "abc123"
  }
}

// Setup failed
{
  "event": "setup.failed",
  "timestamp": "2024-01-15T10:31:00Z",
  "data": {
    "error": "Permission denied",
    "tool": "docker",
    "machine_id": "abc123"
  }
}
```

**Slack/Teams formatting:**

```bash
# Register with Slack formatting
jarvy webhook add slack https://hooks.slack.com/... --format slack

# Slack message format:
{
  "blocks": [
    {
      "type": "section",
      "text": {
        "type": "mrkdwn",
        "text": "✓ *Jarvy Setup Completed*\n\nInstalled: node, docker\nUpdated: git\nDuration: 45s"
      }
    }
  ]
}

# Register with Teams formatting
jarvy webhook add teams https://outlook.office.com/webhook/... --format teams
```

**Webhook features:**
- Multiple webhook targets
- Event filtering
- Retry logic (3 attempts)
- Custom formatting
- Secret signing

### 4. Event System

Subscribe to Jarvy events programmatically.

```rust
// Using the library
use jarvy_core::events::{EventBus, Event, EventHandler};

let mut bus = EventBus::new();

// Subscribe to specific events
bus.subscribe(Event::ToolInstalled, |event| {
    println!("Tool installed: {:?}", event);
});

// Subscribe to all events
bus.subscribe_all(|event| {
    log::info!("Jarvy event: {:?}", event);
});

// Run setup with events
let setup = Setup::new(config)
    .with_event_bus(&bus);

setup.run()?;
```

**Event types:**

```rust
pub enum Event {
    // Setup events
    SetupStarted { tools: Vec<String> },
    SetupCompleted { result: SetupResult },
    SetupFailed { error: Error },

    // Tool events
    ToolChecking { name: String },
    ToolInstalling { name: String },
    ToolInstalled { name: String, version: String },
    ToolUpdating { name: String, from: String, to: String },
    ToolUpdated { name: String, version: String },
    ToolFailed { name: String, error: Error },
    ToolSkipped { name: String, reason: String },

    // Hook events
    HookStarted { tool: String },
    HookCompleted { tool: String },
    HookFailed { tool: String, error: Error },

    // Doctor events
    DoctorStarted,
    DoctorCompleted { result: DoctorResult },
    CheckPassed { name: String },
    CheckWarning { name: String, message: String },
    CheckFailed { name: String, message: String },
}
```

### 5. Enhanced JSON Output

Machine-readable output for all commands.

```bash
# All commands support --format json
jarvy setup --format json

# Output:
{
  "success": true,
  "tools": {
    "installed": [
      {"name": "node", "version": "20.11.0", "duration_ms": 14500}
    ],
    "updated": [
      {"name": "git", "from": "2.42.0", "to": "2.43.0", "duration_ms": 5200}
    ],
    "unchanged": [
      {"name": "docker", "version": "24.0.7"}
    ],
    "failed": []
  },
  "hooks": {
    "executed": ["node", "git"],
    "failed": []
  },
  "duration_ms": 45230,
  "timestamp": "2024-01-15T10:30:45Z"
}

# Doctor with JSON
jarvy doctor --format json

# Output:
{
  "status": "warning",
  "system": {
    "os": "macos",
    "version": "14.2",
    "arch": "arm64",
    "shell": "/bin/zsh"
  },
  "package_managers": [
    {"name": "homebrew", "version": "4.2.0", "status": "ok"}
  ],
  "tools": [
    {
      "name": "git",
      "required": ">=2.40",
      "installed": "2.43.0",
      "status": "ok"
    },
    {
      "name": "node",
      "required": "20",
      "installed": "20.10.0",
      "status": "outdated",
      "available": "20.11.0"
    }
  ],
  "recommendations": [
    {"severity": "info", "message": "Update node to 20.11.0"}
  ]
}

# Streaming JSON (newline-delimited)
jarvy setup --format json-stream

# Output (one JSON object per line):
{"event":"setup_started","tools":["node","docker"]}
{"event":"tool_installing","tool":"node"}
{"event":"tool_installed","tool":"node","version":"20.11.0"}
{"event":"tool_installing","tool":"docker"}
{"event":"tool_installed","tool":"docker","version":"24.0.7"}
{"event":"setup_completed","success":true,"duration_ms":45230}
```

## Acceptance Criteria

### Library Crate
- [ ] `jarvy-core` crate published
- [ ] Config loading API works
- [ ] Tool status checking API works
- [ ] Setup API with callbacks works
- [ ] Doctor API works
- [ ] Registry search API works
- [ ] Documentation generated
- [ ] Examples provided

### REST API
- [ ] `jarvy api start` launches server
- [ ] All endpoints documented
- [ ] OpenAPI spec generated
- [ ] Authentication optional
- [ ] SSE event stream works
- [ ] Daemon mode works
- [ ] Health endpoint works

### Webhook Integrations
- [ ] `jarvy webhook add` registers hooks
- [ ] `jarvy webhook list` shows hooks
- [ ] `jarvy webhook test` sends test
- [ ] Event filtering works
- [ ] Retry logic implemented
- [ ] Slack/Teams formatting works
- [ ] Secret signing available

### Event System
- [ ] EventBus API works
- [ ] All events documented
- [ ] Subscribe/unsubscribe works
- [ ] Events fire correctly
- [ ] No performance impact

### JSON Output
- [ ] All commands support `--format json`
- [ ] Output schema documented
- [ ] Streaming JSON works
- [ ] Backward compatible

## Technical Approach

### Module Structure

```
crates/
  jarvy-core/
    src/
      lib.rs            # Library entry
      config.rs         # Configuration
      tools.rs          # Tool management
      setup.rs          # Setup execution
      doctor.rs         # Health checks
      registry.rs       # Tool registry
      version.rs        # Version management
      events.rs         # Event system
    Cargo.toml
src/
  api/
    mod.rs              # API server
    routes.rs           # API endpoints
    auth.rs             # Authentication
    sse.rs              # Server-sent events
  webhook/
    mod.rs              # Webhook management
    dispatch.rs         # Webhook dispatch
    formatters.rs       # Slack/Teams formatting
```

### API Server Implementation

```rust
// src/api/mod.rs
use axum::{Router, routing::{get, post}};
use std::net::SocketAddr;

pub struct ApiServer {
    config: ApiConfig,
    event_bus: EventBus,
}

impl ApiServer {
    pub async fn start(self) -> Result<(), Error> {
        let app = Router::new()
            .route("/health", get(health))
            .route("/api/v1/tools", get(list_tools))
            .route("/api/v1/tools/:name", get(get_tool))
            .route("/api/v1/tools/:name/install", post(install_tool))
            .route("/api/v1/setup", post(run_setup))
            .route("/api/v1/doctor", get(run_doctor))
            .route("/api/v1/events", get(event_stream))
            .layer(self.auth_layer());

        let addr = SocketAddr::from(([127, 0, 0, 1], self.config.port));
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;

        Ok(())
    }
}

// Event stream endpoint
async fn event_stream(
    State(bus): State<Arc<EventBus>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = bus.subscribe_all();

    Sse::new(ReceiverStream::new(rx).map(|event| {
        Ok(Event::default().data(serde_json::to_string(&event).unwrap()))
    }))
}
```

### Webhook Dispatcher

```rust
// src/webhook/dispatch.rs
pub struct WebhookDispatcher {
    client: reqwest::Client,
    webhooks: Vec<Webhook>,
}

impl WebhookDispatcher {
    pub async fn dispatch(&self, event: &JarvyEvent) -> Result<(), Error> {
        for webhook in &self.webhooks {
            if webhook.matches_event(event) {
                self.send_webhook(webhook, event).await?;
            }
        }
        Ok(())
    }

    async fn send_webhook(&self, webhook: &Webhook, event: &JarvyEvent) -> Result<(), Error> {
        let payload = match webhook.format {
            Format::Json => serde_json::to_string(event)?,
            Format::Slack => self.format_slack(event)?,
            Format::Teams => self.format_teams(event)?,
        };

        let mut attempts = 0;
        let max_attempts = 3;

        while attempts < max_attempts {
            match self.client
                .post(&webhook.url)
                .header("Content-Type", "application/json")
                .header("X-Jarvy-Signature", self.sign(&payload, &webhook.secret))
                .body(payload.clone())
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => return Ok(()),
                _ => {
                    attempts += 1;
                    tokio::time::sleep(Duration::from_secs(2u64.pow(attempts))).await;
                }
            }
        }

        Err(Error::WebhookFailed)
    }
}
```

## Implementation Steps

1. Create `jarvy-core` crate structure
2. Extract core logic to library
3. Implement config API
4. Implement tools API
5. Implement setup API
6. Implement doctor API
7. Implement event system
8. Build REST API server
9. Implement API endpoints
10. Add authentication
11. Implement SSE
12. Build webhook system
13. Add Slack/Teams formatters
14. Enhance JSON output
15. Generate API documentation
16. Write tests
17. Write examples

## Dependencies

### jarvy-core
- `serde` - Serialization (existing)
- `toml` - Config parsing (existing)
- `thiserror` - Error handling (existing)

### API Server
- `axum` - HTTP framework
- `tokio` - Async runtime (existing)
- `tower` - Middleware
- `utoipa` - OpenAPI generation

### Webhooks
- `reqwest` - HTTP client
- `hmac` - Signature generation

## Effort Estimate

| Task | Effort |
|------|--------|
| jarvy-core crate structure | 1 day |
| Extract core logic | 3 days |
| Config API | 1 day |
| Tools API | 1.5 days |
| Setup API | 2 days |
| Doctor API | 1 day |
| Event system | 1.5 days |
| REST API server | 2 days |
| API endpoints | 2.5 days |
| Authentication | 1 day |
| SSE implementation | 1 day |
| Webhook system | 2 days |
| Slack/Teams formatters | 1 day |
| JSON output enhancement | 1 day |
| API documentation | 1 day |
| Testing | 3 days |
| Examples | 1 day |
| **Total** | **26.5 days** |

## Files to Create/Modify

### New Files
- `crates/jarvy-core/src/lib.rs`
- `crates/jarvy-core/src/config.rs`
- `crates/jarvy-core/src/tools.rs`
- `crates/jarvy-core/src/setup.rs`
- `crates/jarvy-core/src/doctor.rs`
- `crates/jarvy-core/src/registry.rs`
- `crates/jarvy-core/src/events.rs`
- `crates/jarvy-core/Cargo.toml`
- `src/api/mod.rs`
- `src/api/routes.rs`
- `src/api/auth.rs`
- `src/api/sse.rs`
- `src/webhook/mod.rs`
- `src/webhook/dispatch.rs`
- `src/webhook/formatters.rs`
- `tests/api_integration.rs`
- `examples/library_usage.rs`

### Modified Files
- `Cargo.toml` - Add workspace member, dependencies
- `src/main.rs` - Add api, webhook commands
- `CLAUDE.md` - Document API features

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Library access | None | Full API |
| REST API | None | Complete |
| Webhook support | None | Full |
| Event system | None | Implemented |
| JSON output | Partial | Complete |
| Integration ease | CLI parsing | SDK |

## Risks

1. **API stability**: API changes break integrations
   - Mitigation: Versioned API, deprecation policy

2. **Security**: Remote API access risks
   - Mitigation: Auth required, localhost default

3. **Performance**: API adds overhead
   - Mitigation: Efficient implementation, caching

4. **Maintenance**: Two codepaths (CLI and library)
   - Mitigation: CLI uses library internally

5. **Dependencies**: New deps increase binary size
   - Mitigation: Feature flags, minimal deps
