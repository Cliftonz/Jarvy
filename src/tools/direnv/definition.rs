//! direnv - directory-specific environment variables via .envrc
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(DIRENV, {
    command: "direnv",
    macos: { brew: "direnv" },
    linux: { uniform: "direnv" },
    windows: { winget: "direnv.direnv" },
    bsd: { pkg: "direnv" },
    default_hook_shell_init: ("direnv", "hook"),
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_direnv_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
