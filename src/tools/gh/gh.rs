//! gh - GitHub's official CLI for PRs, issues, and repos
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(GH, {
    command: "gh",
    macos: { brew: "gh" },
    linux: { uniform: "gh" },
    windows: { winget: "GitHub.cli" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_gh_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
