//! k9s - terminal UI for Kubernetes cluster management
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(K9S, {
    command: "k9s",
    macos: { brew: "derailed/k9s/k9s" },
    linux: { uniform: "k9s" },
    windows: { winget: "Derailed.k9s" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_k9s_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
