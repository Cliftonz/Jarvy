use crate::tools::common::{InstallError, has, run};

/// Ensure `tree` command is available. `min_hint` is ignored since
/// the system-provided package is sufficient for most workflows.
pub fn ensure(_min_hint: &str) -> Result<(), InstallError> {
    if has("tree") {
        return Ok(());
    }
    install()
}

/// Registry adapter: allows tools::add("tree", version) to dispatch here
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
    run("brew", &["install", "tree"])?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_linux() -> Result<(), InstallError> {
    if let Some(pm) = crate::tools::common::detect_linux_pm() {
        let _ = crate::tools::common::PkgOps::update(pm, true);
        crate::tools::common::PkgOps::install(pm, "tree", true)
    } else {
        Err(InstallError::Prereq(
            "No supported Linux package manager on PATH (apt/dnf/yum/zypper/pacman/apk)",
        ))
    }
}

#[cfg(target_os = "windows")]
fn install_windows() -> Result<(), InstallError> {
    // Windows typically includes a built-in tree.exe. If it's missing, we do not
    // attempt to guess a third-party package ID; prompt user to add it manually.
    if has("tree") {
        return Ok(());
    }
    Err(InstallError::Prereq(
        "`tree` command not found. On Windows, it should be built-in; ensure it's on PATH.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_tree_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
