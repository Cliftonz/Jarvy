//! bat - cat clone with syntax highlighting
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(BAT, {
    command: "bat",
    macos: { brew: "bat" },
    linux: { uniform: "bat" },
    windows: { winget: "sharkdp.bat" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_bat_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
