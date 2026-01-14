//! eza - modern ls replacement with colors and Git awareness
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(EZA, {
    command: "eza",
    macos: { brew: "eza" },
    linux: { apt: "eza", dnf: "eza", pacman: "eza", apk: "eza" },
    windows: { winget: "eza-community.eza" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_eza_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
