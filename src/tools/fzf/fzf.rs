//! fzf - command-line fuzzy finder
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(FZF, {
    command: "fzf",
    macos: { brew: "fzf" },
    linux: { uniform: "fzf" },
    windows: { winget: "junegunn.fzf" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_fzf_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
