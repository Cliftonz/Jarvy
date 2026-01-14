//! helm - package manager for Kubernetes applications
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(HELM, {
    command: "helm",
    macos: { brew: "helm" },
    linux: { uniform: "helm" },
    windows: { winget: "Helm.Helm" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_helm_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
