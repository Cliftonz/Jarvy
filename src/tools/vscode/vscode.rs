use crate::tools::common::{InstallError, has, run};

/// Ensure Visual Studio Code is installed. The `min_hint` is ignored for now
/// as VS Code is distributed via package managers without strict semver pins.
pub fn ensure(_min_hint: &str) -> Result<(), InstallError> {
    if has("code") {
        // `code` CLI present; assume VS Code installed
        return Ok(());
    }
    install()
}

/// Registry adapter: allows tools::add("vscode", version) to dispatch here
pub fn add_handler(min_hint: &str) -> Result<(), InstallError> {
    let _ = min_hint;
    ensure("")
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
    if !has("brew") {
        return Err(InstallError::Prereq(
            "Homebrew not found. Install https://brew.sh and re-run.",
        ));
    }
    // VS Code is distributed as a cask
    run("brew", &["install", "--cask", "visual-studio-code"])?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_linux() -> Result<(), InstallError> {
    // Prefer snap if present
    if has("snap") {
        // classic confinement is required for code
        run("sudo", &["snap", "install", "code", "--classic"])?;
        return Ok(());
    }

    // Fallback to distro package manager if detected
    if let Some(pm) = crate::tools::common::detect_linux_pm() {
        let _ = crate::tools::common::PkgOps::update(pm, crate::tools::common::default_use_sudo());
        // Package name is typically "code" when MS repo is configured; still attempt
        crate::tools::common::PkgOps::install(pm, "code", crate::tools::common::default_use_sudo())
    } else {
        Err(InstallError::Prereq(
            "Need snap or a supported package manager (apt/dnf/yum/zypper/pacman/apk) on PATH",
        ))
    }
}

#[cfg(target_os = "windows")]
fn install_windows() -> Result<(), InstallError> {
    if !has("winget") {
        return Err(InstallError::Prereq(
            "winget not found. Install Windows Package Manager, then re-run.",
        ));
    }
    // Official VS Code package ID in winget
    run(
        "winget",
        &["install", "-e", "--id", "Microsoft.VisualStudioCode"],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_vscode_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn vscode_windows_not_unsupported() {
        let res = ensure("");
        assert!(
            !matches!(res, Err(InstallError::Unsupported)),
            "vscode on Windows should not return Unsupported",
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn vscode_windows_prereq_if_no_winget() {
        if !has("winget") {
            let res = ensure("");
            assert!(matches!(res, Err(InstallError::Prereq(_))));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn vscode_macos_expected_outcome() {
        if !has("brew") {
            let res = ensure("");
            assert!(matches!(res, Err(InstallError::Prereq(_))));
        } else {
            let res = ensure("");
            assert!(
                !matches!(res, Err(InstallError::Unsupported)),
                "vscode on macOS should not return Unsupported",
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn vscode_linux_expected_outcome() {
        let has_snap = has("snap");
        let has_pm = crate::tools::common::detect_linux_pm().is_some();
        let res = ensure("");
        if !has_snap && !has_pm {
            assert!(matches!(res, Err(InstallError::Prereq(_))));
        } else {
            assert!(
                !matches!(res, Err(InstallError::Unsupported)),
                "vscode on Linux should not return Unsupported",
            );
        }
    }
}
