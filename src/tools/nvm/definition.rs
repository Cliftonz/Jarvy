#[cfg(target_os = "windows")]
use crate::tools::common::has;
use crate::tools::common::{InstallError, run};

pub fn ensure(_min_hint: &str) -> Result<(), InstallError> {
    // Probe: on POSIX shells nvm is a function; test via bash -lc
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let ok = std::process::Command::new("bash")
            .args(["-lc", "command -v nvm >/dev/null"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Ok(());
        }
        return install_posix();
    }
    #[cfg(target_os = "windows")]
    {
        if has("nvm") {
            return Ok(());
        }
        return install_windows();
    }
    #[allow(unreachable_code)]
    Err(InstallError::Unsupported)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn install_posix() -> Result<(), InstallError> {
    // Official installer script from nvm-sh
    // https://github.com/nvm-sh/nvm
    run(
        "bash",
        &[
            "-lc",
            r#"curl -fsSL https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash"#,
        ],
    )?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_windows() -> Result<(), InstallError> {
    if !has("winget") {
        return Err(InstallError::Prereq(
            "winget not found. Install Windows Package Manager, then re-run.",
        ));
    }
    // NVM for Windows official package ID
    run(
        "winget",
        &["install", "-e", "--id", "CoreyButler.NVMforWindows"],
    )?;
    Ok(())
}

/// Registry adapter: allows tools::add("nvm", version) to dispatch here
pub fn add_handler(min_hint: &str) -> Result<(), InstallError> {
    // nvm installation does not strictly adhere to semantic versions; installer installs latest stable.
    // We ignore provided hint for now and just ensure it's present.
    let _ = min_hint;
    ensure("")
}
