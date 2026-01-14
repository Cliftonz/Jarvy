//! python - Python programming language
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(PYTHON, {
    command: "python3",
    macos: { brew: "python" },
    linux: { apt: "python3", dnf: "python3", pacman: "python", apk: "python3" },
    windows: { winget: "Python.Python.3" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_python_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
