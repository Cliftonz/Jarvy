//! Canonical "how was this tool installed?" classifier.
//!
//! Replaces 4 hand-rolled `detect_install_method(tool: &str) -> String`
//! impls (in `commands/setup_cmd.rs`, `commands/diagnose.rs`,
//! `commands/drift_cmd.rs`, `observability/bundle.rs`) that drifted
//! to slightly different string sets — most notably `"brew"` vs
//! `"homebrew"`, which broke string equality between the drift
//! checker (writes `"brew"` to `state.json`) and ticket bundles
//! (writes `"homebrew"`). Round-2 maint F1 / consolidation item 10.
//!
//! Use `detect_install_method_for_tool(name)` from any subsystem
//! that needs to label where a binary on `$PATH` came from.

#![allow(dead_code)] // Multiple consumers migrate incrementally

/// Canonical install-method label. The `Display` impl produces the
/// string consumers should persist or compare (e.g. drift `state.json`,
/// ticket bundle JSON, support log).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InstallMethod {
    Brew,
    Cargo,
    Nvm,
    Pyenv,
    Rustup,
    Snap,
    System,
    /// `which` couldn't find the binary on `$PATH`.
    NotFound,
    /// Found, but the path matches none of the known patterns.
    Unknown,
}

impl InstallMethod {
    /// Canonical string form. Once persisted in `state.json` or sent
    /// in a ticket bundle, this string is the wire format — don't
    /// change without bumping the state schema.
    pub fn as_str(&self) -> &'static str {
        match self {
            InstallMethod::Brew => "brew",
            InstallMethod::Cargo => "cargo",
            InstallMethod::Nvm => "nvm",
            InstallMethod::Pyenv => "pyenv",
            InstallMethod::Rustup => "rustup",
            InstallMethod::Snap => "snap",
            InstallMethod::System => "system",
            InstallMethod::NotFound => "not_found",
            InstallMethod::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for InstallMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Resolve `name` via `which`, then classify the path.
pub fn detect_install_method_for_tool(name: &str) -> InstallMethod {
    match which::which(name) {
        Ok(path) => detect_install_method_from_path(&path),
        Err(_) => InstallMethod::NotFound,
    }
}

/// Classify an absolute binary path. Exposed for testability — the
/// `which` lookup is the only non-pure step.
pub fn detect_install_method_from_path(path: &std::path::Path) -> InstallMethod {
    let s = path.to_string_lossy();
    if s.contains("/homebrew/") || s.contains("/opt/homebrew/") || s.contains("/Cellar/") {
        InstallMethod::Brew
    } else if s.contains("/.cargo/") {
        InstallMethod::Cargo
    } else if s.contains("/.nvm/") {
        InstallMethod::Nvm
    } else if s.contains("/.pyenv/") {
        InstallMethod::Pyenv
    } else if s.contains("/.rustup/") {
        InstallMethod::Rustup
    } else if s.contains("/snap/") {
        InstallMethod::Snap
    } else if s.contains("/usr/bin/") || s.contains("/usr/local/bin/") {
        InstallMethod::System
    } else {
        InstallMethod::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_homebrew_apple_silicon_and_intel() {
        assert_eq!(
            detect_install_method_from_path(&PathBuf::from("/opt/homebrew/bin/jq")),
            InstallMethod::Brew
        );
        assert_eq!(
            detect_install_method_from_path(&PathBuf::from("/usr/local/Cellar/jq/1.7.1/bin/jq")),
            InstallMethod::Brew
        );
    }

    #[test]
    fn classifies_cargo_pyenv_nvm_rustup_snap() {
        assert_eq!(
            detect_install_method_from_path(&PathBuf::from("/Users/x/.cargo/bin/cargo-watch")),
            InstallMethod::Cargo
        );
        assert_eq!(
            detect_install_method_from_path(&PathBuf::from(
                "/home/u/.nvm/versions/node/v20/bin/node"
            )),
            InstallMethod::Nvm
        );
        assert_eq!(
            detect_install_method_from_path(&PathBuf::from("/home/u/.pyenv/shims/python")),
            InstallMethod::Pyenv
        );
        assert_eq!(
            detect_install_method_from_path(&PathBuf::from(
                "/home/u/.rustup/toolchains/x/bin/rustc"
            )),
            InstallMethod::Rustup
        );
        assert_eq!(
            detect_install_method_from_path(&PathBuf::from("/snap/bin/code")),
            InstallMethod::Snap
        );
    }

    #[test]
    fn classifies_system_paths() {
        assert_eq!(
            detect_install_method_from_path(&PathBuf::from("/usr/bin/git")),
            InstallMethod::System
        );
        assert_eq!(
            detect_install_method_from_path(&PathBuf::from("/usr/local/bin/jq")),
            InstallMethod::System
        );
    }

    #[test]
    fn unknown_paths_classify_as_unknown() {
        assert_eq!(
            detect_install_method_from_path(&PathBuf::from("/tmp/foo/bar")),
            InstallMethod::Unknown
        );
    }

    #[test]
    fn display_strings_are_stable() {
        // These strings end up in state.json and ticket bundles —
        // changing them is a wire-format break.
        assert_eq!(InstallMethod::Brew.to_string(), "brew");
        assert_eq!(InstallMethod::Cargo.to_string(), "cargo");
        assert_eq!(InstallMethod::Nvm.to_string(), "nvm");
        assert_eq!(InstallMethod::Pyenv.to_string(), "pyenv");
        assert_eq!(InstallMethod::Rustup.to_string(), "rustup");
        assert_eq!(InstallMethod::Snap.to_string(), "snap");
        assert_eq!(InstallMethod::System.to_string(), "system");
        assert_eq!(InstallMethod::NotFound.to_string(), "not_found");
        assert_eq!(InstallMethod::Unknown.to_string(), "unknown");
    }
}
