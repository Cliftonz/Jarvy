//! go - Go programming language
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(GO, {
    command: "go",
    macos: { brew: "go" },
    linux: { apt: "golang", dnf: "golang", pacman: "go", apk: "go" },
    windows: { winget: "GoLang.Go" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_go_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
