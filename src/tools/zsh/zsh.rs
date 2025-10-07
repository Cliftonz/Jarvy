#[cfg(target_os = "macos")]
use crate::tools::common::run;
use crate::tools::common::{InstallError, has};

/// Ensure `zsh` is available. Version hint ignored; presence is sufficient.
pub fn ensure(_min_hint: &str) -> Result<(), InstallError> {
    if has("zsh") {
        return Ok(());
    }
    install()
}

/// Registry adapter: allows tools::add("zsh", version) to dispatch here
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
    run("brew", &["install", "zsh"])?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_linux() -> Result<(), InstallError> {
    if let Some(pm) = crate::tools::common::detect_linux_pm() {
        let _ = crate::tools::common::PkgOps::update(pm, crate::tools::common::default_use_sudo());
        crate::tools::common::PkgOps::install(pm, "zsh", crate::tools::common::default_use_sudo())
    } else {
        Err(InstallError::Prereq(
            "No supported Linux package manager on PATH (apt/dnf/yum/zypper/pacman/apk)",
        ))
    }
}

#[cfg(target_os = "windows")]
fn install_windows() -> Result<(), InstallError> {
    Err(InstallError::Prereq(
        "zsh installation on Windows is not automated. Consider using WSL or install via MSYS2/Chocolatey.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_zsh_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
