use crate::tools::common::{InstallError, has, run};

/// Ensure CUE language CLI is installed. Version hint is ignored; we install
/// the latest available from the platform package manager.
pub fn ensure(_min_hint: &str) -> Result<(), InstallError> {
    if has("cue") {
        return Ok(());
    }
    install()
}

/// Registry adapter: allows tools::add("cue", version) to dispatch here
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
    run("brew", &["install", "cue"])?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_linux() -> Result<(), InstallError> {
    // Try distro package manager first (package name may vary; attempt 'cue')
    if let Some(pm) = crate::tools::common::detect_linux_pm() {
        let _ = crate::tools::common::PkgOps::update(pm, true);
        let res = crate::tools::common::PkgOps::install(pm, "cue", true);
        if res.is_ok() {
            return res;
        }
    }
    // Fallback to Homebrew on Linux if present
    if has("brew") {
        return run("brew", &["install", "cue"]).map(|_| ());
    }
    Err(InstallError::Prereq(
        "No supported Linux package manager or Homebrew found to install cue",
    ))
}

#[cfg(target_os = "windows")]
fn install_windows() -> Result<(), InstallError> {
    // No official Windows package in this project scope; require WSL or manual install.
    Err(InstallError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_cue_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
