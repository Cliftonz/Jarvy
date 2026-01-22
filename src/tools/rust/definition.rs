use crate::tools::common::{InstallError, cmd_satisfies, has, run};

/// Ensure Rust toolchain is installed. We accept presence of either `rustc` or `rustup`.
/// If a version hint is provided, we do a best-effort check against `rustc --version`.
pub fn ensure(min_hint: &str) -> Result<(), InstallError> {
    // If rustc satisfies the version hint (or hint empty), we're good
    if !min_hint.is_empty() {
        if cmd_satisfies("rustc", min_hint) {
            return Ok(());
        }
    } else if has("rustc") || has("rustup") {
        return Ok(());
    }
    install()
}

/// Registry adapter: allows tools::add("rust", version) to dispatch here
pub fn add_handler(min_hint: &str) -> Result<(), InstallError> {
    ensure(min_hint)
}

fn install() -> Result<(), InstallError> {
    #[cfg(target_os = "macos")]
    {
        return install_unix();
    }
    #[cfg(target_os = "linux")]
    {
        return install_unix();
    }
    #[cfg(target_os = "windows")]
    {
        return install_windows();
    }
    #[allow(unreachable_code)]
    Err(InstallError::Unsupported)
}

// macOS/Linux: install via rustup non-interactively (-y)
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn install_unix() -> Result<(), InstallError> {
    // Use bash -lc to ensure shell expands the pipe correctly
    run(
        "bash",
        &[
            "-lc",
            "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
        ],
    )
    .map(|_| ())
}

#[cfg(target_os = "windows")]
fn install_windows() -> Result<(), InstallError> {
    if !has("winget") {
        return Err(InstallError::Prereq(
            "winget not found. Install Windows Package Manager, then re-run.",
        ));
    }
    // Official rustup package ID
    run("winget", &["install", "-e", "--id", "Rustlang.Rustup"]).map(|_| ())
}
