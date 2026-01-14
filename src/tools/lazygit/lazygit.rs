//! lazygit - simple terminal UI for git commands
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(LAZYGIT, {
    command: "lazygit",
    macos: { brew: "lazygit" },
    linux: { uniform: "lazygit" },
    windows: { winget: "JesseDuffield.lazygit" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_lazygit_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
