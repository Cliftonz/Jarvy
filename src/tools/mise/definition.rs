//! mise - dev tools, env vars, task runner
//!
//! mise (formerly rtx) is a polyglot tool version manager.
//! It manages languages like Node, Python, Ruby, etc.
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(MISE, {
    command: "mise",
    macos: { brew: "mise" },
    linux: { brew: "mise" },
    windows: { winget: "jdx.mise" },
    bsd: { pkg: "mise" },
    default_hook_shell_init: ("mise", "activate"),
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_mise_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
