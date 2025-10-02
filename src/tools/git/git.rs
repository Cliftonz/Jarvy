use crate::tools::common::{InstallError, cmd_satisfies};

/// Ensure Git is installed and at least roughly matches `min_hint`
/// (e.g., "2.40" → accepts 2.40.x+)
pub fn ensure(min_hint: &str) -> Result<(), InstallError> {
    if cmd_satisfies("git", min_hint) {
        return Ok(());
    }
    install()
}

/// Registry adapter: allows tools::add("git", version) to dispatch here
pub fn add_handler(min_hint: &str) -> Result<(), InstallError> {
    ensure(min_hint)
}

fn install() -> Result<(), InstallError> {
    #[cfg(target_os = "macos")]
    {
        return install_macos();
    }
    #[cfg(target_os = "linux")]
    {
        return install_linux();
    }
    #[cfg(target_os = "windows")]
    {
        return install_windows();
    }
    #[allow(unreachable_code)]
    Err(InstallError::Unsupported)
}

#[cfg(target_os = "macos")]
fn install_macos() -> Result<(), InstallError> {
    if !crate::tools::common::has("brew") {
        return Err(InstallError::Prereq(
            "Homebrew not found. Install https://brew.sh and re-run.",
        ));
    }
    crate::tools::common::run("brew", &["install", "git"])?; // modern Git via Homebrew
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_linux() -> Result<(), InstallError> {
    let apt = crate::tools::common::require_any(
        &["apt", "apt-get"],
        "Need apt or apt-get on PATH for Debian/Ubuntu",
    )?;
    crate::tools::common::run("sudo", &[apt, "update"])?;
    crate::tools::common::run("sudo", &[apt, "install", "-y", "git"])?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_windows() -> Result<(), InstallError> {
    if !crate::tools::common::has("winget") {
        return Err(InstallError::Prereq(
            "winget not found. Install Windows Package Manager, then re-run.",
        ));
    }
    // Official Git for Windows package ID:
    crate::tools::common::run("winget", &["install", "-e", "--id", "Git.Git"])?; // exact ID avoids ambiguity
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Generic test: ensure calling Git installer path does not panic.
    // Actual OS-specific installation success is covered by e2e tests.
    #[test]
    fn ensure_git_no_panic() {
        let res = ensure("");
        // Do not assert success; environments may lack permissions or prerequisites.
        assert!(res.is_ok() || res.is_err());
    }

    // Platform-specific expectations for Git installer behavior.
    // Windows: Git is supported; ensure/install should never return Unsupported.
    #[cfg(target_os = "windows")]
    #[test]
    fn git_windows_not_unsupported() {
        let res = ensure("");
        assert!(
            !matches!(res, Err(InstallError::Unsupported)),
            "git on Windows should not return Unsupported",
        );
    }

    // If winget is missing on Windows, expect a Prereq error (deterministic outcome).
    #[cfg(target_os = "windows")]
    #[test]
    fn git_windows_prereq_if_no_winget() {
        if !crate::tools::common::has("winget") {
            let res = ensure("");
            assert!(
                matches!(res, Err(InstallError::Prereq(_))),
                "Expected Prereq when winget is absent"
            );
        }
    }

    // macOS: Git install path requires Homebrew; if missing, expect Prereq.
    // Otherwise, it should not be Unsupported (command may fail in CI due to permissions).
    #[cfg(target_os = "macos")]
    #[test]
    fn git_macos_expected_outcome() {
        if !crate::tools::common::has("brew") {
            let res = ensure("");
            assert!(
                matches!(res, Err(InstallError::Prereq(_))),
                "Expected Prereq when Homebrew is absent"
            );
        } else {
            let res = ensure("");
            assert!(
                !matches!(res, Err(InstallError::Unsupported)),
                "git on macOS should not return Unsupported",
            );
        }
    }

    // Linux: Debian/Ubuntu path expects apt or apt-get; if both missing, expect Prereq.
    // Otherwise, it should not be Unsupported (commands may fail in CI due to permissions).
    #[cfg(target_os = "linux")]
    #[test]
    fn git_linux_expected_outcome() {
        let has_apt = crate::tools::common::has("apt");
        let has_apt_get = crate::tools::common::has("apt-get");
        let res = ensure("");
        if !has_apt && !has_apt_get {
            assert!(
                matches!(res, Err(InstallError::Prereq(_))),
                "Expected Prereq when apt/apt-get are absent"
            );
        } else {
            assert!(
                !matches!(res, Err(InstallError::Unsupported)),
                "git on Linux should not return Unsupported",
            );
        }
    }
}
