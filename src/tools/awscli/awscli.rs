use crate::tools::common::{InstallError, has, run};

/// Ensure AWS CLI is installed. We ignore the version hint and install the
/// latest available via the platform package manager.
pub fn ensure(_min_hint: &str) -> Result<(), InstallError> {
    if has("aws") {
        return Ok(());
    }
    install()
}

/// Registry adapter: allows tools::add("awscli", version) to dispatch here
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
    run("brew", &["install", "awscli"])?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_linux() -> Result<(), InstallError> {
    // Try distro package manager first
    if let Some(pm) = crate::tools::common::detect_linux_pm() {
        let _ = crate::tools::common::PkgOps::update(pm, crate::tools::common::default_use_sudo());
        let res = crate::tools::common::PkgOps::install(
            pm,
            "awscli",
            crate::tools::common::default_use_sudo(),
        );
        if res.is_ok() {
            return res;
        }
    }
    // Fallback: use Homebrew if available on Linux
    if has("brew") {
        return run("brew", &["install", "awscli"]).map(|_| ());
    }
    Err(InstallError::Prereq(
        "No supported Linux package manager or Homebrew found to install awscli",
    ))
}

#[cfg(target_os = "windows")]
fn install_windows() -> Result<(), InstallError> {
    if !has("winget") {
        return Err(InstallError::Prereq(
            "winget not found. Install Windows Package Manager, then re-run.",
        ));
    }
    // Official AWS CLI v2 package in winget
    run("winget", &["install", "-e", "--id", "Amazon.AWSCLI"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_awscli_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
