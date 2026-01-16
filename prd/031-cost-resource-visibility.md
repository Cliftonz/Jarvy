# PRD-031: Cost & Resource Visibility

## Overview

This PRD defines resource tracking and cost visibility features for Jarvy, enabling users to understand bandwidth consumption, disk space usage, download times, and receive optimization recommendations.

## Problem Statement

CI/CD costs scale with tool installations, but users have no visibility into what resources Jarvy consumes. Without data on bandwidth, disk space, and download times, optimization is impossible. Teams running hundreds of CI jobs per day may be downloading gigabytes of tools unnecessarily.

## Evidence

- CI pipelines reinstall tools on every run without caching insights
- No way to identify which tools consume the most resources
- Users on metered connections lack bandwidth control
- Large organizations can't attribute infrastructure costs to specific tools
- No metrics exist for optimizing tool selection or caching strategies

## Requirements

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-1 | Track bandwidth usage per tool per installation | P0 |
| FR-2 | Report disk space consumption per tool | P0 |
| FR-3 | Record download duration and speed metrics | P0 |
| FR-4 | Provide resource usage summary command | P0 |
| FR-5 | Generate optimization recommendations | P1 |
| FR-6 | Support "light mode" for bandwidth-constrained environments | P1 |
| FR-7 | Export metrics in machine-readable formats | P1 |
| FR-8 | Track cumulative usage over time | P2 |
| FR-9 | Integrate with CI cost tracking systems | P2 |
| FR-10 | Provide real-time download progress with bandwidth | P2 |

### Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-1 | Metrics collection overhead | < 1% performance impact |
| NFR-2 | Storage for metrics database | < 10MB typical usage |
| NFR-3 | Metrics query response time | < 100ms |
| NFR-4 | Historical data retention | Configurable, default 90 days |

## Non-Goals

- Real-time cost calculation with cloud provider APIs
- Financial billing system integration
- Network packet-level analysis
- Multi-tenant cost allocation
- Predictive cost modeling with ML

## Feature Specification

### Resource Usage Command

```bash
# Show resource usage summary
jarvy resources

# Output:
# Resource Usage Summary
# ═══════════════════════════════════════════════════════════════════════
#
# Disk Space Usage (Total: 2.3 GB)
# ────────────────────────────────────────────────────────────────────────
#   rust (rustup)          892 MB  ████████████████████████████████▌  38.8%
#   node (nvm)             456 MB  ████████████████▊                  19.8%
#   docker                 312 MB  ███████████▍                       13.6%
#   go                     234 MB  ████████▌                          10.2%
#   python (pyenv)         198 MB  ███████▎                            8.6%
#   other (12 tools)       208 MB  ███████▋                            9.0%
#
# Network Usage (Last 30 days)
# ────────────────────────────────────────────────────────────────────────
#   Total Downloaded:      4.2 GB
#   Avg per install:       127 MB
#   Cache hit rate:        67.3%
#   Bandwidth saved:       8.6 GB (from caching)
#
# Top Bandwidth Consumers
# ────────────────────────────────────────────────────────────────────────
#   1. rust (rustup)       1.8 GB  (3 installs, 600 MB avg)
#   2. docker              890 MB  (2 installs, 445 MB avg)
#   3. node (nvm)          654 MB  (5 installs, 131 MB avg)

# Detailed view for a specific tool
jarvy resources rust

# Output:
# Resource Details: rust
# ═══════════════════════════════════════════════════════════════════════
#
# Current Installation
# ────────────────────────────────────────────────────────────────────────
#   Version:               1.75.0
#   Install date:          2024-01-10 14:23:05
#   Disk usage:            892 MB
#   Components:            rustc, cargo, rust-std, rust-docs, clippy, rustfmt
#
# Installation History
# ────────────────────────────────────────────────────────────────────────
#   Date                   Version    Downloaded    Duration    Source
#   2024-01-10 14:23:05    1.75.0     623 MB        2m 14s      rustup.rs
#   2024-01-03 09:15:22    1.74.1     598 MB        1m 52s      rustup.rs (cached)
#   2023-12-15 11:42:18    1.74.0     612 MB        3m 05s      rustup.rs
#
# Disk Breakdown
# ────────────────────────────────────────────────────────────────────────
#   ~/.rustup/toolchains/  756 MB
#   ~/.cargo/bin/           89 MB
#   ~/.cargo/registry/      47 MB

# Filter by time period
jarvy resources --period 7d
jarvy resources --period 30d
jarvy resources --since 2024-01-01

# Export metrics
jarvy resources --format json > resources.json
jarvy resources --format csv > resources.csv
```

### Bandwidth Tracking

```bash
# Show bandwidth usage
jarvy bandwidth

# Output:
# Bandwidth Usage
# ═══════════════════════════════════════════════════════════════════════
#
# Summary (Last 30 days)
# ────────────────────────────────────────────────────────────────────────
#   Total downloaded:      4.2 GB
#   Total uploaded:        0 B (tools don't upload)
#   Unique downloads:      1.4 GB (33%)
#   From cache:            2.8 GB (67%)
#
# Daily Breakdown
# ────────────────────────────────────────────────────────────────────────
#   Date          Downloaded    Cache Hits    Tools Installed
#   2024-01-15    892 MB        2             rust, docker
#   2024-01-14    45 MB         5             jq, ripgrep, fd (cached)
#   2024-01-12    234 MB        1             node
#   ...
#
# By Source
# ────────────────────────────────────────────────────────────────────────
#   Source                 Downloaded    Requests    Avg Speed
#   github.com             1.8 GB        45          12.3 MB/s
#   rustup.rs              1.2 GB        3           8.7 MB/s
#   nodejs.org             456 MB        5           15.2 MB/s
#   homebrew               398 MB        28          18.9 MB/s
#   other                  352 MB        19          9.4 MB/s

# Track a specific installation
jarvy setup rust --verbose
# Output includes:
#   Downloading rustup-init... 8.2 MB @ 12.4 MB/s (0.7s)
#   Downloading rust-1.75.0... 615 MB @ 9.8 MB/s (62.8s)
#   Installing components... (no network)
#   Total: 623.2 MB downloaded in 63.5s
```

### Disk Space Management

```bash
# Show disk space usage
jarvy disk

# Output:
# Disk Space Usage
# ═══════════════════════════════════════════════════════════════════════
#
# By Tool (Total: 2.3 GB)
# ────────────────────────────────────────────────────────────────────────
#   Tool                   Size        Location
#   rust                   892 MB      ~/.rustup, ~/.cargo
#   node                   456 MB      ~/.nvm
#   docker                 312 MB      /usr/local/bin, ~/.docker
#   go                     234 MB      ~/go
#   python                 198 MB      ~/.pyenv
#   ...
#
# By Location
# ────────────────────────────────────────────────────────────────────────
#   Location               Size        Tools
#   ~/.rustup              756 MB      rust
#   ~/.nvm                 456 MB      node
#   ~/.cargo               136 MB      rust
#   /usr/local/bin         98 MB       various
#   ~/.pyenv               198 MB      python
#
# Cache Storage
# ────────────────────────────────────────────────────────────────────────
#   Jarvy cache:           156 MB      ~/.jarvy/cache
#   Homebrew cache:        1.2 GB      ~/Library/Caches/Homebrew
#   npm cache:             234 MB      ~/.npm/_cacache

# Clean up disk space
jarvy disk clean

# Output:
# Disk Cleanup
# ═══════════════════════════════════════════════════════════════════════
#
# Cleanable Items
# ────────────────────────────────────────────────────────────────────────
#   [x] Old Rust toolchains (1.73.0, 1.72.0)     412 MB
#   [x] Unused Node versions (18.0.0, 16.0.0)    234 MB
#   [x] Jarvy download cache                      156 MB
#   [ ] Homebrew cache                           1.2 GB
#   [ ] npm cache                                 234 MB
#
# Selected: 802 MB
#
# Proceed with cleanup? [y/N]

# Automatic cleanup with threshold
jarvy disk clean --auto --keep-latest 2
jarvy disk clean --older-than 30d
```

### Optimization Recommendations

```bash
# Get optimization recommendations
jarvy optimize

# Output:
# Optimization Recommendations
# ═══════════════════════════════════════════════════════════════════════
#
# High Impact (Est. savings: 1.8 GB bandwidth, 450 MB disk)
# ────────────────────────────────────────────────────────────────────────
#
# 1. Enable persistent caching for CI
#    Your CI reinstalls tools on every run. Add caching:
#
#    # GitHub Actions
#    - uses: actions/cache@v3
#      with:
#        path: ~/.jarvy/cache
#        key: jarvy-${{ hashFiles('jarvy.toml') }}
#
#    Estimated savings: 1.2 GB/month
#
# 2. Remove unused Rust toolchains
#    You have 3 old toolchains (1.71, 1.72, 1.73) that haven't been used
#    in 45+ days. Run: jarvy disk clean --tool rust
#
#    Disk savings: 450 MB
#
# Medium Impact
# ────────────────────────────────────────────────────────────────────────
#
# 3. Use minimal Node installation
#    You're downloading full Node distributions. Consider:
#
#    [tools.node]
#    version = "20"
#    minimal = true  # Excludes npm docs, reduces size by ~40%
#
#    Estimated savings: 180 MB per install
#
# 4. Pin exact versions
#    Using "latest" causes re-downloads. Pin versions:
#
#    rust = "1.75.0"  # Instead of "latest"
#
#    Estimated savings: 400 MB/month
#
# Low Impact
# ────────────────────────────────────────────────────────────────────────
#
# 5. Enable compression for slow connections
#    Add to jarvy.toml:
#
#    [settings]
#    prefer_compressed = true

# Apply specific recommendations
jarvy optimize --apply 1
jarvy optimize --apply all
```

### Light Mode for Bandwidth-Constrained Environments

```bash
# Enable light mode globally
jarvy config set bandwidth.mode light

# Or in jarvy.toml
[settings]
bandwidth_mode = "light"  # "normal" | "light" | "minimal"

# Light mode behaviors:
# - Prefers smaller package variants when available
# - Skips optional components (docs, debug symbols)
# - Uses compressed downloads even if slower to decompress
# - Warns before large downloads (>100MB by default)
# - Suggests alternatives for bandwidth-heavy tools

# Setup with light mode
jarvy setup --light

# Output:
# Light Mode Active
# ═══════════════════════════════════════════════════════════════════════
#
# Installing tools with bandwidth optimization...
#
#   rust: Using minimal profile (saves ~300 MB)
#         Skipping: rust-docs, rust-src
#   node: Using binary-only distribution (saves ~80 MB)
#         Skipping: npm documentation
#   docker: Standard installation (no light variant)
#
# ⚠ Warning: docker requires 312 MB download
#   Continue? [y/N] or skip with --skip docker

# Set bandwidth limit
jarvy config set bandwidth.limit "5MB/s"
jarvy setup --bandwidth-limit 1MB/s
```

### CI Cost Integration

```bash
# Export metrics for CI cost tracking
jarvy resources --format prometheus > metrics.txt

# Output (Prometheus format):
# # HELP jarvy_tool_disk_bytes Disk space used by tool
# # TYPE jarvy_tool_disk_bytes gauge
# jarvy_tool_disk_bytes{tool="rust"} 935329792
# jarvy_tool_disk_bytes{tool="node"} 478150656
#
# # HELP jarvy_download_bytes_total Total bytes downloaded
# # TYPE jarvy_download_bytes_total counter
# jarvy_download_bytes_total{tool="rust"} 1932735283
# jarvy_download_bytes_total{tool="node"} 685768704
#
# # HELP jarvy_install_duration_seconds Installation duration
# # TYPE jarvy_install_duration_seconds histogram
# jarvy_install_duration_seconds_bucket{tool="rust",le="60"} 0
# jarvy_install_duration_seconds_bucket{tool="rust",le="120"} 2
# jarvy_install_duration_seconds_bucket{tool="rust",le="300"} 3

# OpenTelemetry export
jarvy resources --format otlp --endpoint http://collector:4317

# JSON for custom dashboards
jarvy resources --format json --period 30d | curl -X POST \
  -H "Content-Type: application/json" \
  -d @- https://metrics.example.com/jarvy
```

### Real-Time Progress Display

```bash
# Verbose installation with resource tracking
jarvy setup --verbose

# Output:
# Installing 5 tools...
# ═══════════════════════════════════════════════════════════════════════
#
# [1/5] rust
#   Downloading rustup-init.sh
#   ████████████████████████████████████████ 8.2 MB @ 12.4 MB/s (0.7s)
#
#   Downloading rust-1.75.0-x86_64-apple-darwin
#   ████████████████████░░░░░░░░░░░░░░░░░░░░ 312/615 MB @ 9.8 MB/s (ETA: 31s)
#
#   Network: 312 MB downloaded | Disk: 0 MB (pending)
#
# [2/5] node (queued)
# [3/5] go (queued)
# [4/5] docker (queued)
# [5/5] jq (queued)
#
# ───────────────────────────────────────────────────────────────────────
# Total: 312 MB / 1.8 GB | Elapsed: 32s | ETA: 2m 45s
```

### Configuration

```toml
# jarvy.toml - Resource tracking settings
[settings.resources]
# Enable resource tracking (default: true)
enabled = true

# Bandwidth mode: normal, light, minimal
bandwidth_mode = "normal"

# Bandwidth limit (optional)
bandwidth_limit = "10MB/s"

# Warn before large downloads
large_download_warning = "100MB"

# Historical data retention
metrics_retention_days = 90

# Cleanup settings
auto_cleanup = false
cleanup_keep_versions = 2
cleanup_older_than_days = 30

[settings.resources.export]
# Automatic metrics export
enabled = false
format = "prometheus"
endpoint = "http://localhost:9091/metrics/job/jarvy"
interval = "1h"
```

## Acceptance Criteria

### AC-1: Resource Usage Summary
- [ ] `jarvy resources` shows disk and bandwidth summary
- [ ] Per-tool breakdown includes size, install date, download history
- [ ] Cache hit rate is calculated and displayed
- [ ] Output is visually clear with progress bars

### AC-2: Bandwidth Tracking
- [ ] All downloads are tracked with size and duration
- [ ] Source URLs are recorded for attribution
- [ ] Cache hits vs unique downloads are distinguished
- [ ] Historical data is queryable by time period

### AC-3: Disk Space Management
- [ ] `jarvy disk` shows per-tool and per-location breakdown
- [ ] `jarvy disk clean` identifies cleanable items safely
- [ ] Automatic cleanup respects retention policies
- [ ] No user data is deleted without confirmation

### AC-4: Optimization Recommendations
- [ ] Recommendations are actionable with specific commands
- [ ] Estimated savings are provided
- [ ] CI-specific recommendations are included
- [ ] Recommendations can be applied automatically

### AC-5: Light Mode
- [ ] Light mode reduces download sizes by 30%+ where possible
- [ ] Tool functionality is preserved (no broken installs)
- [ ] Large download warnings are shown appropriately
- [ ] Bandwidth limits are enforced

### AC-6: Export and Integration
- [ ] JSON export includes all metrics
- [ ] Prometheus format is valid and scrapable
- [ ] OTLP export works with standard collectors
- [ ] CI cache recommendations are platform-specific

## Technical Approach

### Module Structure

```
src/
├── resources/
│   ├── mod.rs              # Module exports
│   ├── tracker.rs          # Resource tracking during operations
│   ├── metrics.rs          # Metrics storage and queries
│   ├── disk.rs             # Disk space analysis
│   ├── bandwidth.rs        # Bandwidth tracking and limits
│   ├── optimizer.rs        # Optimization recommendations
│   ├── export.rs           # Prometheus, JSON, OTLP export
│   └── display.rs          # Progress bars and summaries
├── commands/
│   ├── resources.rs        # `jarvy resources` command
│   ├── disk.rs             # `jarvy disk` command
│   ├── bandwidth.rs        # `jarvy bandwidth` command
│   └── optimize.rs         # `jarvy optimize` command
```

### Metrics Storage Schema

```rust
// src/resources/metrics.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallMetrics {
    pub tool: String,
    pub version: String,
    pub timestamp: OffsetDateTime,
    pub download_bytes: u64,
    pub download_duration_ms: u64,
    pub disk_bytes: u64,
    pub source_url: Option<String>,
    pub cache_hit: bool,
    pub components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub tool: String,
    pub total_bytes: u64,
    pub locations: Vec<DiskLocation>,
    pub last_used: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskLocation {
    pub path: PathBuf,
    pub bytes: u64,
    pub file_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthRecord {
    pub timestamp: OffsetDateTime,
    pub url: String,
    pub bytes: u64,
    pub duration_ms: u64,
    pub tool: String,
    pub cache_hit: bool,
}

pub struct MetricsStore {
    db_path: PathBuf,
}

impl MetricsStore {
    pub fn new() -> Result<Self, MetricsError> {
        let db_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("jarvy")
            .join("metrics.db");

        Ok(Self { db_path })
    }

    pub fn record_install(&self, metrics: InstallMetrics) -> Result<(), MetricsError> {
        // Append to SQLite database
        todo!()
    }

    pub fn query_bandwidth(&self, period: Period) -> Result<BandwidthSummary, MetricsError> {
        todo!()
    }

    pub fn query_disk_usage(&self) -> Result<Vec<DiskUsage>, MetricsError> {
        todo!()
    }

    pub fn get_optimization_recommendations(&self) -> Vec<Recommendation> {
        todo!()
    }
}
```

### Download Tracking Integration

```rust
// src/resources/tracker.rs

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt};

pub struct DownloadTracker {
    bytes_downloaded: Arc<AtomicU64>,
    start_time: std::time::Instant,
    total_size: Option<u64>,
    tool: String,
    url: String,
}

impl DownloadTracker {
    pub fn new(tool: &str, url: &str, total_size: Option<u64>) -> Self {
        Self {
            bytes_downloaded: Arc::new(AtomicU64::new(0)),
            start_time: std::time::Instant::now(),
            total_size,
            tool: tool.to_string(),
            url: url.to_string(),
        }
    }

    pub fn wrap_reader<R: AsyncRead + Unpin>(&self, reader: R) -> TrackedReader<R> {
        TrackedReader {
            inner: reader,
            bytes: self.bytes_downloaded.clone(),
        }
    }

    pub fn progress(&self) -> DownloadProgress {
        let bytes = self.bytes_downloaded.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();
        let speed = if elapsed.as_secs() > 0 {
            bytes / elapsed.as_secs()
        } else {
            0
        };

        DownloadProgress {
            bytes_downloaded: bytes,
            total_bytes: self.total_size,
            bytes_per_second: speed,
            elapsed,
        }
    }

    pub fn finish(&self) -> InstallMetrics {
        let elapsed = self.start_time.elapsed();
        InstallMetrics {
            tool: self.tool.clone(),
            version: String::new(), // Set by caller
            timestamp: time::OffsetDateTime::now_utc(),
            download_bytes: self.bytes_downloaded.load(Ordering::Relaxed),
            download_duration_ms: elapsed.as_millis() as u64,
            disk_bytes: 0, // Calculated separately
            source_url: Some(self.url.clone()),
            cache_hit: false,
            components: vec![],
        }
    }
}

pub struct TrackedReader<R> {
    inner: R,
    bytes: Arc<AtomicU64>,
}

impl<R: AsyncRead + Unpin> AsyncRead for TrackedReader<R> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let result = std::pin::Pin::new(&mut self.inner).poll_read(cx, buf);
        let after = buf.filled().len();
        self.bytes.fetch_add((after - before) as u64, Ordering::Relaxed);
        result
    }
}
```

### Disk Space Analysis

```rust
// src/resources/disk.rs

use std::path::PathBuf;
use walkdir::WalkDir;

pub struct DiskAnalyzer;

impl DiskAnalyzer {
    /// Get disk usage for a specific tool
    pub fn analyze_tool(tool: &str) -> Result<DiskUsage, DiskError> {
        let locations = Self::get_tool_locations(tool);
        let mut usage = DiskUsage {
            tool: tool.to_string(),
            total_bytes: 0,
            locations: vec![],
            last_used: None,
        };

        for path in locations {
            if path.exists() {
                let (bytes, count) = Self::directory_size(&path)?;
                usage.total_bytes += bytes;
                usage.locations.push(DiskLocation {
                    path,
                    bytes,
                    file_count: count,
                });
            }
        }

        Ok(usage)
    }

    fn get_tool_locations(tool: &str) -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_default();

        match tool {
            "rust" => vec![
                home.join(".rustup"),
                home.join(".cargo"),
            ],
            "node" => vec![
                home.join(".nvm"),
                home.join(".npm"),
            ],
            "python" => vec![
                home.join(".pyenv"),
            ],
            "go" => vec![
                home.join("go"),
            ],
            _ => vec![],
        }
    }

    fn directory_size(path: &PathBuf) -> Result<(u64, u32), DiskError> {
        let mut total = 0u64;
        let mut count = 0u32;

        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
                count += 1;
            }
        }

        Ok((total, count))
    }

    /// Identify cleanable items
    pub fn find_cleanable(tool: &str, keep_versions: usize) -> Vec<CleanableItem> {
        // Tool-specific logic to find old versions, caches, etc.
        todo!()
    }
}
```

### Light Mode Implementation

```rust
// src/resources/bandwidth.rs

use crate::config::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandwidthMode {
    Normal,
    Light,
    Minimal,
}

impl BandwidthMode {
    pub fn from_settings(settings: &Settings) -> Self {
        match settings.bandwidth_mode.as_deref() {
            Some("light") => Self::Light,
            Some("minimal") => Self::Minimal,
            _ => Self::Normal,
        }
    }

    pub fn tool_profile(&self, tool: &str) -> ToolProfile {
        match self {
            Self::Normal => ToolProfile::Full,
            Self::Light => Self::light_profile(tool),
            Self::Minimal => Self::minimal_profile(tool),
        }
    }

    fn light_profile(tool: &str) -> ToolProfile {
        match tool {
            "rust" => ToolProfile::Custom {
                components: vec!["rustc", "cargo", "clippy", "rustfmt"],
                skip: vec!["rust-docs", "rust-src"],
            },
            "node" => ToolProfile::Custom {
                components: vec!["node", "npm"],
                skip: vec!["docs"],
            },
            _ => ToolProfile::Full,
        }
    }

    fn minimal_profile(tool: &str) -> ToolProfile {
        match tool {
            "rust" => ToolProfile::Custom {
                components: vec!["rustc", "cargo"],
                skip: vec!["rust-docs", "rust-src", "clippy", "rustfmt"],
            },
            "node" => ToolProfile::Custom {
                components: vec!["node"],
                skip: vec!["npm", "docs"],
            },
            _ => ToolProfile::Full,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ToolProfile {
    Full,
    Custom {
        components: Vec<&'static str>,
        skip: Vec<&'static str>,
    },
}
```

## Implementation Steps

### Phase 1: Core Metrics Collection (Week 1-2)
1. Create metrics storage schema with SQLite
2. Implement download tracking wrapper
3. Integrate tracking into existing install functions
4. Add disk space analysis utilities

### Phase 2: Resource Commands (Week 3-4)
5. Implement `jarvy resources` command
6. Implement `jarvy disk` command
7. Implement `jarvy bandwidth` command
8. Add progress display with resource info

### Phase 3: Optimization (Week 5-6)
9. Implement optimization recommendation engine
10. Implement `jarvy optimize` command
11. Add `jarvy disk clean` functionality
12. Implement light mode for tools

### Phase 4: Export & Integration (Week 7-8)
13. Add JSON export format
14. Add Prometheus export format
15. Add OTLP export support
16. Implement CI-specific recommendations
17. Add configuration options
18. Write documentation

## Dependencies

- **Internal**: Core installation functions, config system
- **PRD-027**: Observability integration for metrics export
- **External**: `walkdir` for disk analysis, `indicatif` for progress bars

## Effort Estimate

| Phase | Tasks | Days |
|-------|-------|------|
| Design & Schema | Metrics model, storage design | 2 |
| Core Tracking | Download/disk tracking implementation | 5 |
| Commands | resources, disk, bandwidth commands | 4 |
| Optimization | Recommendations engine, cleanup | 4 |
| Light Mode | Bandwidth-constrained profiles | 3 |
| Export | JSON, Prometheus, OTLP | 3 |
| Testing | Unit, integration, CI testing | 3 |
| Documentation | User guide, API docs | 2 |
| **Total** | | **26 days** |

## Files to Create/Modify

### New Files
- `src/resources/mod.rs`
- `src/resources/tracker.rs`
- `src/resources/metrics.rs`
- `src/resources/disk.rs`
- `src/resources/bandwidth.rs`
- `src/resources/optimizer.rs`
- `src/resources/export.rs`
- `src/resources/display.rs`
- `src/commands/resources.rs`
- `src/commands/disk.rs`
- `src/commands/bandwidth.rs`
- `src/commands/optimize.rs`
- `tests/resources_test.rs`
- `docs/resource-visibility.md`

### Modified Files
- `src/main.rs` - Add new commands
- `src/config.rs` - Add resource settings
- `src/tools/common.rs` - Integrate download tracking
- `Cargo.toml` - Add dependencies (walkdir, indicatif)

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Metrics accuracy | 99% | Verify tracked bytes match actual |
| Disk analysis coverage | 95% of tools | Count tools with location mapping |
| Optimization savings | 30% for light mode | Compare download sizes |
| CI cache hit rate | 70%+ with recommendations | Track before/after |
| Command response time | < 500ms | Benchmark queries |
| Export compatibility | 100% | Validate with Prometheus/OTLP |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Tracking overhead | Low | Medium | Async, non-blocking tracking |
| Disk scan performance | Medium | Low | Parallel scanning, caching |
| Inaccurate tool locations | Medium | Medium | User-configurable paths |
| SQLite concurrency | Low | Low | WAL mode, connection pooling |
| Light mode breakage | Medium | High | Extensive testing per tool |

---

*PRD-031 v1.0 | Cost & Resource Visibility | Priority: Low*
