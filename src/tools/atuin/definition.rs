//! atuin - magical shell history
//!
//! Atuin replaces your existing shell history with a SQLite database,
//! and records additional context for your commands.
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(ATUIN, {
    command: "atuin",
    macos: { brew: "atuin" },
    linux: { uniform: "atuin" },
    windows: { winget: "atuinsh.atuin" },
    bsd: { pkg: "atuin" },
    default_hook_shell_init: ("atuin", "init"),
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_atuin_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
