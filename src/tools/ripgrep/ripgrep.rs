//! ripgrep - fast regex search tool
//!
//! This tool uses the ToolSpec pattern for declarative installation.
//! Note: The command is "rg" but the package name is "ripgrep".

use crate::define_tool;

define_tool!(RIPGREP, {
    command: "rg",
    macos: { brew: "ripgrep" },
    linux: { uniform: "ripgrep" },
    windows: { winget: "BurntSushi.ripgrep.MSVC" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_ripgrep_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
