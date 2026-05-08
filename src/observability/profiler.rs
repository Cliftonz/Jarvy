//! Performance Profiler
//!
//! Tracks timing of all operations with phase breakdown and per-tool statistics.
//!
//! ## Usage (crate-internal)
//!
//! ```ignore
//! let mut profiler = Profiler::new();
//! profiler.start_phase("config_parsing");
//! // ... do work ...
//! profiler.end_phase();
//!
//! profiler.start_tool("git");
//! // ... install git ...
//! profiler.end_tool(true);
//!
//! let report = profiler.report();
//! println!("{}", report.to_summary());
//! ```

#![allow(dead_code)] // Public API for performance profiling

use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Phase timing information
#[derive(Debug, Clone, Serialize)]
pub struct PhaseTiming {
    /// Duration of the phase
    pub duration: Duration,
    /// Start time (not serialized)
    #[serde(skip)]
    pub start: Option<Instant>,
}

impl Default for PhaseTiming {
    fn default() -> Self {
        Self {
            duration: Duration::ZERO,
            start: None,
        }
    }
}

/// Tool timing information
#[derive(Debug, Clone, Serialize)]
pub struct ToolTiming {
    /// Tool name
    pub name: String,
    /// Duration of installation
    pub duration: Duration,
    /// Whether installation succeeded
    pub success: bool,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Network request timing
#[derive(Debug, Clone, Serialize)]
pub struct NetworkTiming {
    /// Request URL
    pub url: String,
    /// HTTP method
    pub method: String,
    /// Response status code
    pub status: u16,
    /// Duration of request
    pub duration: Duration,
    /// Response size in bytes
    pub bytes: u64,
}

/// Performance profiler for tracking operation timing
#[derive(Debug)]
pub struct Profiler {
    /// Overall start time
    start: Instant,
    /// Phase timings by name
    phases: HashMap<String, PhaseTiming>,
    /// Current active phase
    current_phase: Option<String>,
    /// Tool installation timings
    tools: Vec<ToolTiming>,
    /// Current tool being installed
    current_tool: Option<(String, Instant)>,
    /// Network request timings
    network_requests: Vec<NetworkTiming>,
    /// Whether profiling is enabled
    enabled: bool,
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Profiler {
    /// Create a new profiler
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            phases: HashMap::new(),
            current_phase: None,
            tools: Vec::new(),
            current_tool: None,
            network_requests: Vec::new(),
            enabled: true,
        }
    }

    /// Create a disabled profiler (no-op)
    pub fn disabled() -> Self {
        let mut profiler = Self::new();
        profiler.enabled = false;
        profiler
    }

    /// Check if profiling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Start timing a phase
    pub fn start_phase(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        // End current phase if any
        self.end_phase();

        self.current_phase = Some(name.to_string());
        self.phases.insert(
            name.to_string(),
            PhaseTiming {
                duration: Duration::ZERO,
                start: Some(Instant::now()),
            },
        );
    }

    /// End the current phase
    pub fn end_phase(&mut self) {
        if !self.enabled {
            return;
        }

        if let Some(name) = self.current_phase.take() {
            if let Some(phase) = self.phases.get_mut(&name) {
                if let Some(start) = phase.start.take() {
                    phase.duration = start.elapsed();
                }
            }
        }
    }

    /// Start timing a tool installation
    pub fn start_tool(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        self.current_tool = Some((name.to_string(), Instant::now()));
    }

    /// End tool installation timing
    pub fn end_tool(&mut self, success: bool) {
        self.end_tool_with_error(success, None);
    }

    /// End tool installation timing with optional error
    pub fn end_tool_with_error(&mut self, success: bool, error: Option<String>) {
        if !self.enabled {
            return;
        }

        if let Some((name, start)) = self.current_tool.take() {
            self.tools.push(ToolTiming {
                name,
                duration: start.elapsed(),
                success,
                error,
            });
        }
    }

    /// Record a network request
    pub fn record_network(&mut self, timing: NetworkTiming) {
        if !self.enabled {
            return;
        }

        self.network_requests.push(timing);
    }

    /// Get total elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Generate a profile report
    pub fn report(&self) -> ProfileReport {
        let total_duration = self.start.elapsed();

        // Calculate phase breakdown
        let phase_breakdown: HashMap<String, PhaseTiming> = self.phases.clone();

        // Calculate network summary
        let network_summary = NetworkSummary {
            total_requests: self.network_requests.len(),
            total_bytes: self.network_requests.iter().map(|r| r.bytes).sum(),
            total_time: self.network_requests.iter().map(|r| r.duration).sum(),
            slowest: self
                .network_requests
                .iter()
                .max_by_key(|r| r.duration)
                .cloned(),
        };

        // Generate recommendations
        let recommendations = self.generate_recommendations();

        ProfileReport {
            total_duration,
            phases: phase_breakdown,
            tools: self.tools.clone(),
            network: network_summary,
            recommendations,
        }
    }

    /// Generate optimization recommendations
    fn generate_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        let total = self.start.elapsed();

        // Check for slow tools
        for tool in &self.tools {
            let percentage = (tool.duration.as_secs_f64() / total.as_secs_f64()) * 100.0;
            if percentage > 40.0 {
                recommendations.push(format!(
                    "{} took {:.0}% of total time ({:.1}s). Consider pre-caching or using a local mirror.",
                    tool.name,
                    percentage,
                    tool.duration.as_secs_f64()
                ));
            }
        }

        // Check for slow network requests
        for req in &self.network_requests {
            if req.duration.as_secs() > 10 && req.bytes > 100_000_000 {
                recommendations.push(format!(
                    "Large download: {} ({:.1} MB, {:.1}s). Consider caching.",
                    req.url,
                    req.bytes as f64 / 1_000_000.0,
                    req.duration.as_secs_f64()
                ));
            }
        }

        // Check for failed tools
        let failed_count = self.tools.iter().filter(|t| !t.success).count();
        if failed_count > 0 {
            recommendations.push(format!(
                "{} tool(s) failed to install. Run 'jarvy diagnose' for details.",
                failed_count
            ));
        }

        recommendations
    }
}

/// Network request summary
#[derive(Debug, Clone, Serialize)]
pub struct NetworkSummary {
    /// Total number of requests
    pub total_requests: usize,
    /// Total bytes transferred
    pub total_bytes: u64,
    /// Total time for all requests
    pub total_time: Duration,
    /// Slowest request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slowest: Option<NetworkTiming>,
}

/// Profile report with all timing information
#[derive(Debug, Clone, Serialize)]
pub struct ProfileReport {
    /// Total duration of profiled operations
    pub total_duration: Duration,
    /// Phase breakdown
    pub phases: HashMap<String, PhaseTiming>,
    /// Tool installation timings
    pub tools: Vec<ToolTiming>,
    /// Network summary
    pub network: NetworkSummary,
    /// Optimization recommendations
    pub recommendations: Vec<String>,
}

impl ProfileReport {
    /// Generate a human-readable summary
    pub fn to_summary(&self) -> String {
        let mut output = String::new();

        output.push_str("══════════════════════════════════════════════════════════\n");
        output.push_str("Performance Profile\n");
        output.push_str("══════════════════════════════════════════════════════════\n\n");

        output.push_str(&format!(
            "Total duration: {:.2}s\n\n",
            self.total_duration.as_secs_f64()
        ));

        // Phase breakdown
        if !self.phases.is_empty() {
            output.push_str("Phase breakdown:\n");
            let mut phases: Vec<_> = self.phases.iter().collect();
            phases.sort_by_key(|p| std::cmp::Reverse(p.1.duration));

            for (name, timing) in phases {
                let percentage =
                    (timing.duration.as_secs_f64() / self.total_duration.as_secs_f64()) * 100.0;
                output.push_str(&format!(
                    "  {:20} {:>6.2}s  ({:>5.1}%)\n",
                    name,
                    timing.duration.as_secs_f64(),
                    percentage
                ));
            }
            output.push('\n');
        }

        // Tool installation times
        if !self.tools.is_empty() {
            output.push_str("Tool installation times:\n");
            let mut tools = self.tools.clone();
            tools.sort_by_key(|t| std::cmp::Reverse(t.duration));

            let max_duration = tools
                .first()
                .map(|t| t.duration.as_secs_f64())
                .unwrap_or(1.0);

            for tool in tools {
                let bar_width = ((tool.duration.as_secs_f64() / max_duration) * 40.0) as usize;
                let bar = "█".repeat(bar_width.max(1));
                let status = if tool.success { "" } else { " ✗" };
                output.push_str(&format!(
                    "  {:12} {:>6.2}s  {}{}\n",
                    tool.name,
                    tool.duration.as_secs_f64(),
                    bar,
                    status
                ));
            }
            output.push('\n');
        }

        // Network summary
        if self.network.total_requests > 0 {
            output.push_str("Network requests:\n");
            output.push_str(&format!(
                "  Total requests: {}\n",
                self.network.total_requests
            ));
            output.push_str(&format!(
                "  Total downloaded: {:.1} MB\n",
                self.network.total_bytes as f64 / 1_000_000.0
            ));
            if let Some(ref slowest) = self.network.slowest {
                output.push_str(&format!(
                    "  Slowest: {} ({:.1} MB, {:.1}s)\n",
                    slowest.url,
                    slowest.bytes as f64 / 1_000_000.0,
                    slowest.duration.as_secs_f64()
                ));
            }
            output.push('\n');
        }

        // Recommendations
        if !self.recommendations.is_empty() {
            output.push_str("Recommendations:\n");
            for (i, rec) in self.recommendations.iter().enumerate() {
                output.push_str(&format!("  {}. {}\n", i + 1, rec));
            }
        }

        output
    }

    /// Export as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Export as JSON to file
    pub fn to_json_file(&self, path: &str) -> Result<(), super::error::ObservabilityError> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_profiler_phases() {
        let mut profiler = Profiler::new();

        profiler.start_phase("test_phase");
        sleep(Duration::from_millis(10));
        profiler.end_phase();

        let report = profiler.report();
        assert!(report.phases.contains_key("test_phase"));
        assert!(report.phases["test_phase"].duration.as_millis() >= 10);
    }

    #[test]
    fn test_profiler_tools() {
        let mut profiler = Profiler::new();

        profiler.start_tool("git");
        sleep(Duration::from_millis(5));
        profiler.end_tool(true);

        profiler.start_tool("node");
        sleep(Duration::from_millis(5));
        profiler.end_tool_with_error(false, Some("Failed".to_string()));

        let report = profiler.report();
        assert_eq!(report.tools.len(), 2);
        assert!(report.tools[0].success);
        assert!(!report.tools[1].success);
    }

    #[test]
    fn test_profiler_disabled() {
        let mut profiler = Profiler::disabled();
        assert!(!profiler.is_enabled());

        profiler.start_phase("test");
        profiler.end_phase();
        profiler.start_tool("git");
        profiler.end_tool(true);

        let report = profiler.report();
        assert!(report.phases.is_empty());
        assert!(report.tools.is_empty());
    }

    #[test]
    fn test_profile_report_summary() {
        let mut profiler = Profiler::new();
        profiler.start_phase("config");
        profiler.end_phase();
        profiler.start_tool("git");
        profiler.end_tool(true);

        let report = profiler.report();
        let summary = report.to_summary();

        assert!(summary.contains("Performance Profile"));
        assert!(summary.contains("Total duration"));
    }

    #[test]
    fn test_profile_report_json() {
        let profiler = Profiler::new();
        let report = profiler.report();
        let json = report.to_json().unwrap();

        assert!(json.contains("total_duration"));
        assert!(json.contains("phases"));
        assert!(json.contains("tools"));
    }

    #[test]
    fn test_network_timing() {
        let mut profiler = Profiler::new();

        profiler.record_network(NetworkTiming {
            url: "https://example.com/file.tar.gz".to_string(),
            method: "GET".to_string(),
            status: 200,
            duration: Duration::from_secs(5),
            bytes: 100_000_000,
        });

        let report = profiler.report();
        assert_eq!(report.network.total_requests, 1);
        assert_eq!(report.network.total_bytes, 100_000_000);
    }
}
