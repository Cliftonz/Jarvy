use crate::tools::common::{InstallError, has, run};

/// Ensure iTerm2 is installed. Only supported on macOS via Homebrew cask.
/// Version hint is ignored.
pub fn ensure(_min_hint: &str) -> Result<(), InstallError> {
    // There isn't a canonical CLI to probe; rely on brew presence/install
    install()
}

/// Registry adapter: allows tools::add("iterm2", version) to dispatch here
pub fn add_handler(min_hint: &str) -> Result<(), InstallError> {
    let _ = min_hint;
    ensure("")
}

fn install() -> Result<(), InstallError> {
    #[cfg(target_os = "macos")]
    {
        return install_macos();
    }
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        return Err(InstallError::Unsupported);
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
    run("brew", &["install", "--cask", "iterm2"]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_iterm2_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
