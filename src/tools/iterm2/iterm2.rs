//! iterm2 - iTerm2 terminal emulator
//!
//! This tool uses the ToolSpec pattern for declarative installation.
//! Note: macOS only via Homebrew cask.

use crate::define_tool;

define_tool!(ITERM2, {
    command: "iterm2",
    macos: { cask: "iterm2" },
    // No Linux or Windows support - macOS only
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_iterm2_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
