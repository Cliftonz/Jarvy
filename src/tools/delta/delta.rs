//! delta - syntax-highlighting pager for git diff output
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(DELTA, {
    command: "delta",
    macos: { brew: "git-delta" },
    linux: { apt: "git-delta", dnf: "git-delta", pacman: "git-delta", apk: "git-delta" },
    windows: { winget: "dandavison.delta" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_delta_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
