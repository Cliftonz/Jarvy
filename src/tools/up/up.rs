#[cfg(any(target_os = "macos", target_os = "linux"))]
use crate::tools::common::run;
use crate::tools::common::{InstallError, has};

/// Ensure Upbound Up CLI (`up`) is available. Version hint is ignored; presence is enough.
pub fn ensure(_min_hint: &str) -> Result<(), InstallError> {
    if has("up") {
        return Ok(());
    }
    install()
}

/// Registry adapter
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
    // Upbound tap with formula `up`
    run("brew", &["install", "upbound/tap/up"])?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_linux() -> Result<(), InstallError> {
    // Prefer Homebrew on Linux as official tap exists; distro packages may refer to unrelated tools named `up`.
    if has("brew") {
        return run("brew", &["install", "upbound/tap/up"])
            .map(|_| ())
            .map_err(|e| e);
    }
    Err(InstallError::Prereq(
        "Automatic install for Upbound Up on Linux requires Homebrew (brew install upbound/tap/up). Install Homebrew or follow Upbound docs.",
    ))
}

#[cfg(target_os = "windows")]
fn install_windows() -> Result<(), InstallError> {
    Err(InstallError::Prereq(
        "Upbound Up installation on Windows is not automated in Jarvy. Use WSL with Homebrew (brew install upbound/tap/up) or see Upbound docs.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_up_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
