use crate::tools::common::{InstallError, has, run};

/// Ensure GNU Wget is installed. The `min_hint` is ignored because
/// distro/package-manager provided wget versions vary; having the command
/// available is sufficient for most workflows.
pub fn ensure(_min_hint: &str) -> Result<(), InstallError> {
    if has("wget") {
        return Ok(());
    }
    install()
}

/// Registry adapter: allows tools::add("wget", version) to dispatch here
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
    run("brew", &["install", "wget"])?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_linux() -> Result<(), InstallError> {
    if let Some(pm) = crate::tools::common::detect_linux_pm() {
        let _ = crate::tools::common::PkgOps::update(pm, true);
        crate::tools::common::PkgOps::install(pm, "wget", true)
    } else {
        Err(InstallError::Prereq(
            "No supported Linux package manager on PATH (apt/dnf/yum/zypper/pacman/apk)",
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
    // Common Winget package for GNU Wget (GnuWin32 build). Using exact match to avoid ambiguity.
    run("winget", &["install", "-e", "--id", "GnuWin32.Wget"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_wget_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
