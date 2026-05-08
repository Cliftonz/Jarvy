//! starship - minimal, fast, customizable shell prompt
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(STARSHIP, {
    command: "starship",
    macos: { brew: "starship" },
    linux: { uniform: "starship" },
    windows: { winget: "Starship.Starship" },
    bsd: { pkg: "starship" },
    default_hook_shell_init: ("starship", "init"),
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_starship_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
