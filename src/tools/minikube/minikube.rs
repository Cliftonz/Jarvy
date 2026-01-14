//! minikube - local Kubernetes cluster for development
//!
//! This tool uses the ToolSpec pattern for declarative installation.

use crate::define_tool;

define_tool!(MINIKUBE, {
    command: "minikube",
    macos: { brew: "minikube" },
    linux: { uniform: "minikube" },
    windows: { winget: "Kubernetes.minikube" },
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_minikube_no_panic() {
        let res = ensure("");
        assert!(res.is_ok() || res.is_err());
    }
}
