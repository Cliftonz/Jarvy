use std::process::{Command, Output};
use std::sync::OnceLock;

#[derive(thiserror::Error, Debug)]
pub enum InstallError {
    #[error("unsupported platform")]
    Unsupported,
    #[error("prerequisite missing: {0}")]
    Prereq(&'static str),
    #[error("invalid permissions: {0}")]
    InvalidPermissions(&'static str),
    #[error("command failed: {cmd} (code: {code:?})\n{stderr}")]
    CommandFailed {
        cmd: String,
        code: Option<i32>,
        stderr: String,
    },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(&'static str),
}

// OS enum for config keys and runtime resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Linux,
    Macos,
    Windows,
}

// Determine the current OS as our enum
pub fn current_os() -> Os {
    #[cfg(target_os = "linux")]
    {
        Os::Linux
    }
    #[cfg(target_os = "macos")]
    {
        Os::Macos
    }
    #[cfg(target_os = "windows")]
    {
        Os::Windows
    }
}

// Global default for whether to use sudo on POSIX installs. Can be set from Config in main.
// None means: auto-detect per operation (try without sudo, then with if available).
static USE_SUDO_DEFAULT: OnceLock<Option<bool>> = OnceLock::new();

pub fn set_default_use_sudo(val: Option<bool>) {
    let _ = USE_SUDO_DEFAULT.set(val);
}

pub fn default_use_sudo() -> Option<bool> {
    if let Some(v) = USE_SUDO_DEFAULT.get() {
        *v
    } else {
        // Unset -> auto mode
        None
    }
}

pub fn run(cmd: &str, args: &[&str]) -> Result<Output, InstallError> {
    // Fast, deterministic tests: allow skipping external command execution.
    // Integration tests can opt-in via JARVY_FAST_TEST; unit tests default to skip unless explicitly overridden.
    if std::env::var_os("JARVY_FAST_TEST").is_some() {
        return Err(InstallError::Prereq(
            "skipped external command in fast test mode",
        ));
    }
    #[cfg(test)]
    {
        if std::env::var_os("JARVY_RUN_EXTERNAL_CMDS_IN_TEST").is_none() {
            return Err(InstallError::Prereq(
                "external commands disabled during unit tests",
            ));
        }
    }

    let out = Command::new(cmd).args(args).output().map_err(|e| {
        use std::io::ErrorKind::*;
        match e.kind() {
            NotFound => InstallError::Prereq("required command not found on PATH"),
            PermissionDenied => {
                InstallError::InvalidPermissions("operation requires elevated privileges")
            }
            _ => InstallError::Io(e),
        }
    })?;

    if !out.status.success() {
        // Try to capture stderr for easier diagnostics.
        return Err(InstallError::CommandFailed {
            cmd: cmd.to_string(),
            code: out.status.code(),
            stderr: String::from_utf8_lossy(&out.stderr).into(),
        });
    }
    Ok(out)
}

// Run a command, prefixing with sudo if configured and applicable (non-Windows)
pub fn run_maybe_sudo(use_sudo: bool, cmd: &str, args: &[&str]) -> Result<Output, InstallError> {
    match current_os() {
        Os::Windows => run(cmd, args),
        Os::Linux | Os::Macos => {
            if use_sudo {
                // sudo <cmd> <args...>
                let mut all = Vec::with_capacity(1 + args.len());
                all.push(cmd);
                all.extend_from_slice(args);
                run("sudo", &all)
            } else {
                run(cmd, args)
            }
        }
    }
}

pub fn has(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// Require a single command to exist on PATH, otherwise return a Prereq error with remediation.
pub fn require(cmd: &str, remediation: &'static str) -> Result<(), InstallError> {
    if has(cmd) {
        Ok(())
    } else {
        Err(InstallError::Prereq(remediation))
    }
}

// Require one of multiple candidates (e.g., apt or apt-get)
pub fn require_any<'a>(
    candidates: &[&'a str],
    remediation: &'static str,
) -> Result<&'a str, InstallError> {
    for c in candidates {
        if has(c) {
            return Ok(*c);
        }
    }
    Err(InstallError::Prereq(remediation))
}

// crude semver probe like: "git version 2.44.0"
pub fn cmd_satisfies(cmd: &str, min_prefix: &str) -> bool {
    if let Ok(out) = Command::new(cmd).arg("--version").output() {
        let s = String::from_utf8_lossy(&out.stdout);
        return s.contains(min_prefix);
    }
    false
}

pub fn plan_sudo_attempts(use_sudo: Option<bool>, sudo_available: bool) -> Vec<bool> {
    match use_sudo {
        Some(flag) => vec![flag],
        None => {
            if sudo_available {
                vec![false, true]
            } else {
                vec![false]
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PackageManager {
    Apt,
    Dnf,
    Yum,
    Zypper,
    Pacman,
    Apk,
    Brew,
    Winget,
}

#[cfg(target_os = "linux")]
pub fn detect_linux_pm() -> Option<PackageManager> {
    use std::{fs, process::Command};
    let has = |c| {
        Command::new(c)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    // (Optional) use /etc/os-release to bias choices when you need vendor repos
    // ID / ID_LIKE fields are the standard signals.  [oai_citation:0‡Freedesktop](https://www.freedesktop.org/software/systemd/man/os-release.html?utm_source=chatgpt.com) [oai_citation:1‡Debian Manpages](https://manpages.debian.org/trixie/systemd/os-release.5.en.html?utm_source=chatgpt.com)
    let _os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();

    if has("apt-get") || has("apt") {
        return Some(PackageManager::Apt);
    }
    if has("dnf") {
        return Some(PackageManager::Dnf);
    }
    if has("yum") {
        return Some(PackageManager::Yum);
    }
    if has("zypper") {
        return Some(PackageManager::Zypper);
    }
    if has("pacman") {
        return Some(PackageManager::Pacman);
    }
    if has("apk") {
        return Some(PackageManager::Apk);
    }
    None
}

#[cfg(test)]
mod sudo_plan_tests {
    use super::plan_sudo_attempts;

    #[test]
    fn plan_some_true_only_true() {
        let v = plan_sudo_attempts(Some(true), true);
        assert_eq!(v, vec![true]);
    }

    #[test]
    fn plan_some_false_only_false() {
        let v = plan_sudo_attempts(Some(false), true);
        assert_eq!(v, vec![false]);
    }

    #[test]
    fn plan_none_with_sudo_available() {
        let v = plan_sudo_attempts(None, true);
        assert_eq!(v, vec![false, true]);
    }

    #[test]
    fn plan_none_without_sudo_available() {
        let v = plan_sudo_attempts(None, false);
        assert_eq!(v, vec![false]);
    }
}

#[allow(dead_code)]
pub struct PkgOps {
    name: &'static str,
}

impl PkgOps {
    pub fn update(pm: PackageManager, use_sudo: Option<bool>) -> Result<(), InstallError> {
        match pm {
            PackageManager::Apt => {
                // Ensure prerequisites exist before attempting the update
                let apt = require_any(&["apt-get", "apt"], "apt is required to update packages")?;
                // Decide sudo strategy
                match use_sudo {
                    Some(flag) => {
                        if flag {
                            require("sudo", "sudo is required to update packages")?;
                        }
                        run_maybe_sudo(flag, apt, &["update"])?;
                    }
                    None => {
                        // Try without sudo first
                        if let Err(e) = run_maybe_sudo(false, apt, &["update"]) {
                            // Retry with sudo if available
                            if has("sudo") {
                                run_maybe_sudo(true, apt, &["update"])?;
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            PackageManager::Dnf => { /* dnf auto-refreshes; optional */ }
            PackageManager::Yum => { /* optional */ }
            PackageManager::Zypper => {
                require("zypper", "zypper is required to update packages")?;
                match use_sudo {
                    Some(flag) => {
                        if flag {
                            require("sudo", "sudo is required to update packages")?;
                        }
                        run_maybe_sudo(flag, "zypper", &["--non-interactive", "refresh"])?;
                    }
                    None => {
                        if let Err(e) =
                            run_maybe_sudo(false, "zypper", &["--non-interactive", "refresh"])
                        {
                            if has("sudo") {
                                run_maybe_sudo(true, "zypper", &["--non-interactive", "refresh"])?;
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            PackageManager::Pacman => {
                require("pacman", "pacman is required to update packages")?;
                match use_sudo {
                    Some(flag) => {
                        if flag {
                            require("sudo", "sudo is required to update packages")?;
                        }
                        run_maybe_sudo(flag, "pacman", &["-Sy"])?;
                    }
                    None => {
                        if let Err(e) = run_maybe_sudo(false, "pacman", &["-Sy"]) {
                            if has("sudo") {
                                run_maybe_sudo(true, "pacman", &["-Sy"])?;
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            PackageManager::Apk => { /* `apk add` refreshes on demand */ }
            _ => {}
        }
        Ok(())
    }

    pub fn install(
        pm: PackageManager,
        pkg: &str,
        use_sudo: Option<bool>,
    ) -> Result<(), InstallError> {
        match pm {
            PackageManager::Apt => {
                let apt = require_any(&["apt-get", "apt"], "apt is required to install packages")?;
                match use_sudo {
                    Some(flag) => {
                        if flag {
                            require("sudo", "sudo is required to install packages")?;
                        }
                        run_maybe_sudo(flag, apt, &["install", "-y", pkg])?;
                    }
                    None => {
                        if let Err(e) = run_maybe_sudo(false, apt, &["install", "-y", pkg]) {
                            if has("sudo") {
                                run_maybe_sudo(true, apt, &["install", "-y", pkg])?;
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            PackageManager::Dnf => {
                require("dnf", "dnf is required to install packages")?;
                match use_sudo {
                    Some(flag) => {
                        if flag {
                            require("sudo", "sudo is required to install packages")?;
                        }
                        run_maybe_sudo(flag, "dnf", &["install", "-y", pkg])?;
                    }
                    None => {
                        if let Err(e) = run_maybe_sudo(false, "dnf", &["install", "-y", pkg]) {
                            if has("sudo") {
                                run_maybe_sudo(true, "dnf", &["install", "-y", pkg])?;
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            PackageManager::Yum => {
                require("yum", "yum is required to install packages")?;
                match use_sudo {
                    Some(flag) => {
                        if flag {
                            require("sudo", "sudo is required to install packages")?;
                        }
                        run_maybe_sudo(flag, "yum", &["install", "-y", pkg])?;
                    }
                    None => {
                        if let Err(e) = run_maybe_sudo(false, "yum", &["install", "-y", pkg]) {
                            if has("sudo") {
                                run_maybe_sudo(true, "yum", &["install", "-y", pkg])?;
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            PackageManager::Zypper => {
                require("zypper", "zypper is required to install packages")?;
                match use_sudo {
                    Some(flag) => {
                        if flag {
                            require("sudo", "sudo is required to install packages")?;
                        }
                        run_maybe_sudo(
                            flag,
                            "zypper",
                            &["--non-interactive", "install", "--no-confirm", pkg],
                        )?;
                    }
                    None => {
                        if let Err(e) = run_maybe_sudo(
                            false,
                            "zypper",
                            &["--non-interactive", "install", "--no-confirm", pkg],
                        ) {
                            if has("sudo") {
                                run_maybe_sudo(
                                    true,
                                    "zypper",
                                    &["--non-interactive", "install", "--no-confirm", pkg],
                                )?;
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            PackageManager::Pacman => {
                require("pacman", "pacman is required to install packages")?;
                match use_sudo {
                    Some(flag) => {
                        if flag {
                            require("sudo", "sudo is required to install packages")?;
                        }
                        run_maybe_sudo(flag, "pacman", &["--noconfirm", "-S", pkg])?;
                    }
                    None => {
                        if let Err(e) = run_maybe_sudo(false, "pacman", &["--noconfirm", "-S", pkg])
                        {
                            if has("sudo") {
                                run_maybe_sudo(true, "pacman", &["--noconfirm", "-S", pkg])?;
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            PackageManager::Apk => {
                require("apk", "apk is required to install packages")?;
                match use_sudo {
                    Some(flag) => {
                        if flag {
                            require("sudo", "sudo is required to install packages")?;
                        }
                        run_maybe_sudo(flag, "apk", &["add", pkg])?;
                    }
                    None => {
                        if let Err(e) = run_maybe_sudo(false, "apk", &["add", pkg]) {
                            if has("sudo") {
                                run_maybe_sudo(true, "apk", &["add", pkg])?;
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            // These package managers generally do not require sudo by design here
            PackageManager::Brew => {
                require("brew", "Homebrew is required to install packages")?;
                run("brew", &["install", pkg])?;
            }
            PackageManager::Winget => {
                require("winget", "Winget is required to install packages")?;
                run("winget", &["install", "-e", "--id", pkg])?;
            }
        };
        Ok(())
    }
}
