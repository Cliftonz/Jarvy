# PRD-005: Error Handling Improvements

## Overview

Replace panics with proper error returns, add context to errors, and improve user-facing error messages throughout the codebase.

## Problem Statement

Current error handling has several issues:

1. **Panics in production code**: `setup.rs` uses `.expect()` and `panic!()`
2. **Lost context**: `InstallError::Prereq(&'static str)` can't include dynamic info
3. **Unhelpful messages**: "Failed to parse config" with no line number
4. **No retry logic**: Single-attempt operations fail on transient errors
5. **Swallowed details**: Match arms use `_` ignoring actual error info

## Evidence

From `setup.rs` (lines 54-85):
```rust
Err(_) => { panic!("Failed to run Homebrew check"); }
```

From `config.rs` (lines 58-72):
```rust
Err(_) => {
    println!("Failed to parse config file. Please ensure it's in correct format.");
    process::exit(crate::error_codes::CONFIG_ERROR);
}
```

From `common.rs`:
```rust
pub enum InstallError {
    Prereq(&'static str),  // Can't include tool name or version
}
```

## Requirements

### Functional Requirements

1. **No panics in library code**: All errors return `Result<T, Error>`
2. **Contextual errors**: Include tool name, version, platform in errors
3. **Structured error chain**: Errors wrap underlying causes
4. **User-friendly display**: Errors format nicely for CLI output
5. **Actionable remediation**: Suggest fixes for common errors
6. **Retry support**: Transient errors can be retried

### Non-Functional Requirements

1. Zero-cost error construction (no heap allocation for static errors)
2. Errors implement `std::error::Error` trait
3. Debug and Display traits for all errors
4. Serde serialization for telemetry

## New Error Type Design

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JarvyError {
    #[error("Tool '{tool}' not found in registry")]
    ToolNotFound { tool: String },

    #[error("Tool '{tool}' is already installed (version {installed})")]
    AlreadyInstalled { tool: String, installed: String },

    #[error("Prerequisite missing: {tool} is required but not installed\n  Remediation: {remediation}")]
    PrerequisiteMissing { tool: String, remediation: String },

    #[error("Platform '{platform}' is not supported for tool '{tool}'")]
    PlatformUnsupported { tool: String, platform: String },

    #[error("Command failed: {cmd}\n  Exit code: {exit_code}\n  Stderr: {stderr}")]
    CommandFailed {
        cmd: String,
        exit_code: i32,
        stderr: String,
    },

    #[error("Permission denied: {action}\n  Try running with sudo or as administrator")]
    PermissionDenied { action: String },

    #[error("Network error: {message}\n  Check your internet connection")]
    NetworkError { message: String, #[source] source: Option<Box<dyn std::error::Error + Send + Sync>> },

    #[error("Timeout after {seconds}s: {operation}")]
    Timeout { operation: String, seconds: u64 },

    #[error("Configuration error at {file}:{line}:{column}\n  {message}")]
    ConfigError {
        file: String,
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Configuration error: {message}")]
    ConfigParseError { message: String, #[source] source: toml::de::Error },

    #[error("I/O error: {context}")]
    Io { context: String, #[source] source: std::io::Error },

    #[error("Version mismatch for {tool}: required {required}, found {installed}")]
    VersionMismatch {
        tool: String,
        required: String,
        installed: String,
    },
}

impl JarvyError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::ConfigError { .. } | Self::ConfigParseError { .. } => 2,
            Self::PrerequisiteMissing { .. } => 3,
            Self::NetworkError { .. } | Self::Timeout { .. } => 4,
            Self::PermissionDenied { .. } => 5,
            _ => 1,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::NetworkError { .. } | Self::Timeout { .. })
    }
}
```

## Context Extension Trait

```rust
// src/error.rs
pub trait ResultExt<T> {
    fn context(self, ctx: impl Into<String>) -> Result<T, JarvyError>;
    fn with_tool(self, tool: &str) -> Result<T, JarvyError>;
}

impl<T, E: Into<JarvyError>> ResultExt<T> for Result<T, E> {
    fn context(self, ctx: impl Into<String>) -> Result<T, JarvyError> {
        self.map_err(|e| {
            let inner = e.into();
            // Wrap with context
            inner
        })
    }

    fn with_tool(self, tool: &str) -> Result<T, JarvyError> {
        self.map_err(|e| {
            // Add tool name to error context
            e.into()
        })
    }
}
```

## Config Error Improvement

```rust
// src/config.rs
pub fn load(path: &Path) -> Result<Config, JarvyError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| JarvyError::Io {
            context: format!("Failed to read config file: {}", path.display()),
            source: e,
        })?;

    toml::from_str(&content).map_err(|e| {
        // Extract line/column from TOML error
        let (line, col) = e.line_col().unwrap_or((0, 0));
        JarvyError::ConfigError {
            file: path.display().to_string(),
            line: line + 1,
            column: col + 1,
            message: e.message().to_string(),
        }
    })
}
```

## Panic Replacement Pattern

Before:
```rust
// setup.rs
let output = Command::new("brew")
    .arg("--version")
    .output()
    .expect("Failed to run Homebrew check");
```

After:
```rust
// setup.rs
let output = Command::new("brew")
    .arg("--version")
    .output()
    .map_err(|e| JarvyError::CommandFailed {
        cmd: "brew --version".into(),
        exit_code: -1,
        stderr: e.to_string(),
    })?;
```

## Retry Logic

```rust
// src/retry.rs
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            backoff_factor: 2.0,
        }
    }
}

pub fn with_retry<T, F>(config: &RetryConfig, mut f: F) -> Result<T, JarvyError>
where
    F: FnMut() -> Result<T, JarvyError>,
{
    let mut delay = config.initial_delay;
    let mut last_error = None;

    for attempt in 1..=config.max_attempts {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) if e.is_retryable() => {
                eprintln!("Attempt {}/{} failed: {}. Retrying in {:?}...",
                    attempt, config.max_attempts, e, delay);
                std::thread::sleep(delay);
                delay = (delay.as_secs_f64() * config.backoff_factor)
                    .min(config.max_delay.as_secs_f64())
                    .into();
                last_error = Some(e);
            }
            Err(e) => return Err(e),
        }
    }

    Err(last_error.unwrap())
}
```

## User-Friendly Error Display

```rust
// src/main.rs
fn main() {
    if let Err(e) = run() {
        // Pretty-print error with box
        eprintln!("\n┌─ Error ─────────────────────────────────────┐");
        for line in e.to_string().lines() {
            eprintln!("│ {:<43} │", line);
        }
        eprintln!("└─────────────────────────────────────────────┘\n");

        // Show source chain in verbose mode
        if std::env::var("JARVY_VERBOSE").is_ok() {
            let mut source = e.source();
            while let Some(s) = source {
                eprintln!("  Caused by: {}", s);
                source = s.source();
            }
        }

        std::process::exit(e.exit_code());
    }
}
```

## Implementation Steps

1. Create `src/error.rs` with new `JarvyError` enum
2. Add `ResultExt` trait for context chaining
3. Create `src/retry.rs` with retry logic
4. Refactor `config.rs` to return Result instead of exiting
5. Replace all `panic!()` and `.expect()` in setup.rs
6. Replace all `.unwrap()` in library code
7. Update `InstallError` to use new types or migrate fully
8. Add error formatting for CLI output
9. Write tests for error cases
10. Update telemetry to log structured errors

## Migration Strategy

Phase 1: Add new error types alongside existing
Phase 2: Migrate config.rs and setup.rs
Phase 3: Migrate tool implementations
Phase 4: Remove old InstallError type

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Panic calls in lib | 15+ | 0 |
| .unwrap() in lib | 30+ | 0 |
| Error context quality | Low | High |
| Retry support | None | All network ops |

## Risks

1. **Large refactor**: Touches many files
   - Mitigation: Phased migration with compatibility layer
2. **Performance**: String allocation in errors
   - Mitigation: Use Cow<'static, str> where possible
3. **Breaking API**: Public error types change
   - Mitigation: Keep InstallError as alias initially

## Dependencies

- `thiserror` (already in use)
- No new dependencies

## Effort Estimate

- Error type design: 0.5 days
- config.rs migration: 1 day
- setup.rs migration: 1 day
- Tool migration: 2 days
- Retry logic: 0.5 days
- Testing: 1 day

## Files to Modify

- `src/error.rs` - New comprehensive error module
- `src/retry.rs` - New retry logic
- `src/config.rs` - Return Result, detailed errors
- `src/setup.rs` - Replace panics
- `src/tools/common.rs` - Migrate InstallError
- `src/main.rs` - Error display
- All tool files - Update error handling
