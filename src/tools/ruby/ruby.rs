//! ruby - Ruby programming language
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(RUBY, {
    command: "ruby",
    macos: { brew: "ruby" },
    linux: { uniform: "ruby" },
    windows: { winget: "RubyInstallerTeam.Ruby" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_ruby_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
