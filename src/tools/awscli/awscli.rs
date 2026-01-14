//! awscli - AWS Command Line Interface
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(AWSCLI, {
    command: "aws",
    macos: { brew: "awscli" },
    linux: { uniform: "awscli" },
    windows: { winget: "Amazon.AWSCLI" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_awscli_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
