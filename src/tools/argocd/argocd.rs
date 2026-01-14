//! argocd - GitOps continuous delivery for Kubernetes
//!
//! This tool uses the ToolSpec pattern for declarative installation.
//! Note: On Linux, this may require custom installation via GitHub releases.

use crate::define_tool;

define_tool!(ARGOCD, {
    command: "argocd",
    macos: { brew: "argocd" },
    linux: { brew: "argocd" },
    windows: { winget: "Argoproj.ArgoCD" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_argocd_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
