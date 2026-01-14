//! zoxide - smarter cd command that learns your navigation patterns
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(ZOXIDE, {
    command: "zoxide",
    macos: { brew: "zoxide" },
    linux: { uniform: "zoxide" },
    windows: { winget: "ajeetdsouza.zoxide" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_zoxide_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
