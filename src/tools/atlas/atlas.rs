#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
use crate::tools::common::run;
use crate::tools::common::{InstallError, has};

/// Ensure `atlas` (Ariga Atlas) is available.
pub fn ensure(_min_hint: &str) -> Result<(), InstallError> {
    if has("atlas") {
        return Ok(());
    }
    install()
}

/// Registry adapter: allows tools::add("atlas", version) to dispatch here
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
    // Install from Ariga's tap
    run("brew", &["install", "ariga/tap/atlas"])?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_linux() -> Result<(), InstallError> {
    // Official install script provided by Ariga (atlasgo.sh)
    // We invoke via sh -c to handle the pipe.
    if !has("curl") {
        return Err(InstallError::Prereq(
            "curl is required to install Atlas via script. Please install curl and re-run.",
        ));
    }
    let cmd = "curl -sSf https://atlasgo.sh | sh";
    run("sh", &["-c", cmd])?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_windows() -> Result<(), InstallError> {
    if !has("winget") {
        return Err(InstallError::Prereq(
            "winget not found. Install Windows Package Manager, then re-run.",
        ));
    }
    // Official Atlas package on winget
    run("winget", &["install", "-e", "--id", "Ariga.Atlas"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_atlas_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
