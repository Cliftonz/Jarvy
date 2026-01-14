//! kubectl - official Kubernetes command-line tool
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(KUBECTL, {
    command: "kubectl",
    macos: { brew: "kubectl" },
    linux: { uniform: "kubectl" },
    windows: { winget: "Kubernetes.kubectl" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_kubectl_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
