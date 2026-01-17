//! Network Request Tracing (PRD-027 T6)
//!
//! Tracks all HTTP requests made during Jarvy operations with detailed timing.
//!
//! ## Features
//!
//! - Request/response timing breakdown (DNS, connect, TLS, transfer)
//! - Bandwidth tracking
//! - Domain aggregation for summary
//! - Export to JSON for analysis
//!
//! ## Usage
//!
//! ```bash
//! jarvy setup --trace-network          # Enable network tracing
//! jarvy setup --trace-network --network-log network.json
//! ```

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Network request timing breakdown
#[derive(Debug, Clone, Serialize)]
pub struct NetworkTiming {
    /// Request URL
    pub url: String,
    /// HTTP method
    pub method: String,
    /// Response status code
    pub status: u16,
    /// Total request duration
    pub duration: Duration,
    /// DNS lookup time (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_time: Option<Duration>,
    /// Connection time (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_time: Option<Duration>,
    /// TLS handshake time (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_time: Option<Duration>,
    /// Time to first byte (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttfb: Option<Duration>,
    /// Response body size in bytes
    pub bytes: u64,
    /// Transfer speed in bytes per second
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_bps: Option<f64>,
    /// Request headers (sanitized)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_headers: Option<HashMap<String, String>>,
    /// Response headers (sanitized)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_headers: Option<HashMap<String, String>>,
    /// Error message if request failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl NetworkTiming {
    /// Create a new network timing record
    pub fn new(url: &str, method: &str) -> Self {
        Self {
            url: url.to_string(),
            method: method.to_string(),
            status: 0,
            duration: Duration::ZERO,
            dns_time: None,
            connect_time: None,
            tls_time: None,
            ttfb: None,
            bytes: 0,
            speed_bps: None,
            request_headers: None,
            response_headers: None,
            error: None,
        }
    }

    /// Complete the timing with response details
    pub fn complete(&mut self, status: u16, duration: Duration, bytes: u64) {
        self.status = status;
        self.duration = duration;
        self.bytes = bytes;

        // Calculate transfer speed
        if duration.as_secs_f64() > 0.0 {
            self.speed_bps = Some(bytes as f64 / duration.as_secs_f64());
        }
    }

    /// Mark as failed with error message
    pub fn fail(&mut self, error: &str, duration: Duration) {
        self.error = Some(error.to_string());
        self.duration = duration;
    }
}

/// Domain-level statistics
#[derive(Debug, Clone, Serialize)]
pub struct DomainStats {
    /// Domain name
    pub domain: String,
    /// Number of requests
    pub request_count: usize,
    /// Total bytes transferred
    pub total_bytes: u64,
    /// Total time spent
    pub total_time: Duration,
    /// Average response time
    pub avg_time: Duration,
    /// Failed request count
    pub failures: usize,
}

/// Network trace collector
#[derive(Debug, Clone)]
pub struct NetworkTracer {
    /// Collected timings (thread-safe)
    timings: Arc<Mutex<Vec<NetworkTiming>>>,
    /// Whether tracing is enabled
    enabled: bool,
    /// Start time
    start: Instant,
}

impl Default for NetworkTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkTracer {
    /// Create a new network tracer
    pub fn new() -> Self {
        Self {
            timings: Arc::new(Mutex::new(Vec::new())),
            enabled: true,
            start: Instant::now(),
        }
    }

    /// Create a disabled tracer (no-op)
    pub fn disabled() -> Self {
        Self {
            timings: Arc::new(Mutex::new(Vec::new())),
            enabled: false,
            start: Instant::now(),
        }
    }

    /// Check if tracing is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record a completed network request
    pub fn record(&self, timing: NetworkTiming) {
        if !self.enabled {
            return;
        }

        if let Ok(mut timings) = self.timings.lock() {
            timings.push(timing);
        }
    }

    /// Start tracking a request (returns a guard that records on drop)
    pub fn start_request(&self, url: &str, method: &str) -> RequestGuard {
        RequestGuard {
            tracer: self.clone(),
            timing: NetworkTiming::new(url, method),
            start: Instant::now(),
        }
    }

    /// Get all recorded timings
    pub fn get_timings(&self) -> Vec<NetworkTiming> {
        self.timings.lock().map(|t| t.clone()).unwrap_or_default()
    }

    /// Generate a summary report
    pub fn summary(&self) -> NetworkSummary {
        let timings = self.get_timings();

        let total_requests = timings.len();
        let successful = timings
            .iter()
            .filter(|t| t.error.is_none() && t.status >= 200 && t.status < 400)
            .count();
        let failed = timings
            .iter()
            .filter(|t| t.error.is_some() || t.status >= 400)
            .count();
        let total_bytes: u64 = timings.iter().map(|t| t.bytes).sum();
        let total_time: Duration = timings.iter().map(|t| t.duration).sum();

        // Find slowest request
        let slowest = timings.iter().max_by_key(|t| t.duration).cloned();

        // Find largest download
        let largest = timings.iter().max_by_key(|t| t.bytes).cloned();

        // Aggregate by domain
        let domains = self.aggregate_by_domain(&timings);

        NetworkSummary {
            total_requests,
            successful,
            failed,
            total_bytes,
            total_time,
            wall_time: self.start.elapsed(),
            slowest,
            largest,
            domains,
        }
    }

    /// Aggregate timings by domain
    fn aggregate_by_domain(&self, timings: &[NetworkTiming]) -> Vec<DomainStats> {
        let mut domain_map: HashMap<String, (usize, u64, Duration, usize)> = HashMap::new();

        for timing in timings {
            let domain = extract_domain(&timing.url).unwrap_or_else(|| "unknown".to_string());
            let entry = domain_map
                .entry(domain)
                .or_insert((0, 0, Duration::ZERO, 0));
            entry.0 += 1; // request count
            entry.1 += timing.bytes; // total bytes
            entry.2 += timing.duration; // total time
            if timing.error.is_some() || timing.status >= 400 {
                entry.3 += 1; // failures
            }
        }

        let mut domains: Vec<DomainStats> = domain_map
            .into_iter()
            .map(|(domain, (count, bytes, time, failures))| {
                let avg_time = if count > 0 {
                    time / count as u32
                } else {
                    Duration::ZERO
                };
                DomainStats {
                    domain,
                    request_count: count,
                    total_bytes: bytes,
                    total_time: time,
                    avg_time,
                    failures,
                }
            })
            .collect();

        // Sort by total bytes (largest first)
        domains.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
        domains
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let report = NetworkTraceReport {
            timings: self.get_timings(),
            summary: self.summary(),
        };
        serde_json::to_string_pretty(&report)
    }

    /// Export to JSON file
    pub fn to_json_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

/// Request guard that records timing on drop
pub struct RequestGuard {
    tracer: NetworkTracer,
    timing: NetworkTiming,
    start: Instant,
}

impl RequestGuard {
    /// Complete the request successfully
    pub fn complete(mut self, status: u16, bytes: u64) {
        self.timing.complete(status, self.start.elapsed(), bytes);
        self.tracer.record(self.timing.clone());
    }

    /// Mark the request as failed
    pub fn fail(mut self, error: &str) {
        self.timing.fail(error, self.start.elapsed());
        self.tracer.record(self.timing.clone());
    }

    /// Set response headers
    pub fn set_response_headers(&mut self, headers: HashMap<String, String>) {
        self.timing.response_headers = Some(headers);
    }
}

/// Network trace summary
#[derive(Debug, Clone, Serialize)]
pub struct NetworkSummary {
    /// Total number of requests
    pub total_requests: usize,
    /// Successful requests (2xx/3xx)
    pub successful: usize,
    /// Failed requests (4xx/5xx or errors)
    pub failed: usize,
    /// Total bytes transferred
    pub total_bytes: u64,
    /// Total time for all requests
    pub total_time: Duration,
    /// Wall clock time since tracer creation
    pub wall_time: Duration,
    /// Slowest request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slowest: Option<NetworkTiming>,
    /// Largest download
    #[serde(skip_serializing_if = "Option::is_none")]
    pub largest: Option<NetworkTiming>,
    /// Per-domain statistics
    pub domains: Vec<DomainStats>,
}

impl NetworkSummary {
    /// Generate human-readable summary
    pub fn to_summary_string(&self) -> String {
        let mut output = String::new();

        output.push_str("Network Trace Summary\n");
        output.push_str("=====================\n\n");

        output.push_str(&format!("Total requests:    {}\n", self.total_requests));
        output.push_str(&format!("Successful:        {}\n", self.successful));
        output.push_str(&format!("Failed:            {}\n", self.failed));
        output.push_str(&format!(
            "Total data:        {:.2} MB\n",
            self.total_bytes as f64 / 1_000_000.0
        ));
        output.push_str(&format!(
            "Total time:        {:.2}s\n",
            self.total_time.as_secs_f64()
        ));
        output.push('\n');

        if let Some(ref slowest) = self.slowest {
            output.push_str(&format!(
                "Slowest request:   {} ({:.2}s)\n",
                truncate_url(&slowest.url, 50),
                slowest.duration.as_secs_f64()
            ));
        }

        if let Some(ref largest) = self.largest {
            output.push_str(&format!(
                "Largest download:  {} ({:.2} MB)\n",
                truncate_url(&largest.url, 50),
                largest.bytes as f64 / 1_000_000.0
            ));
        }

        if !self.domains.is_empty() {
            output.push_str("\nDomains contacted:\n");
            for domain in &self.domains {
                output.push_str(&format!(
                    "  {:30} {} requests, {:.2} MB\n",
                    domain.domain,
                    domain.request_count,
                    domain.total_bytes as f64 / 1_000_000.0
                ));
            }
        }

        output
    }
}

/// Complete network trace report
#[derive(Debug, Clone, Serialize)]
pub struct NetworkTraceReport {
    /// All recorded timings
    pub timings: Vec<NetworkTiming>,
    /// Summary statistics
    pub summary: NetworkSummary,
}

/// Extract domain from URL
fn extract_domain(url: &str) -> Option<String> {
    // Simple domain extraction without pulling in url crate
    let url = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    url.split('/').next().map(|s| s.to_string())
}

/// Truncate URL for display
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_string()
    } else {
        format!("{}...", &url[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_timing_creation() {
        let timing = NetworkTiming::new("https://example.com/file.tar.gz", "GET");
        assert_eq!(timing.url, "https://example.com/file.tar.gz");
        assert_eq!(timing.method, "GET");
        assert_eq!(timing.status, 0);
    }

    #[test]
    fn test_network_timing_complete() {
        let mut timing = NetworkTiming::new("https://example.com/file.tar.gz", "GET");
        timing.complete(200, Duration::from_secs(5), 100_000_000);

        assert_eq!(timing.status, 200);
        assert_eq!(timing.bytes, 100_000_000);
        assert!(timing.speed_bps.is_some());
        assert!((timing.speed_bps.unwrap() - 20_000_000.0).abs() < 1.0);
    }

    #[test]
    fn test_network_tracer() {
        let tracer = NetworkTracer::new();

        let mut timing = NetworkTiming::new("https://example.com/test", "GET");
        timing.complete(200, Duration::from_millis(100), 1000);
        tracer.record(timing);

        let timings = tracer.get_timings();
        assert_eq!(timings.len(), 1);
        assert_eq!(timings[0].status, 200);
    }

    #[test]
    fn test_network_tracer_disabled() {
        let tracer = NetworkTracer::disabled();
        assert!(!tracer.is_enabled());

        let timing = NetworkTiming::new("https://example.com/test", "GET");
        tracer.record(timing);

        let timings = tracer.get_timings();
        assert!(timings.is_empty());
    }

    #[test]
    fn test_network_summary() {
        let tracer = NetworkTracer::new();

        let mut t1 = NetworkTiming::new("https://example.com/a", "GET");
        t1.complete(200, Duration::from_secs(1), 1000);
        tracer.record(t1);

        let mut t2 = NetworkTiming::new("https://example.com/b", "GET");
        t2.complete(200, Duration::from_secs(2), 2000);
        tracer.record(t2);

        let summary = tracer.summary();
        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.successful, 2);
        assert_eq!(summary.total_bytes, 3000);
    }

    #[test]
    fn test_domain_aggregation() {
        let tracer = NetworkTracer::new();

        let mut t1 = NetworkTiming::new("https://example.com/a", "GET");
        t1.complete(200, Duration::from_secs(1), 1000);
        tracer.record(t1);

        let mut t2 = NetworkTiming::new("https://other.com/b", "GET");
        t2.complete(200, Duration::from_secs(2), 5000);
        tracer.record(t2);

        let summary = tracer.summary();
        assert_eq!(summary.domains.len(), 2);
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://example.com/path/file"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain("http://test.org:8080/api"),
            Some("test.org:8080".to_string())
        );
    }

    #[test]
    fn test_truncate_url() {
        assert_eq!(truncate_url("short", 10), "short");
        assert_eq!(truncate_url("this is a very long url", 10), "this is...");
    }

    #[test]
    fn test_summary_string() {
        let tracer = NetworkTracer::new();
        let mut timing = NetworkTiming::new("https://example.com/test", "GET");
        timing.complete(200, Duration::from_millis(100), 1000);
        tracer.record(timing);

        let summary = tracer.summary();
        let output = summary.to_summary_string();

        assert!(output.contains("Network Trace Summary"));
        assert!(output.contains("Total requests:"));
        assert!(output.contains("example.com"));
    }
}
