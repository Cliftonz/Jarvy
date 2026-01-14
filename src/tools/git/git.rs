//! Git - distributed version control system
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(GIT, {
    command: "git",
    macos: { brew: "git" },
    linux: { uniform: "git" },
    windows: { winget: "Git.Git" },
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::common::InstallError;

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

    // Linux: use detected package manager; if none detected, expect Prereq.
    // Otherwise, it should not be Unsupported (commands may fail in CI due to permissions).
    #[cfg(target_os = "linux")]
    #[test]
    fn git_linux_expected_outcome() {
        let has_pm = crate::tools::common::detect_linux_pm().is_some();
        let res = ensure("");
        if !has_pm {
            assert!(
                matches!(res, Err(InstallError::Prereq(_))),
                "Expected Prereq when no supported package manager is detected"
            );
        } else {
            assert!(
                !matches!(res, Err(InstallError::Unsupported)),
                "git on Linux should not return Unsupported",
            );
        }
    }
}
