# PRD-046: Per-Tool Performance Analytics

## Overview

Enable Jarvy to track and report per-tool installation metrics including duration, success rates, failure reasons, and resource usage, helping identify bottlenecks and problematic tools for optimization.

## Problem Statement

When `jarvy setup` runs slowly or fails, users lack visibility into:

- Which tools take the longest to install
- Which tools fail most frequently
- Why specific tools fail (network, permissions, conflicts)
- Historical trends in installation performance
- Resource usage during installation

This lack of observability makes troubleshooting and optimization difficult.

## Evidence

- "Setup is slow" without knowing which tools cause delays
- Repeated failures without tracking which tools are problematic
- No data to prioritize optimization efforts
- Network issues blamed without evidence
- Cannot compare setup times across machines/runs

## Requirements

### Functional Requirements

1. **Timing**: Track installation duration per tool
2. **Status tracking**: Record success/failure per tool
3. **Error capture**: Capture failure reasons and messages
4. **History**: Store historical data for trend analysis
5. **Reporting**: Generate human-readable reports
6. **Export**: Export data for external analysis

### Non-Functional Requirements

1. **Low overhead**: <1% impact on installation time
2. **Privacy-respecting**: No PII, tool names/times only
3. **Persistent**: Survive restarts/crashes
4. **Queryable**: Efficient filtering and aggregation
5. **Bounded**: Limit storage to prevent disk bloat

## Non-Goals

- Real-time dashboards (use OTEL for that)
- Cross-machine aggregation (server-side analytics)
- Predictive analytics
- Automatic optimization recommendations
- Integration with external APM tools

## Feature Specifications

### 1. Metrics Collection

```rust
// Collected for each tool installation
struct ToolMetrics {
    tool_name: String,
    version: String,
    install_method: String,        // brew, apt, cargo, etc.

    // Timing
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    duration_ms: u64,

    // Status
    status: InstallStatus,         // Success, Failed, Skipped
    skip_reason: Option<String>,   // "already installed", "dependency missing"

    // Error details (if failed)
    error_message: Option<String>,
    error_type: Option<String>,    // network, permission, conflict, unknown

    // Resource usage
    network_bytes: Option<u64>,    // Downloaded bytes
    disk_bytes: Option<u64>,       // Installed size

    // Context
    os: String,
    arch: String,
    jarvy_version: String,
}
```

### 2. CLI Commands

```bash
# Show performance summary for last run
jarvy analytics

# Output:
# Setup Performance Report
# ========================
# Last run: 2024-01-20 15:30:00 (2 minutes ago)
# Total duration: 3m 42s
# Tools: 15 installed, 3 skipped, 1 failed
#
# Slowest Tools:
#   1. rust           45.2s  (rustup, download + install)
#   2. docker         32.1s  (cask, large download)
#   3. node           18.5s  (brew, compile from source)
#   4. kubectl        12.3s  (brew)
#   5. ripgrep         3.2s  (brew)
#
# Failed:
#   terraform  - network timeout (retry suggested)
#
# Skipped:
#   git        - already installed (2.43.0)
#   vim        - already installed (9.0)
#   curl       - already installed (8.4.0)

# Show historical analytics
jarvy analytics history

# Output:
# Installation History (last 30 days)
# ===================================
#
# Run #    Date        Duration  Tools  Success  Failed
# ─────────────────────────────────────────────────────
# 1        2024-01-20  3m 42s    19     18       1
# 2        2024-01-18  4m 15s    19     19       0
# 3        2024-01-15  12m 03s   19     17       2
# 4        2024-01-10  3m 38s    15     15       0
#
# Trends:
#   Average duration: 5m 55s
#   Success rate: 94.7%
#   Most failed: terraform (3 failures)

# Show tool-specific history
jarvy analytics tool rust

# Output:
# Tool: rust
# ==========
# Install method: rustup
# Avg duration: 42.3s
# Success rate: 100%
# Last 5 runs:
#   2024-01-20  45.2s  ✓  1.75.0
#   2024-01-18  41.1s  ✓  1.75.0
#   2024-01-15  43.8s  ✓  1.74.0
#   2024-01-10  38.5s  ✓  1.74.0
#   2024-01-05  42.1s  ✓  1.74.0

# Show failure analysis
jarvy analytics failures

# Output:
# Failure Analysis
# ================
#
# By Tool:
#   terraform    3 failures  (network: 2, permission: 1)
#   kubectl      1 failure   (network: 1)
#
# By Error Type:
#   network      3 failures  (60%)
#   permission   1 failure   (20%)
#   conflict     1 failure   (20%)
#
# Common failure times:
#   Morning (9-11am): 2 failures
#   Evening (5-7pm):  3 failures

# Export analytics data
jarvy analytics export --format json > analytics.json
jarvy analytics export --format csv > analytics.csv

# Clear analytics history
jarvy analytics clear --older-than 90d
```

### 3. Configuration

```toml
# jarvy.toml

[analytics]
# Enable/disable analytics collection
enabled = true

# Storage location (default: ~/.jarvy/analytics/)
storage_path = "~/.jarvy/analytics"

# Retention policy
retention_days = 90
max_runs = 100

# What to track
track_timing = true
track_errors = true
track_resource_usage = false  # Requires additional overhead
```

### 4. Storage Format

```json
// ~/.jarvy/analytics/runs/2024-01-20T15-30-00.json
{
  "run_id": "run_2024-01-20T15-30-00_abc123",
  "started_at": "2024-01-20T15:30:00Z",
  "ended_at": "2024-01-20T15:33:42Z",
  "total_duration_ms": 222000,
  "config_hash": "sha256:abc123...",
  "environment": {
    "os": "darwin",
    "arch": "aarch64",
    "jarvy_version": "0.5.0"
  },
  "summary": {
    "total_tools": 19,
    "installed": 15,
    "skipped": 3,
    "failed": 1
  },
  "tools": [
    {
      "name": "rust",
      "version": "1.75.0",
      "install_method": "rustup",
      "started_at": "2024-01-20T15:30:05Z",
      "duration_ms": 45200,
      "status": "success",
      "network_bytes": 245000000
    },
    {
      "name": "terraform",
      "version": "1.7.0",
      "install_method": "brew",
      "started_at": "2024-01-20T15:32:15Z",
      "duration_ms": 30000,
      "status": "failed",
      "error_type": "network",
      "error_message": "Connection timeout after 30s"
    }
  ]
}
```

## Technical Approach

### Module Structure

```
src/
  analytics/
    mod.rs           # Public API
    config.rs        # Analytics configuration
    collector.rs     # Metrics collection
    storage.rs       # Persistence layer
    reporter.rs      # Report generation
    aggregator.rs    # Historical aggregation
    commands.rs      # CLI command handlers
```

### Metrics Collection

```rust
// src/analytics/collector.rs
use std::time::Instant;
use std::sync::Mutex;

pub struct AnalyticsCollector {
    config: AnalyticsConfig,
    current_run: Mutex<Option<RunMetrics>>,
}

pub struct RunMetrics {
    run_id: String,
    started_at: chrono::DateTime<chrono::Utc>,
    tools: Vec<ToolMetrics>,
    environment: EnvironmentInfo,
}

impl AnalyticsCollector {
    pub fn start_run(&self) {
        let mut current = self.current_run.lock().unwrap();
        *current = Some(RunMetrics {
            run_id: generate_run_id(),
            started_at: chrono::Utc::now(),
            tools: Vec::new(),
            environment: EnvironmentInfo::capture(),
        });
    }

    pub fn start_tool(&self, tool_name: &str, version: &str, method: &str) -> ToolTimer {
        ToolTimer {
            tool_name: tool_name.to_string(),
            version: version.to_string(),
            install_method: method.to_string(),
            started_at: chrono::Utc::now(),
            start_instant: Instant::now(),
            collector: self,
        }
    }

    pub fn record_tool(&self, metrics: ToolMetrics) {
        if let Ok(mut current) = self.current_run.lock() {
            if let Some(ref mut run) = *current {
                run.tools.push(metrics);
            }
        }
    }

    pub fn finish_run(&self) -> Result<(), AnalyticsError> {
        let run = {
            let mut current = self.current_run.lock().unwrap();
            current.take()
        };

        if let Some(run) = run {
            self.storage.save_run(&run)?;
        }

        Ok(())
    }
}

pub struct ToolTimer<'a> {
    tool_name: String,
    version: String,
    install_method: String,
    started_at: chrono::DateTime<chrono::Utc>,
    start_instant: Instant,
    collector: &'a AnalyticsCollector,
}

impl<'a> ToolTimer<'a> {
    pub fn success(self) {
        let metrics = ToolMetrics {
            tool_name: self.tool_name,
            version: self.version,
            install_method: self.install_method,
            started_at: self.started_at,
            ended_at: chrono::Utc::now(),
            duration_ms: self.start_instant.elapsed().as_millis() as u64,
            status: InstallStatus::Success,
            skip_reason: None,
            error_message: None,
            error_type: None,
            network_bytes: None,
            disk_bytes: None,
        };
        self.collector.record_tool(metrics);
    }

    pub fn failed(self, error: &InstallError) {
        let metrics = ToolMetrics {
            tool_name: self.tool_name,
            version: self.version,
            install_method: self.install_method,
            started_at: self.started_at,
            ended_at: chrono::Utc::now(),
            duration_ms: self.start_instant.elapsed().as_millis() as u64,
            status: InstallStatus::Failed,
            skip_reason: None,
            error_message: Some(error.to_string()),
            error_type: Some(classify_error(error)),
            network_bytes: None,
            disk_bytes: None,
        };
        self.collector.record_tool(metrics);
    }

    pub fn skipped(self, reason: &str) {
        let metrics = ToolMetrics {
            tool_name: self.tool_name,
            version: self.version,
            install_method: self.install_method,
            started_at: self.started_at,
            ended_at: chrono::Utc::now(),
            duration_ms: 0,
            status: InstallStatus::Skipped,
            skip_reason: Some(reason.to_string()),
            error_message: None,
            error_type: None,
            network_bytes: None,
            disk_bytes: None,
        };
        self.collector.record_tool(metrics);
    }
}
```

### Storage Layer

```rust
// src/analytics/storage.rs
use std::path::PathBuf;

pub struct AnalyticsStorage {
    storage_path: PathBuf,
    retention_days: u32,
    max_runs: usize,
}

impl AnalyticsStorage {
    pub fn save_run(&self, run: &RunMetrics) -> Result<(), AnalyticsError> {
        let runs_dir = self.storage_path.join("runs");
        std::fs::create_dir_all(&runs_dir)?;

        let filename = format!("{}.json", run.run_id);
        let path = runs_dir.join(filename);

        let content = serde_json::to_string_pretty(run)?;
        std::fs::write(&path, content)?;

        // Cleanup old runs
        self.cleanup_old_runs()?;

        Ok(())
    }

    pub fn list_runs(&self) -> Result<Vec<RunSummary>, AnalyticsError> {
        let runs_dir = self.storage_path.join("runs");
        let mut runs = Vec::new();

        for entry in std::fs::read_dir(&runs_dir)? {
            let entry = entry?;
            if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(run) = self.load_run(&entry.path()) {
                    runs.push(run.summary());
                }
            }
        }

        runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(runs)
    }

    pub fn load_run(&self, path: &Path) -> Result<RunMetrics, AnalyticsError> {
        let content = std::fs::read_to_string(path)?;
        let run: RunMetrics = serde_json::from_str(&content)?;
        Ok(run)
    }

    pub fn get_tool_history(&self, tool_name: &str, limit: usize) -> Result<Vec<ToolMetrics>, AnalyticsError> {
        let runs = self.list_runs()?;
        let mut history = Vec::new();

        for run_summary in runs.iter().take(limit * 2) {
            let run = self.load_run_by_id(&run_summary.run_id)?;
            for tool in run.tools {
                if tool.tool_name == tool_name {
                    history.push(tool);
                    if history.len() >= limit {
                        return Ok(history);
                    }
                }
            }
        }

        Ok(history)
    }

    fn cleanup_old_runs(&self) -> Result<(), AnalyticsError> {
        let runs_dir = self.storage_path.join("runs");
        let cutoff = chrono::Utc::now() - chrono::Duration::days(self.retention_days as i64);

        let mut runs: Vec<_> = std::fs::read_dir(&runs_dir)?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let metadata = e.metadata().ok()?;
                let modified = metadata.modified().ok()?;
                Some((e.path(), modified))
            })
            .collect();

        // Sort by modification time (oldest first)
        runs.sort_by(|a, b| a.1.cmp(&b.1));

        // Remove old files
        for (path, modified) in &runs {
            let modified_time: chrono::DateTime<chrono::Utc> = (*modified).into();
            if modified_time < cutoff {
                std::fs::remove_file(path)?;
            }
        }

        // Remove excess files
        while runs.len() > self.max_runs {
            if let Some((path, _)) = runs.first() {
                std::fs::remove_file(path)?;
                runs.remove(0);
            }
        }

        Ok(())
    }
}
```

### Report Generation

```rust
// src/analytics/reporter.rs
use colored::Colorize;

pub struct AnalyticsReporter;

impl AnalyticsReporter {
    pub fn print_summary(run: &RunMetrics) {
        println!("Setup Performance Report");
        println!("========================");
        println!("Run: {} ({} ago)", run.started_at, humanize_duration(run.age()));
        println!("Total duration: {}", humanize_duration(run.total_duration()));
        println!("Tools: {} installed, {} skipped, {} failed\n",
            run.count_by_status(InstallStatus::Success).to_string().green(),
            run.count_by_status(InstallStatus::Skipped).to_string().yellow(),
            run.count_by_status(InstallStatus::Failed).to_string().red()
        );

        // Slowest tools
        let mut slowest: Vec<_> = run.tools.iter()
            .filter(|t| t.status == InstallStatus::Success)
            .collect();
        slowest.sort_by(|a, b| b.duration_ms.cmp(&a.duration_ms));

        println!("Slowest Tools:");
        for (i, tool) in slowest.iter().take(5).enumerate() {
            println!("  {}. {:<15} {:>6}  ({})",
                i + 1,
                tool.tool_name,
                humanize_duration_ms(tool.duration_ms),
                tool.install_method
            );
        }

        // Failed tools
        let failed: Vec<_> = run.tools.iter()
            .filter(|t| t.status == InstallStatus::Failed)
            .collect();

        if !failed.is_empty() {
            println!("\nFailed:");
            for tool in failed {
                println!("  {}  - {} ({})",
                    tool.tool_name.red(),
                    tool.error_message.as_deref().unwrap_or("unknown error"),
                    tool.error_type.as_deref().unwrap_or("unknown")
                );
            }
        }

        // Skipped tools
        let skipped: Vec<_> = run.tools.iter()
            .filter(|t| t.status == InstallStatus::Skipped)
            .collect();

        if !skipped.is_empty() {
            println!("\nSkipped:");
            for tool in skipped {
                println!("  {}  - {}",
                    tool.tool_name.yellow(),
                    tool.skip_reason.as_deref().unwrap_or("no reason given")
                );
            }
        }
    }

    pub fn print_history(runs: &[RunSummary]) {
        println!("Installation History (last 30 days)");
        println!("===================================\n");
        println!("{:<8} {:<12} {:<10} {:<7} {:<8} {:<8}",
            "Run #", "Date", "Duration", "Tools", "Success", "Failed"
        );
        println!("{}", "─".repeat(55));

        for (i, run) in runs.iter().enumerate() {
            println!("{:<8} {:<12} {:<10} {:<7} {:<8} {:<8}",
                i + 1,
                run.started_at.format("%Y-%m-%d"),
                humanize_duration_ms(run.total_duration_ms),
                run.total_tools,
                run.success_count.to_string().green(),
                if run.failed_count > 0 {
                    run.failed_count.to_string().red()
                } else {
                    run.failed_count.to_string().normal()
                }
            );
        }
    }
}
```

## Implementation Steps

1. Create analytics module structure
2. Implement AnalyticsConfig parsing
3. Implement ToolMetrics and RunMetrics types
4. Implement AnalyticsCollector with timing
5. Implement AnalyticsStorage with JSON persistence
6. Integrate collector into setup command
7. Implement summary report generation
8. Implement history report
9. Implement tool-specific history
10. Implement failure analysis
11. Implement export (JSON, CSV)
12. Add cleanup and retention logic
13. Write tests for analytics
14. Update documentation

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Visibility into slow tools | None | Complete |
| Failure pattern detection | Manual | Automatic |
| Optimization prioritization | Guesswork | Data-driven |
| Time to troubleshoot issues | Hours | Minutes |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Disk space from analytics data | Medium | Low | Retention limits, cleanup |
| Performance overhead | Low | Medium | Async writes, minimal collection |
| Privacy concerns | Low | Low | Local only, no PII, no phone-home |
| Clock skew issues | Low | Low | Use monotonic timers for duration |

## Dependencies

### New Dependencies
- None required (uses existing dependencies)

### Existing Dependencies
- `chrono` - Timestamps
- `serde_json` - Storage format
- `colored` - Report formatting

## Effort Estimate

| Task | Effort |
|------|--------|
| Module structure and config | 0.5 days |
| Metrics types | 0.5 days |
| Collector implementation | 1 day |
| Storage layer | 1 day |
| Setup integration | 0.5 days |
| Summary report | 0.5 days |
| History report | 0.5 days |
| Tool history | 0.5 days |
| Failure analysis | 0.5 days |
| Export functionality | 0.5 days |
| CLI commands | 0.5 days |
| Testing | 1 day |
| Documentation | 0.5 days |
| **Total** | **8 days** |

## Files to Create/Modify

### New Files
- `src/analytics/mod.rs`
- `src/analytics/config.rs`
- `src/analytics/collector.rs`
- `src/analytics/storage.rs`
- `src/analytics/reporter.rs`
- `src/analytics/aggregator.rs`
- `src/analytics/commands.rs`
- `tests/analytics_integration.rs`

### Modified Files
- `src/config.rs` - Add analytics config parsing
- `src/lib.rs` - Export analytics module
- `src/main.rs` - Add analytics subcommand
- `src/commands/setup_cmd.rs` - Integrate metrics collection
- `CLAUDE.md` - Document [analytics] section

---

*PRD-046 v1.0 | Per-Tool Performance Analytics | Priority: Low*
