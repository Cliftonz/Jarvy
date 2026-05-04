//! MCP Resource Handlers
//!
//! Implements the MCP resource interface for Jarvy:
//! - jarvy://tools/index: Complete index of all supported tools
//! - jarvy://platform/info: Current platform information
//! - jarvy://tools/{name}: Detailed information about a specific tool

use crate::mcp::error::{McpError, McpResult};
use crate::tools::spec::{generate_tool_index, get_tool_spec};
use serde::Serialize;

/// Resource definition for MCP resources/list response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResourceDefinition {
    /// Resource URI
    pub uri: String,
    /// Resource name
    pub name: String,
    /// Resource description
    pub description: String,
    /// MIME type
    pub mime_type: String,
}

/// List all MCP resources exposed by Jarvy
pub fn list_resources() -> Vec<McpResourceDefinition> {
    vec![
        McpResourceDefinition {
            uri: "jarvy://tools/index".to_string(),
            name: "Tool Index".to_string(),
            description: "Complete index of all supported tools with metadata".to_string(),
            mime_type: "application/json".to_string(),
        },
        McpResourceDefinition {
            uri: "jarvy://platform/info".to_string(),
            name: "Platform Info".to_string(),
            description: "Current platform, OS version, and available package managers".to_string(),
            mime_type: "application/json".to_string(),
        },
        McpResourceDefinition {
            uri: "jarvy://config".to_string(),
            name: "Project Config".to_string(),
            description: "Parsed jarvy.toml configuration for the current project".to_string(),
            mime_type: "application/json".to_string(),
        },
        McpResourceDefinition {
            uri: "jarvy://doctor".to_string(),
            name: "Environment Health".to_string(),
            description: "Doctor diagnostics for configured tools (installed, versions, issues)"
                .to_string(),
            mime_type: "application/json".to_string(),
        },
        McpResourceDefinition {
            uri: "jarvy://schema".to_string(),
            name: "Config Schema".to_string(),
            description: "JSON Schema for jarvy.toml (for editor autocomplete and validation)"
                .to_string(),
            mime_type: "application/schema+json".to_string(),
        },
    ]
}

/// Read a resource by URI
pub fn read_resource(uri: &str) -> McpResult<String> {
    match uri {
        "jarvy://tools/index" => read_tools_index(),
        "jarvy://platform/info" => read_platform_info(),
        "jarvy://config" => read_project_config(),
        "jarvy://doctor" => read_doctor_results(),
        "jarvy://schema" => read_config_schema(),
        _ if uri.starts_with("jarvy://tools/") => {
            let tool_name = uri.strip_prefix("jarvy://tools/").unwrap();
            read_tool_details(tool_name)
        }
        _ => Err(McpError::invalid_params(format!(
            "Unknown resource URI: {}",
            uri
        ))),
    }
}

/// Read the complete tool index
fn read_tools_index() -> McpResult<String> {
    let index = generate_tool_index();
    serde_json::to_string_pretty(&index).map_err(|e| McpError::internal_error(e.to_string()))
}

/// Read platform information
fn read_platform_info() -> McpResult<String> {
    let info = PlatformInfo::detect();
    serde_json::to_string_pretty(&info).map_err(|e| McpError::internal_error(e.to_string()))
}

/// Read details for a specific tool
fn read_tool_details(tool_name: &str) -> McpResult<String> {
    let spec = get_tool_spec(tool_name).ok_or_else(|| McpError::unknown_tool(tool_name))?;

    let index = generate_tool_index();
    let tool_entry = index.tools.iter().find(|t| t.name == tool_name);

    let details = serde_json::json!({
        "name": tool_name,
        "command": spec.command,
        "platforms": tool_entry.map(|t| serde_json::json!({
            "macos": t.macos,
            "linux": t.linux,
            "windows": t.windows
        })),
        "custom_install": tool_entry.map(|t| t.custom_install.has_custom_installer).unwrap_or(false),
    });

    serde_json::to_string_pretty(&details).map_err(|e| McpError::internal_error(e.to_string()))
}

/// Platform information
#[derive(Debug, Serialize)]
struct PlatformInfo {
    /// Operating system
    os: String,
    /// OS version
    os_version: Option<String>,
    /// Architecture
    arch: String,
    /// Available package managers
    package_managers: Vec<PackageManagerInfo>,
}

/// Package manager information
#[derive(Debug, Serialize)]
struct PackageManagerInfo {
    /// Package manager name
    name: String,
    /// Whether it's installed
    installed: bool,
    /// Version if installed
    version: Option<String>,
}

impl PlatformInfo {
    /// Detect current platform information
    fn detect() -> Self {
        let os = detect_os();
        let arch = detect_arch();
        let os_version = detect_os_version();
        let package_managers = detect_package_managers(&os);

        Self {
            os,
            os_version,
            arch,
            package_managers,
        }
    }
}

fn detect_os() -> String {
    #[cfg(target_os = "macos")]
    return "macos".to_string();
    #[cfg(target_os = "linux")]
    return "linux".to_string();
    #[cfg(target_os = "windows")]
    return "windows".to_string();
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    return std::env::consts::OS.to_string();
}

fn detect_arch() -> String {
    std::env::consts::ARCH.to_string()
}

fn detect_os_version() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            })
    }

    #[cfg(target_os = "linux")]
    {
        // Try to read /etc/os-release
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                for line in content.lines() {
                    if line.starts_with("VERSION_ID=") {
                        return Some(
                            line.trim_start_matches("VERSION_ID=")
                                .trim_matches('"')
                                .to_string(),
                        );
                    }
                }
                None
            })
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "ver"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    None
}

fn detect_package_managers(os: &str) -> Vec<PackageManagerInfo> {
    let mut managers = Vec::new();

    match os {
        "macos" => {
            managers.push(check_package_manager("brew", &["--version"]));
            managers.push(check_package_manager("port", &["version"]));
        }
        "linux" => {
            managers.push(check_package_manager("apt", &["--version"]));
            managers.push(check_package_manager("dnf", &["--version"]));
            managers.push(check_package_manager("yum", &["--version"]));
            managers.push(check_package_manager("pacman", &["--version"]));
            managers.push(check_package_manager("apk", &["--version"]));
            managers.push(check_package_manager("zypper", &["--version"]));
        }
        "windows" => {
            managers.push(check_package_manager("winget", &["--version"]));
            managers.push(check_package_manager("choco", &["--version"]));
            managers.push(check_package_manager("scoop", &["--version"]));
        }
        _ => {}
    }

    // Common cross-platform package managers
    managers.push(check_package_manager("npm", &["--version"]));
    managers.push(check_package_manager("pip", &["--version"]));
    managers.push(check_package_manager("cargo", &["--version"]));

    // Filter to only include installed ones
    managers.into_iter().filter(|m| m.installed).collect()
}

fn check_package_manager(name: &str, version_args: &[&str]) -> PackageManagerInfo {
    let output = std::process::Command::new(name).args(version_args).output();

    match output {
        Ok(o) if o.status.success() => {
            let version_output = String::from_utf8_lossy(&o.stdout);
            let version = extract_version_number(&version_output);
            PackageManagerInfo {
                name: name.to_string(),
                installed: true,
                version,
            }
        }
        _ => PackageManagerInfo {
            name: name.to_string(),
            installed: false,
            version: None,
        },
    }
}

fn extract_version_number(output: &str) -> Option<String> {
    // Extract first version-like string
    for word in output.split_whitespace() {
        let word = word.trim_start_matches('v').trim_end_matches(',');
        if word.chars().next().is_some_and(|c| c.is_ascii_digit()) && word.contains('.') {
            return Some(word.to_string());
        }
    }
    // Fallback: return first line trimmed
    output.lines().next().map(|s| s.trim().to_string())
}

/// Read the project's jarvy.toml as JSON
fn read_project_config() -> McpResult<String> {
    let config_path = "./jarvy.toml";
    let content = std::fs::read_to_string(config_path)
        .map_err(|e| McpError::internal_error(format!("Cannot read {}: {}", config_path, e)))?;
    let parsed: toml::Value = toml::from_str(&content)
        .map_err(|e| McpError::internal_error(format!("Invalid TOML: {}", e)))?;
    serde_json::to_string_pretty(&parsed).map_err(|e| McpError::internal_error(e.to_string()))
}

/// Read doctor diagnostics as JSON
fn read_doctor_results() -> McpResult<String> {
    let result = crate::commands::doctor::run_doctor(None, None);
    serde_json::to_string_pretty(&result).map_err(|e| McpError::internal_error(e.to_string()))
}

/// Read the JSON Schema for jarvy.toml
fn read_config_schema() -> McpResult<String> {
    let schema_output = crate::commands::schema::generate_schema();
    serde_json::to_string_pretty(&schema_output.schema)
        .map_err(|e| McpError::internal_error(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_resources() {
        let resources = list_resources();
        assert!(!resources.is_empty());
        assert!(resources.iter().any(|r| r.uri == "jarvy://tools/index"));
        assert!(resources.iter().any(|r| r.uri == "jarvy://platform/info"));
    }

    #[test]
    fn test_read_tools_index() {
        crate::tools::register_all();
        let result = read_resource("jarvy://tools/index");
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("tools"));
    }

    #[test]
    fn test_read_platform_info() {
        let result = read_resource("jarvy://platform/info");
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("os"));
        assert!(json.contains("arch"));
    }

    #[test]
    fn test_read_unknown_resource() {
        let result = read_resource("jarvy://unknown/resource");
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_os() {
        let os = detect_os();
        #[cfg(target_os = "macos")]
        assert_eq!(os, "macos");
        #[cfg(target_os = "linux")]
        assert_eq!(os, "linux");
        #[cfg(target_os = "windows")]
        assert_eq!(os, "windows");
    }
}
