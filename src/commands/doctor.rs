//! Environment health diagnostics
//!
//! Diagnose environment issues, check tool health, and verify PATH configuration.

use crate::config::Config;
use crate::output::{ExitCode, Format, Outputable, colors, header, icons, subheader};
use crate::tools::common::{cmd_satisfies, has};
use crate::tools::spec::{get_tool_default_hook, get_tool_spec, list_tool_names};
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

/// System information
#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub shell: String,
    pub home: String,
    pub package_manager: Option<String>,
}

/// PATH check result
#[derive(Debug, Clone, Serialize)]
pub struct PathCheck {
    pub path: String,
    pub status: PathStatus,
    pub in_path: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PathStatus {
    Ok,
    Missing,
    NotInPath,
}

/// Tool health status
#[derive(Debug, Clone, Serialize)]
pub struct ToolHealth {
    pub name: String,
    pub required: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed: Option<String>,
    pub status: ToolStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    Ok,
    Outdated,
    NotInstalled,
    Unknown,
}

/// Hook status check
#[derive(Debug, Clone, Serialize)]
pub struct HookStatus {
    pub name: String,
    pub description: String,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
}

/// Recommendation for fixing issues
#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    pub severity: RecommendationSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RecommendationSeverity {
    Error,
    Warning,
    Info,
}

/// Complete doctor result
#[derive(Debug, Clone, Serialize)]
pub struct DoctorResult {
    pub system: SystemInfo,
    pub path_checks: Vec<PathCheck>,
    pub tools: Vec<ToolHealth>,
    pub hooks: Vec<HookStatus>,
    pub recommendations: Vec<Recommendation>,
    pub exit_code: i32,
}

impl Outputable for DoctorResult {
    fn to_human(&self) -> String {
        let mut output = String::new();

        output.push_str(&header("Jarvy Doctor"));
        output.push('\n');

        // System Information
        output.push_str(&subheader("System Information"));
        output.push_str(&format!(
            "  OS: {} {} ({})\n",
            self.system.os, self.system.os_version, self.system.arch
        ));
        output.push_str(&format!("  Shell: {}\n", self.system.shell));
        if let Some(ref pm) = self.system.package_manager {
            output.push_str(&format!("  Package Manager: {}\n", pm));
        }

        // PATH Analysis
        if !self.path_checks.is_empty() {
            output.push_str(&subheader("PATH Analysis"));
            for check in &self.path_checks {
                let (icon, color) = match check.status {
                    PathStatus::Ok => (icons::OK, colors::GREEN),
                    PathStatus::Missing => (icons::ERROR, colors::RED),
                    PathStatus::NotInPath => (icons::WARN, colors::YELLOW),
                };
                let status_msg = if check.in_path {
                    "in PATH"
                } else {
                    "not in PATH"
                };
                output.push_str(&format!(
                    "  {}{}{} {} - {}\n",
                    color,
                    icon,
                    colors::RESET,
                    check.path,
                    status_msg
                ));
            }
        }

        // Tool Health
        if !self.tools.is_empty() {
            output.push_str(&subheader("Tool Health"));
            for tool in &self.tools {
                let (icon, color) = match tool.status {
                    ToolStatus::Ok => (icons::OK, colors::GREEN),
                    ToolStatus::Outdated => (icons::WARN, colors::YELLOW),
                    ToolStatus::NotInstalled => (icons::ERROR, colors::RED),
                    ToolStatus::Unknown => (icons::INFO, colors::CYAN),
                };

                let installed_str = tool
                    .installed
                    .as_ref()
                    .map(|v| format!(" (installed: {})", v))
                    .unwrap_or_else(|| " - not found".to_string());

                let status_msg = match tool.status {
                    ToolStatus::Ok => "satisfies requirement",
                    ToolStatus::Outdated => "outdated",
                    ToolStatus::NotInstalled => "not installed",
                    ToolStatus::Unknown => "unknown tool",
                };

                output.push_str(&format!(
                    "  {}{}{} {} {}{} - {}\n",
                    color,
                    icon,
                    colors::RESET,
                    tool.name,
                    tool.required,
                    installed_str,
                    status_msg
                ));
            }
        }

        // Hooks Status
        if !self.hooks.is_empty() {
            output.push_str(&subheader("Hooks Status"));
            for hook in &self.hooks {
                let (icon, color) = if hook.active {
                    (icons::OK, colors::GREEN)
                } else {
                    (icons::WARN, colors::YELLOW)
                };
                output.push_str(&format!(
                    "  {}{}{} {}: {}\n",
                    color,
                    icon,
                    colors::RESET,
                    hook.name,
                    hook.description
                ));
                if let Some(ref issue) = hook.issue {
                    output.push_str(&format!(
                        "      {}{}{}\n",
                        colors::DIM,
                        issue,
                        colors::RESET
                    ));
                }
            }
        }

        // Recommendations
        if !self.recommendations.is_empty() {
            output.push_str(&subheader("Recommendations"));
            for (i, rec) in self.recommendations.iter().enumerate() {
                let color = match rec.severity {
                    RecommendationSeverity::Error => colors::RED,
                    RecommendationSeverity::Warning => colors::YELLOW,
                    RecommendationSeverity::Info => colors::CYAN,
                };
                output.push_str(&format!(
                    "  {}{}. {}{}\n",
                    color,
                    i + 1,
                    rec.message,
                    colors::RESET
                ));
                if let Some(ref fix) = rec.fix {
                    output.push_str(&format!("     Fix: {}\n", fix));
                }
            }
        }

        output
    }

    fn exit_code(&self) -> ExitCode {
        match self.exit_code {
            0 => ExitCode::Ok,
            1 => ExitCode::Warning,
            _ => ExitCode::Error,
        }
    }
}

/// Run the doctor command
pub fn run_doctor(config: Option<&Config>, specific_tools: Option<Vec<String>>) -> DoctorResult {
    let system = collect_system_info();
    let path_checks = check_path_entries();

    // Get tools to check
    let tools_to_check = if let Some(tools) = specific_tools {
        // Specific tools requested
        tools
            .iter()
            .map(|t| (t.clone(), "latest".to_string()))
            .collect()
    } else if let Some(cfg) = config {
        // From config file
        cfg.get_tool_configs()
            .iter()
            .map(|(_, t)| (t.name.clone(), t.version.clone()))
            .collect()
    } else {
        // Default: check common tools
        vec![
            ("git".to_string(), "latest".to_string()),
            ("node".to_string(), "latest".to_string()),
            ("python".to_string(), "latest".to_string()),
        ]
    };

    let tools = check_tool_health(&tools_to_check);
    let hooks = check_hook_status(config);
    let recommendations = generate_recommendations(&path_checks, &tools, &hooks);

    // Calculate exit code
    let has_errors = tools.iter().any(|t| t.status == ToolStatus::NotInstalled)
        || recommendations
            .iter()
            .any(|r| r.severity == RecommendationSeverity::Error);
    let has_warnings = tools.iter().any(|t| t.status == ToolStatus::Outdated)
        || recommendations
            .iter()
            .any(|r| r.severity == RecommendationSeverity::Warning);

    let exit_code = if has_errors {
        2
    } else if has_warnings {
        1
    } else {
        0
    };

    DoctorResult {
        system,
        path_checks,
        tools,
        hooks,
        recommendations,
        exit_code,
    }
}

fn collect_system_info() -> SystemInfo {
    let os = if cfg!(target_os = "macos") {
        "macOS".to_string()
    } else if cfg!(target_os = "linux") {
        "Linux".to_string()
    } else if cfg!(target_os = "windows") {
        "Windows".to_string()
    } else {
        "Unknown".to_string()
    };

    let os_version = get_os_version();

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64".to_string()
    } else if cfg!(target_arch = "aarch64") {
        "arm64".to_string()
    } else {
        std::env::consts::ARCH.to_string()
    };

    let shell = env::var("SHELL").unwrap_or_else(|_| "unknown".to_string());
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| "unknown".to_string());

    let package_manager = detect_package_manager();

    SystemInfo {
        os,
        os_version,
        arch,
        shell,
        home,
        package_manager,
    }
}

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

fn detect_package_manager() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        if has("brew") {
            // Get brew version
            let version = std::process::Command::new("brew")
                .arg("--version")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.lines().next().map(|l| l.to_string()))
                .unwrap_or_else(|| "Homebrew".to_string());
            return Some(version);
        }
    }
    #[cfg(target_os = "linux")]
    {
        if has("apt") {
            return Some("apt (Debian/Ubuntu)".to_string());
        }
        if has("dnf") {
            return Some("dnf (Fedora/RHEL)".to_string());
        }
        if has("pacman") {
            return Some("pacman (Arch)".to_string());
        }
        if has("apk") {
            return Some("apk (Alpine)".to_string());
        }
    }
    #[cfg(target_os = "windows")]
    {
        if has("winget") {
            return Some("winget".to_string());
        }
        if has("choco") {
            return Some("Chocolatey".to_string());
        }
    }
    None
}

fn check_path_entries() -> Vec<PathCheck> {
    let mut checks = Vec::new();

    // Common paths to check
    let paths_to_check = get_expected_paths();

    let current_path = env::var("PATH").unwrap_or_default();
    let path_entries: Vec<&str> = current_path.split(':').collect();

    for expected_path in paths_to_check {
        let exists = Path::new(&expected_path).exists();
        let in_path = path_entries.iter().any(|p| *p == expected_path);

        let status = if !exists {
            PathStatus::Missing
        } else if !in_path {
            PathStatus::NotInPath
        } else {
            PathStatus::Ok
        };

        checks.push(PathCheck {
            path: expected_path,
            status,
            in_path,
        });
    }

    checks
}

fn get_expected_paths() -> Vec<String> {
    let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    let mut paths = Vec::new();

    #[cfg(target_os = "macos")]
    {
        paths.push("/opt/homebrew/bin".to_string());
        paths.push("/usr/local/bin".to_string());
    }

    paths.push(format!("{}/.cargo/bin", home));
    paths.push(format!("{}/.local/bin", home));
    paths.push(format!("{}/.nvm/current/bin", home));

    #[cfg(target_os = "linux")]
    {
        paths.push("/usr/bin".to_string());
        paths.push("/usr/local/bin".to_string());
    }

    paths
}

fn check_tool_health(tools: &[(String, String)]) -> Vec<ToolHealth> {
    tools
        .iter()
        .map(|(name, version)| {
            let spec = get_tool_spec(name);
            let is_known = spec.is_some() || crate::tools::get_tool(name).is_some();

            if !is_known {
                return ToolHealth {
                    name: name.clone(),
                    required: version.clone(),
                    installed: None,
                    status: ToolStatus::Unknown,
                    path: None,
                };
            }

            let command = spec.map(|s| s.command).unwrap_or(name.as_str());
            let installed = get_installed_version(command);
            let path = which_command(command);

            let status = if installed.is_none() {
                ToolStatus::NotInstalled
            } else if cmd_satisfies(command, version) {
                ToolStatus::Ok
            } else {
                ToolStatus::Outdated
            };

            ToolHealth {
                name: name.clone(),
                required: version.clone(),
                installed,
                status,
                path,
            }
        })
        .collect()
}

fn get_installed_version(command: &str) -> Option<String> {
    // Try common version flags
    for flag in ["--version", "-v", "-V", "version"] {
        if let Ok(output) = std::process::Command::new(command).arg(flag).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}{}", stdout, stderr);

                // Extract version number
                if let Some(version) = extract_version(&combined) {
                    return Some(version);
                }
            }
        }
    }
    None
}

fn extract_version(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"v?(\d+\.\d+(?:\.\d+)?)").ok()?;
    re.captures(text).map(|c| c[1].to_string())
}

fn which_command(command: &str) -> Option<String> {
    #[cfg(unix)]
    {
        std::process::Command::new("which")
            .arg(command)
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
    }
    #[cfg(windows)]
    {
        std::process::Command::new("where")
            .arg(command)
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
                        .ok()
                        .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
                } else {
                    None
                }
            })
    }
}

fn check_hook_status(config: Option<&Config>) -> Vec<HookStatus> {
    let mut statuses = Vec::new();
    let home = env::var("HOME").unwrap_or_default();

    // Check common shell integrations
    let shell_rc = env::var("SHELL").ok().and_then(|s| {
        if s.contains("zsh") {
            Some(format!("{}/.zshrc", home))
        } else if s.contains("bash") {
            Some(format!("{}/.bashrc", home))
        } else {
            None
        }
    });

    if let Some(rc_path) = &shell_rc {
        let rc_content = std::fs::read_to_string(rc_path).unwrap_or_default();

        // Check for common integrations
        let integrations = [
            ("starship", "starship init", "Starship prompt"),
            ("zoxide", "zoxide init", "Zoxide directory jumper"),
            ("nvm", "nvm.sh", "Node Version Manager"),
            ("direnv", "direnv hook", "Directory environment"),
            ("fzf", "fzf", "Fuzzy finder"),
        ];

        for (name, pattern, desc) in integrations {
            if has(name) {
                let active = rc_content.contains(pattern);
                statuses.push(HookStatus {
                    name: name.to_string(),
                    description: desc.to_string(),
                    active,
                    issue: if !active {
                        Some(format!("{} not initialized in {}", name, rc_path))
                    } else {
                        None
                    },
                });
            }
        }
    }

    statuses
}

fn generate_recommendations(
    path_checks: &[PathCheck],
    tools: &[ToolHealth],
    hooks: &[HookStatus],
) -> Vec<Recommendation> {
    let mut recommendations = Vec::new();

    // Recommendations for missing tools
    for tool in tools {
        if tool.status == ToolStatus::NotInstalled {
            recommendations.push(Recommendation {
                severity: RecommendationSeverity::Error,
                message: format!("Install {}", tool.name),
                fix: Some(format!("jarvy setup --only {}", tool.name)),
            });
        } else if tool.status == ToolStatus::Outdated {
            recommendations.push(Recommendation {
                severity: RecommendationSeverity::Warning,
                message: format!("Update {} to {}", tool.name, tool.required),
                fix: Some(format!("jarvy upgrade {}", tool.name)),
            });
        }
    }

    // Recommendations for PATH issues
    for check in path_checks {
        if check.status == PathStatus::NotInPath {
            recommendations.push(Recommendation {
                severity: RecommendationSeverity::Warning,
                message: format!("{} not in PATH", check.path),
                fix: Some(format!(
                    "Add 'export PATH=\"{}:$PATH\"' to your shell rc",
                    check.path
                )),
            });
        }
    }

    // Recommendations for inactive hooks
    for hook in hooks {
        if !hook.active {
            if let Some(ref issue) = hook.issue {
                let default_hook = get_tool_default_hook(&hook.name);
                let fix = default_hook
                    .map(|h| format!("Run the default hook or add manually: {}", h.description));

                recommendations.push(Recommendation {
                    severity: RecommendationSeverity::Info,
                    message: issue.clone(),
                    fix,
                });
            }
        }
    }

    recommendations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_system_info() {
        let info = collect_system_info();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
    }

    #[test]
    fn test_extract_version() {
        assert_eq!(
            extract_version("git version 2.43.0"),
            Some("2.43.0".to_string())
        );
        assert_eq!(extract_version("v20.11.0"), Some("20.11.0".to_string()));
        assert_eq!(extract_version("Python 3.12.1"), Some("3.12.1".to_string()));
        assert_eq!(extract_version("1.75.0"), Some("1.75.0".to_string()));
    }

    #[test]
    fn test_path_status_serialization() {
        let check = PathCheck {
            path: "/test".to_string(),
            status: PathStatus::Ok,
            in_path: true,
        };
        let json = serde_json::to_string(&check).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
    }

    #[test]
    fn test_tool_status_serialization() {
        let health = ToolHealth {
            name: "git".to_string(),
            required: "latest".to_string(),
            installed: Some("2.43.0".to_string()),
            status: ToolStatus::Ok,
            path: Some("/usr/bin/git".to_string()),
        };
        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
    }

    #[test]
    fn test_doctor_result_exit_codes() {
        let result = DoctorResult {
            system: collect_system_info(),
            path_checks: vec![],
            tools: vec![],
            hooks: vec![],
            recommendations: vec![],
            exit_code: 0,
        };
        assert_eq!(result.exit_code(), ExitCode::Ok);

        let result_warn = DoctorResult {
            exit_code: 1,
            ..result.clone()
        };
        assert_eq!(result_warn.exit_code(), ExitCode::Warning);

        let result_err = DoctorResult {
            exit_code: 2,
            ..result
        };
        assert_eq!(result_err.exit_code(), ExitCode::Error);
    }
}
