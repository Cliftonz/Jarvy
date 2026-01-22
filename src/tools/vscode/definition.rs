//! vscode - Visual Studio Code editor
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(VSCODE, {
    command: "code",
    macos: { cask: "visual-studio-code" },
    linux: { uniform: "code" },
    windows: { winget: "Microsoft.VisualStudioCode" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_vscode_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
