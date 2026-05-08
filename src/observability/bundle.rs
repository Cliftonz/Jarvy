//! Diagnostic Bundle Export (PRD-027 T13)
//!
//! Creates shareable diagnostic bundles for support and troubleshooting.
//!
//! ## Features
//!
//! - System information collection
//! - Jarvy configuration (sanitized)
//! - Tool status snapshot
//! - Environment variables (sanitized)
//! - Network connectivity tests
//! - ZIP archive creation
//!
//! ## Usage
//!

#![allow(dead_code)] // Public API for diagnostic bundle export
//! ```bash
//! jarvy diagnose --export                    # Create diagnostic bundle
//! jarvy diagnose docker --export             # Tool-specific bundle
//! jarvy diagnose --export --scope tools,network
//! ```

use crate::observability::Sanitizer;
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

/// Scope of data to include in bundle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleScope {
    /// System information
    System,
    /// Jarvy configuration
    Config,
    /// Tool status
    Tools,
    /// Environment variables
    Environment,
    /// Network connectivity
    Network,
    /// All scopes
    All,
}

impl FromStr for BundleScope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "system" => Ok(BundleScope::System),
            "config" => Ok(BundleScope::Config),
            "tools" => Ok(BundleScope::Tools),
            "environment" | "env" => Ok(BundleScope::Environment),
            "network" => Ok(BundleScope::Network),
            "all" => Ok(BundleScope::All),
            _ => Err(format!("unknown bundle scope: {s}")),
        }
    }
}

impl BundleScope {
    /// Parse comma-separated scope list
    pub fn parse_list(s: &str) -> Vec<Self> {
        if s.is_empty() || s == "all" {
            return vec![BundleScope::All];
        }

        s.split(',')
            .filter_map(|part| part.trim().parse().ok())
            .collect()
    }
}

/// System information for the bundle
#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub hostname: String,
    pub shell: String,
    pub home_dir: String,
    pub current_dir: String,
    pub jarvy_version: String,
    pub rust_version: Option<String>,
}

impl SystemInfo {
    /// Collect system information
    pub fn collect() -> Self {
        let os = if cfg!(target_os = "macos") {
            "macOS".to_string()
        } else if cfg!(target_os = "linux") {
            "Linux".to_string()
        } else if cfg!(target_os = "windows") {
            "Windows".to_string()
        } else {
            std::env::consts::OS.to_string()
        };

        let os_version = get_os_version();

        let arch = if cfg!(target_arch = "x86_64") {
            "x86_64".to_string()
        } else if cfg!(target_arch = "aarch64") {
            "arm64".to_string()
        } else {
            std::env::consts::ARCH.to_string()
        };

        // Hostname: hash by default so org/project names cannot leak via
        // ticket bundles. Set JARVY_BUNDLE_INCLUDE_HOSTNAME=1 to include the
        // raw hostname (e.g., when the user is the maintainer themselves).
        let raw_hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let include_raw_hostname = matches!(
            std::env::var("JARVY_BUNDLE_INCLUDE_HOSTNAME").as_deref(),
            Ok("1") | Ok("true")
        );
        let hostname = if include_raw_hostname {
            raw_hostname
        } else {
            use sha2::{Digest, Sha256};
            let h = Sha256::digest(raw_hostname.as_bytes());
            format!("hashed-{}", &hex::encode(h)[..16])
        };

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string());

        // Paths: replace home prefix with `~` so usernames / project names
        // don't end up in the ZIP.
        let home_dir = dirs::home_dir()
            .map(|p| crate::network::redact_home(&p.to_string_lossy()))
            .unwrap_or_else(|| "unknown".to_string());

        let current_dir = std::env::current_dir()
            .map(|p| crate::network::redact_home(&p.to_string_lossy()))
            .unwrap_or_else(|_| "unknown".to_string());

        let jarvy_version = env!("CARGO_PKG_VERSION").to_string();

        let rust_version = std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        Self {
            os,
            os_version,
            arch,
            hostname,
            shell,
            home_dir,
            current_dir,
            jarvy_version,
            rust_version,
        }
    }
}

/// Tool status information
#[derive(Debug, Clone, Serialize)]
pub struct ToolStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub method: Option<String>,
}

/// Environment variable (sanitized)
#[derive(Debug, Clone, Serialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

/// Network connectivity test result
#[derive(Debug, Clone, Serialize)]
pub struct ConnectivityTest {
    pub target: String,
    pub reachable: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Complete diagnostic bundle
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticBundle {
    /// Bundle creation timestamp
    pub created_at: String,
    /// System information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemInfo>,
    /// Configuration (sanitized)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    /// Tool statuses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolStatus>>,
    /// Environment variables (sanitized)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<EnvVar>>,
    /// Network connectivity tests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Vec<ConnectivityTest>>,
    /// Included scopes
    pub scopes: Vec<String>,
}

impl DiagnosticBundle {
    /// Create a new diagnostic bundle
    pub fn new() -> Self {
        Self {
            created_at: timestamp(),
            system: None,
            config: None,
            tools: None,
            environment: None,
            network: None,
            scopes: Vec::new(),
        }
    }

    /// Collect bundle data for specified scopes
    pub fn collect(scopes: &[BundleScope], config_path: Option<&str>) -> Self {
        let mut bundle = Self::new();
        let sanitizer = Sanitizer::new();

        let include_all = scopes.contains(&BundleScope::All);

        // System info
        if include_all || scopes.contains(&BundleScope::System) {
            bundle.system = Some(SystemInfo::collect());
            bundle.scopes.push("system".to_string());
        }

        // Config
        if include_all || scopes.contains(&BundleScope::Config) {
            bundle.config = collect_config(config_path, &sanitizer);
            bundle.scopes.push("config".to_string());
        }

        // Tools
        if include_all || scopes.contains(&BundleScope::Tools) {
            bundle.tools = Some(collect_tool_status());
            bundle.scopes.push("tools".to_string());
        }

        // Environment
        if include_all || scopes.contains(&BundleScope::Environment) {
            bundle.environment = Some(collect_environment(&sanitizer));
            bundle.scopes.push("environment".to_string());
        }

        // Network
        if include_all || scopes.contains(&BundleScope::Network) {
            bundle.network = Some(run_connectivity_tests());
            bundle.scopes.push("network".to_string());
        }

        bundle
    }

    /// Export as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Export to JSON file
    pub fn to_json_file(&self, path: &str) -> Result<(), super::error::ObservabilityError> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Export as ZIP archive
    pub fn to_zip_file(&self, path: &str) -> Result<(), super::error::ObservabilityError> {
        let file = std::fs::File::create(path)?;
        let mut zip = zip::ZipWriter::new(file);

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Write manifest
        let manifest = serde_json::json!({
            "version": "1.0",
            "created_at": self.created_at,
            "scopes": self.scopes,
        });
        zip.start_file("manifest.json", options)?;
        zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

        // Write system info
        if let Some(ref system) = self.system {
            zip.start_file("system-info.json", options)?;
            zip.write_all(serde_json::to_string_pretty(system)?.as_bytes())?;
        }

        // Write config
        if let Some(ref config) = self.config {
            zip.start_file("config.json", options)?;
            zip.write_all(serde_json::to_string_pretty(config)?.as_bytes())?;
        }

        // Write tools
        if let Some(ref tools) = self.tools {
            zip.start_file("tools.json", options)?;
            zip.write_all(serde_json::to_string_pretty(tools)?.as_bytes())?;
        }

        // Write environment
        if let Some(ref env) = self.environment {
            zip.start_file("environment.json", options)?;
            zip.write_all(serde_json::to_string_pretty(env)?.as_bytes())?;
        }

        // Write network
        if let Some(ref network) = self.network {
            zip.start_file("network.json", options)?;
            zip.write_all(serde_json::to_string_pretty(network)?.as_bytes())?;
        }

        zip.finish()?;
        Ok(())
    }

    /// Generate a default filename for the bundle
    pub fn default_filename(format: &str) -> String {
        let ts = timestamp().replace([':', '-', 'T', 'Z'], "");
        match format {
            "zip" => format!("jarvy-diagnostic-{}.zip", ts),
            _ => format!("jarvy-diagnostic-{}.json", ts),
        }
    }
}

impl Default for DiagnosticBundle {
    fn default() -> Self {
        Self::new()
    }
}

/// Collect configuration (sanitized)
fn collect_config(config_path: Option<&str>, sanitizer: &Sanitizer) -> Option<serde_json::Value> {
    // Try to find config file
    let paths = if let Some(path) = config_path {
        vec![PathBuf::from(path)]
    } else {
        vec![PathBuf::from("jarvy.toml"), PathBuf::from(".jarvy.toml")]
    };

    for path in paths {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let sanitized = sanitizer.sanitize(&content);
                // Parse as TOML and convert to JSON
                if let Ok(toml_value) = toml::from_str::<toml::Value>(&sanitized) {
                    return Some(toml_to_json(&toml_value));
                }
            }
        }
    }

    None
}

/// Convert TOML value to JSON value
fn toml_to_json(toml: &toml::Value) -> serde_json::Value {
    match toml {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => {
            let map: serde_json::Map<String, serde_json::Value> = table
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
    }
}

/// Collect tool status
fn collect_tool_status() -> Vec<ToolStatus> {
    let common_tools = [
        "git", "node", "npm", "python", "pip", "rust", "cargo", "docker", "kubectl", "go", "java",
        "ruby", "php",
    ];

    common_tools
        .iter()
        .map(|&tool| {
            let which_result = std::process::Command::new("which").arg(tool).output();

            let (installed, path) = match which_result {
                Ok(output) if output.status.success() => {
                    let p = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    (true, Some(p))
                }
                _ => (false, None),
            };

            let version = if installed {
                get_tool_version(tool)
            } else {
                None
            };

            let method = path.as_ref().and_then(|p| detect_install_method(p));

            ToolStatus {
                name: tool.to_string(),
                installed,
                version,
                path,
                method,
            }
        })
        .collect()
}

/// Get tool version
fn get_tool_version(tool: &str) -> Option<String> {
    let output = std::process::Command::new(tool)
        .arg("--version")
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}{}", stdout, stderr);

        // Extract version number
        let re = regex::Regex::new(r"v?(\d+\.\d+(?:\.\d+)?)").ok()?;
        re.captures(&combined)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    } else {
        None
    }
}

/// Detect install method from path. Delegates to the canonical
/// classifier in `tools::install_method` (round-2 maint F1).
///
/// Bundle preserves its `Brew → "homebrew"` long-form label and
/// returns `None` for `Unknown` since the bundle JSON omits the
/// field when no method is detectable.
fn detect_install_method(path: &str) -> Option<String> {
    use crate::tools::install_method::{InstallMethod, detect_install_method_from_path};
    let method = detect_install_method_from_path(std::path::Path::new(path));
    match method {
        InstallMethod::Unknown | InstallMethod::NotFound => None,
        InstallMethod::Brew => Some("homebrew".to_string()),
        other => Some(other.to_string()),
    }
}

/// Collect environment variables (sanitized)
fn collect_environment(sanitizer: &Sanitizer) -> Vec<EnvVar> {
    // Only include relevant environment variables
    let relevant_prefixes = [
        "PATH",
        "HOME",
        "USER",
        "SHELL",
        "TERM",
        "LANG",
        "LC_",
        "JARVY_",
        "RUST",
        "CARGO",
        "NODE",
        "NPM",
        "PYTHON",
        "PIP",
        "DOCKER",
        "KUBE",
        "GO",
        "JAVA",
        "RUBY",
        "XDG_",
        "SSH_AUTH_SOCK",
    ];

    std::env::vars()
        .filter(|(key, _)| {
            relevant_prefixes
                .iter()
                .any(|prefix| key.starts_with(prefix))
        })
        .map(|(key, value)| EnvVar {
            name: key,
            value: sanitizer.sanitize(&value),
        })
        .collect()
}

/// Run network connectivity tests
fn run_connectivity_tests() -> Vec<ConnectivityTest> {
    let targets = [
        ("github.com", "GitHub"),
        ("registry.npmjs.org", "npm Registry"),
        ("pypi.org", "PyPI"),
        ("crates.io", "crates.io"),
        ("formulae.brew.sh", "Homebrew"),
    ];

    targets
        .iter()
        .map(|(host, name)| {
            let start = std::time::Instant::now();

            // Simple TCP connect test
            let result = std::net::TcpStream::connect_timeout(
                &format!("{}:443", host).parse().unwrap(),
                std::time::Duration::from_secs(5),
            );

            let latency = start.elapsed().as_millis() as u64;

            match result {
                Ok(_) => ConnectivityTest {
                    target: format!("{} ({})", name, host),
                    reachable: true,
                    latency_ms: Some(latency),
                    error: None,
                },
                Err(e) => ConnectivityTest {
                    target: format!("{} ({})", name, host),
                    reachable: false,
                    latency_ms: None,
                    error: Some(e.to_string()),
                },
            }
        })
        .collect()
}

/// Get OS version
fn get_os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("VERSION_ID="))
                    .map(|l| {
                        l.trim_start_matches("VERSION_ID=")
                            .trim_matches('"')
                            .to_string()
                    })
            })
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "ver"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        "unknown".to_string()
    }
}

/// Generate timestamp
fn timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    // Format as ISO 8601-ish
    let secs = duration.as_secs();
    format!("{}", secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_scope_parsing() {
        assert_eq!("system".parse::<BundleScope>(), Ok(BundleScope::System));
        assert_eq!("TOOLS".parse::<BundleScope>(), Ok(BundleScope::Tools));
        assert_eq!("env".parse::<BundleScope>(), Ok(BundleScope::Environment));
        assert!("unknown".parse::<BundleScope>().is_err());
    }

    #[test]
    fn test_bundle_scope_list() {
        let scopes = BundleScope::parse_list("system,tools,network");
        assert_eq!(scopes.len(), 3);
        assert!(scopes.contains(&BundleScope::System));
        assert!(scopes.contains(&BundleScope::Tools));
        assert!(scopes.contains(&BundleScope::Network));
    }

    #[test]
    fn test_system_info_collect() {
        let info = SystemInfo::collect();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(!info.jarvy_version.is_empty());
    }

    #[test]
    fn test_bundle_creation() {
        let bundle = DiagnosticBundle::new();
        assert!(bundle.system.is_none());
        assert!(!bundle.created_at.is_empty());
    }

    #[test]
    fn test_bundle_collect_system() {
        let bundle = DiagnosticBundle::collect(&[BundleScope::System], None);
        assert!(bundle.system.is_some());
        assert!(bundle.config.is_none());
        assert!(bundle.scopes.contains(&"system".to_string()));
    }

    #[test]
    fn test_bundle_to_json() {
        let bundle = DiagnosticBundle::collect(&[BundleScope::System], None);
        let json = bundle.to_json().unwrap();
        assert!(json.contains("system"));
        assert!(json.contains("created_at"));
    }

    #[test]
    fn test_default_filename() {
        let json_name = DiagnosticBundle::default_filename("json");
        assert!(json_name.starts_with("jarvy-diagnostic-"));
        assert!(json_name.ends_with(".json"));

        let zip_name = DiagnosticBundle::default_filename("zip");
        assert!(zip_name.ends_with(".zip"));
    }

    #[test]
    fn test_detect_install_method() {
        assert_eq!(
            detect_install_method("/opt/homebrew/bin/git"),
            Some("homebrew".to_string())
        );
        assert_eq!(
            detect_install_method("/usr/bin/ls"),
            Some("system".to_string())
        );
    }
}
